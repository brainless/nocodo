use actix_web::{get, post, web, HttpResponse, Responder};
use nocodo_agents::{
    AgentStorage, FormLayout, SchemaStorage, SqliteAgentStorage, SqliteSchemaStorage,
    SqliteTaskStorage, SqliteUiFormStorage, Task, TaskStatus, TaskStorage, UiFormStorage,
};
use shared_types::SchemaDef;

use crate::agents_api::state::AgentState;

use super::types::{
    FormLayoutJson, FormLayoutResponse, GenerateFormQueued, GenerateFormRequest, ListFormsResponse,
};

const AGENT_TYPE: &str = "ui_designer";

/// POST /api/agents/ui-designer/form
///
/// If a form layout already exists for (project_id, entity_name), returns it immediately.
/// Otherwise creates a task and dispatches the ui_designer agent, returning task_id.
#[post("/api/agents/ui-designer/form")]
pub async fn generate_form(
    state: web::Data<AgentState>,
    request: web::Json<GenerateFormRequest>,
) -> impl Responder {
    let GenerateFormRequest { project_id, entity_name } = request.into_inner();

    let form_storage = match SqliteUiFormStorage::open(&state.db_path) {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("{}", e) }))
        }
    };

    // Return cached layout if available.
    match form_storage.get_form_layout(project_id, &entity_name).await {
        Ok(Some(json)) => {
            if let Ok(layout) = serde_json::from_str::<FormLayout>(&json) {
                return HttpResponse::Ok().json(FormLayoutResponse {
                    entity_name: entity_name.clone(),
                    layout: FormLayoutJson::from(layout),
                });
            }
        }
        Ok(None) => {}
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("{}", e) }))
        }
    }

    // Look up the latest schema to build the source_prompt for the agent.
    let schema_storage = match SqliteSchemaStorage::open(&state.db_path) {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("{}", e) }))
        }
    };

    let table_def_json = match schema_storage.get_latest_schema_for_project(project_id).await {
        Ok(Some(schema_json)) => {
            match serde_json::from_str::<SchemaDef>(&schema_json) {
                Ok(schema) => {
                    match schema.tables.into_iter().find(|t| t.name == entity_name) {
                        Some(table) => serde_json::to_string(&table).unwrap_or_default(),
                        None => {
                            return HttpResponse::BadRequest().json(serde_json::json!({
                                "error": format!("Entity '{}' not found in latest schema", entity_name)
                            }))
                        }
                    }
                }
                Err(_) => {
                    return HttpResponse::InternalServerError()
                        .json(serde_json::json!({ "error": "Failed to parse schema" }))
                }
            }
        }
        Ok(None) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "No schema found for this project. Run the schema designer first."
            }))
        }
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("{}", e) }))
        }
    };

    // Create task + session, then fire dispatch.
    let task_storage = match SqliteTaskStorage::open(&state.db_path) {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("{}", e) }))
        }
    };

    let agent_storage = match SqliteAgentStorage::open(&state.db_path) {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("{}", e) }))
        }
    };

    let title = format!("Design form for {}", entity_name);
    let task_id = match task_storage
        .create_task(Task {
            id: None,
            project_id,
            epic_id: None,
            title,
            description: format!("Generate a form layout for the '{}' entity.", entity_name),
            source_prompt: table_def_json.clone(),
            assigned_to_agent: AGENT_TYPE.to_string(),
            status: TaskStatus::Open,
            depends_on_task_id: None,
            created_by_agent: "user".to_string(),
            created_at: 0,
            updated_at: 0,
        })
        .await
    {
        Ok(id) => id,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("{}", e) }))
        }
    };

    let session = match agent_storage
        .create_task_session(project_id, task_id, AGENT_TYPE)
        .await
    {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("{}", e) }))
        }
    };

    // Store the table definition as the first user message so the agent can read it.
    if let Err(e) = agent_storage
        .create_message(nocodo_agents::ChatMessage {
            id: None,
            session_id: session.id.unwrap_or(0),
            role: "user".to_string(),
            agent_type: None,
            content: table_def_json.clone(),
            tool_call_id: None,
            tool_name: None,
            turn_id: None,
            created_at: 0,
        })
        .await
    {
        return HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": format!("{}", e) }));
    }

    // Fire the dispatch event so the background dispatcher picks it up.
    // source_prompt is not used by dispatch_ui_designer (session already created),
    // but included for the startup reconciliation path.
    state
        .dispatch_tx
        .send(crate::agents_api::dispatcher::DispatchEvent {
            task_id,
            project_id,
            assigned_to_agent: AGENT_TYPE.to_string(),
            source_prompt: table_def_json,
        })
        .ok();
    state.board_notify.notify_waiters();

    HttpResponse::Accepted().json(GenerateFormQueued { task_id })
}

