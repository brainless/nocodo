use std::path::Path;

use actix_web::{get, post, web, HttpResponse, Responder};
use nocodo_agents::{
    AgentStorage, ContextStorage, SqliteAgentStorage, SqliteContextStorage, SqliteTaskStorage,
    Task, TaskStatus, TaskStorage,
};
use rusqlite::OptionalExtension;

use crate::agents_api::state::AgentState;

use super::types::{ContextResponse, GatherContextQueued, GatherContextRequest};

const BACKEND_ENGINEER: &str = "backend_engineer";
const FRONTEND_ENGINEER: &str = "frontend_engineer";

fn validate_context_type(context_type: &str) -> Option<&'static str> {
    match context_type {
        "backend" | "backend_engineer" => Some(BACKEND_ENGINEER),
        "frontend" | "admin_gui" | "admin-gui" | "frontend_engineer" => Some(FRONTEND_ENGINEER),
        _ => None,
    }
}

/// POST /api/agents/context/gather
///
/// If context already exists for (project_id, context_type), returns it immediately.
/// Otherwise creates a task and dispatches the context agent, returning task_id.
#[post("/api/agents/context/gather")]
pub async fn gather_context(
    state: web::Data<AgentState>,
    request: web::Json<GatherContextRequest>,
) -> impl Responder {
    let GatherContextRequest { project_id, context_type } = request.into_inner();

    let ct = match validate_context_type(&context_type) {
        Some(ct) => ct,
        None => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": format!("Invalid context_type '{}'. Use 'backend' or 'frontend'.", context_type)
            }));
        }
    };

    let context_storage = match SqliteContextStorage::open(&state.db_path) {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("{}", e) }))
        }
    };

    // Return cached context if available.
    match context_storage.get_context(project_id, ct).await {
        Ok(Some(context)) => {
            return HttpResponse::Ok().json(ContextResponse {
                context_type: ct.to_string(),
                context,
            });
        }
        Ok(None) => {}
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("{}", e) }))
        }
    }

    // Look up the project path from the DB.
    let project_path = match get_project_path(&state.db_path, project_id) {
        Ok(Some(path)) => path,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(serde_json::json!({ "error": "Project not found" }));
        }
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("{}", e) }))
        }
    };
    let project_path_ref = Path::new(&project_path);
    if !project_path_ref.is_dir() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": format!("Project path is not a readable directory: {}", project_path)
        }));
    }
    if let Err(e) = std::fs::read_dir(project_path_ref) {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": format!("Project path is not readable: {} ({})", project_path, e)
        }));
    }

    let agent_type_str = ct.to_string();
    let title = if ct == BACKEND_ENGINEER {
        format!("Gather backend context for project {}", project_id)
    } else {
        format!("Gather frontend context for project {}", project_id)
    };

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

    let task_id = match task_storage
        .create_task(Task {
            id: None,
            project_id,
            epic_id: None,
            title,
            description: format!("Gather {} context.", ct),
            source_prompt: project_path.clone(),
            assigned_to_agent: agent_type_str.clone(),
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
        .create_task_session(project_id, task_id, &agent_type_str)
        .await
    {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("{}", e) }))
        }
    };

    // Store the project path as the first user message so the agent knows where to look.
    if let Err(e) = agent_storage
        .create_message(nocodo_agents::ChatMessage {
            id: None,
            session_id: session.id.unwrap_or(0),
            role: "user".to_string(),
            agent_type: None,
            content: format!(
                "Analyze the {} of the project at: {}",
                if ct == BACKEND_ENGINEER { "backend" } else { "admin-gui" },
                project_path
            ),
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
    state
        .dispatch_tx
        .send(crate::agents_api::dispatcher::DispatchEvent {
            task_id,
            project_id,
            assigned_to_agent: agent_type_str,
            source_prompt: project_path,
        })
        .ok();
    state.board_notify.notify_waiters();

    HttpResponse::Accepted().json(GatherContextQueued { task_id })
}

/// GET /api/agents/context/{project_id}/{context_type}
///
/// Returns the stored context or 404 if not yet gathered.
#[get("/api/agents/context/{project_id}/{context_type}")]
pub async fn get_context(
    state: web::Data<AgentState>,
    path: web::Path<(i64, String)>,
) -> impl Responder {
    let (project_id, context_type) = path.into_inner();

    let ct = match validate_context_type(&context_type) {
        Some(ct) => ct,
        None => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": format!("Invalid context_type '{}'. Use 'backend' or 'frontend'.", context_type)
            }));
        }
    };

    let context_storage = match SqliteContextStorage::open(&state.db_path) {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("{}", e) }))
        }
    };

    match context_storage.get_context(project_id, ct).await {
        Ok(Some(context)) => HttpResponse::Ok().json(ContextResponse {
            context_type: ct.to_string(),
            context,
        }),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({ "status": "pending" })),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": format!("{}", e) })),
    }
}

fn get_project_path(db_path: &str, project_id: i64) -> Result<Option<String>, String> {
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    let result = conn
        .query_row(
            "SELECT path FROM project WHERE id = ?1",
            [project_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    Ok(result)
}