/// GET /api/agents/ui-designer/form/{project_id}/{entity_name}
///
/// Returns the stored form layout or 404 if not yet generated.
#[get("/api/agents/ui-designer/form/{project_id}/{entity_name}")]
pub async fn get_form(
    state: web::Data<AgentState>,
    path: web::Path<(i64, String)>,
) -> impl Responder {
    let (project_id, entity_name) = path.into_inner();

    let form_storage = match SqliteUiFormStorage::open(&state.db_path) {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("{}", e) }))
        }
    };

    match form_storage.get_form_layout(project_id, &entity_name).await {
        Ok(Some(json)) => match serde_json::from_str::<FormLayout>(&json) {
            Ok(layout) => HttpResponse::Ok().json(FormLayoutResponse {
                entity_name,
                layout: FormLayoutJson::from(layout),
            }),
            Err(_) => HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": "Corrupt form layout in database" })),
        },
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({ "status": "pending" })),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": format!("{}", e) })),
    }
}

/// GET /api/agents/ui-designer/entities/{project_id}
///
/// Returns entity names from the latest agent-generated schema for this project.
#[get("/api/agents/ui-designer/entities/{project_id}")]
pub async fn list_entities(
    state: web::Data<AgentState>,
    path: web::Path<i64>,
) -> impl Responder {
    let project_id = path.into_inner();

    let schema_storage = match SqliteSchemaStorage::open(&state.db_path) {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("{}", e) }))
        }
    };

    match schema_storage.get_latest_schema_for_project(project_id).await {
        Ok(Some(schema_json)) => match serde_json::from_str::<SchemaDef>(&schema_json) {
            Ok(schema) => {
                let entities: Vec<String> = schema.tables.into_iter().map(|t| t.name).collect();
                HttpResponse::Ok().json(serde_json::json!({ "entities": entities }))
            }
            Err(_) => HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": "Failed to parse schema" })),
        },
        Ok(None) => HttpResponse::Ok().json(serde_json::json!({ "entities": [] })),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": format!("{}", e) })),
    }
}

/// GET /api/agents/ui-designer/forms/{project_id}
///
/// Lists all cached form layouts for a project.
#[get("/api/agents/ui-designer/forms/{project_id}")]
pub async fn list_forms(
    state: web::Data<AgentState>,
    path: web::Path<i64>,
) -> impl Responder {
    let project_id = path.into_inner();

    let form_storage = match SqliteUiFormStorage::open(&state.db_path) {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("{}", e) }))
        }
    };

    match form_storage.list_form_layouts(project_id).await {
        Ok(pairs) => {
            let forms = pairs
                .into_iter()
                .filter_map(|(entity_name, json)| {
                    serde_json::from_str::<FormLayout>(&json).ok().map(|layout| {
                        FormLayoutResponse {
                            entity_name,
                            layout: FormLayoutJson::from(layout),
                        }
                    })
                })
                .collect();
            HttpResponse::Ok().json(ListFormsResponse { forms })
        }
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": format!("{}", e) })),
    }
}
