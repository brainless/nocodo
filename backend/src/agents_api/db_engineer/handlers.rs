use crate::agents_api::db_engineer::types::{
    BoardQuery, BoardResponse, EpicItem, EpicListQuery, ListEpicsResponse, ListTasksQuery,
    ListTasksResponse, SchemaCodegenResponse, SchemaPreviewQuery, SchemaPreviewResponse, TaskItem,
};
use crate::agents_api::state::AgentState;
use actix_web::{get, web, HttpResponse, Responder};
use nocodo_agents::{
    AgentStorage, SchemaStorage, SqliteAgentStorage, SqliteSchemaStorage, SqliteTaskStorage,
    TaskStorage,
};
use rusqlite::params as sql_params;
use shared_types::SchemaDef;
use std::time::Duration;

const AGENT_TYPE: &str = "db_engineer";

/// GET /api/agents/tasks?project_id=X
#[get("/api/agents/tasks")]
pub async fn list_tasks(
    state: web::Data<AgentState>,
    query: web::Query<ListTasksQuery>,
) -> impl Responder {
    let task_storage = match SqliteTaskStorage::open(&state.db_path) {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to open task storage: {}", e)
            }));
        }
    };

    match task_storage.list_tasks_for_project(query.project_id).await {
        Ok(tasks) => HttpResponse::Ok().json(ListTasksResponse {
            tasks: tasks
                .into_iter()
                .map(|t| TaskItem {
                    id: t.id.unwrap_or(0),
                    project_id: t.project_id,
                    epic_id: t.epic_id,
                    title: t.title,
                    source_prompt: t.source_prompt,
                    assigned_to_agent: t.assigned_to_agent,
                    status: t.status.as_str().to_string(),
                    created_at: t.created_at,
                    updated_at: t.updated_at,
                })
                .collect(),
        }),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to list tasks: {}", e)
        })),
    }
}

/// GET /api/agents/epics?project_id=X
#[get("/api/agents/epics")]
pub async fn list_epics(
    state: web::Data<AgentState>,
    query: web::Query<EpicListQuery>,
) -> impl Responder {
    let task_storage = match SqliteTaskStorage::open(&state.db_path) {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to open task storage: {}", e)
            }));
        }
    };

    match task_storage.list_epics(query.project_id).await {
        Ok(epics) => HttpResponse::Ok().json(ListEpicsResponse {
            epics: epics
                .into_iter()
                .map(|e| EpicItem {
                    id: e.id.unwrap_or(0),
                    project_id: e.project_id,
                    title: e.title,
                    description: e.description,
                    status: e.status.as_str().to_string(),
                    created_by_agent: e.created_by_agent,
                    created_by_task_id: e.created_by_task_id,
                    created_at: e.created_at,
                    updated_at: e.updated_at,
                })
                .collect(),
        }),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to list epics: {}", e)
        })),
    }
}

/// GET /api/agents/board?project_id=X[&since=Y]
/// Long-poll: holds the connection until tasks/epics are newer than `since`, or 30s elapses.
#[get("/api/agents/board")]
pub async fn get_board(
    state: web::Data<AgentState>,
    query: web::Query<BoardQuery>,
) -> impl Responder {
    let project_id = query.project_id;
    let since = query.since.unwrap_or(0);

    let notified = state.board_notify.notified();
    tokio::pin!(notified);
    notified.as_mut().enable();

    let task_storage = match SqliteTaskStorage::open(&state.db_path) {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("storage error: {}", e)
            }))
        }
    };

    let (tasks, epics) = match fetch_board_data(&task_storage, project_id).await {
        Ok(d) => d,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({ "error": e }))
        }
    };

    let updated_at = tasks
        .iter()
        .map(|t| t.updated_at)
        .chain(epics.iter().map(|e| e.updated_at))
        .max()
        .unwrap_or(0);

    let project_name = fetch_project_name(&state.db_path, project_id);

    if since == 0 || updated_at > since {
        return HttpResponse::Ok().json(BoardResponse {
            tasks,
            epics,
            updated_at,
            project_name,
        });
    }

    let _ = tokio::time::timeout(Duration::from_secs(30), notified).await;

    match fetch_board_data(&task_storage, project_id).await {
        Ok((tasks, epics)) => {
            let updated_at = tasks
                .iter()
                .map(|t| t.updated_at)
                .chain(epics.iter().map(|e| e.updated_at))
                .max()
                .unwrap_or(0);
            let project_name = fetch_project_name(&state.db_path, project_id);
            HttpResponse::Ok().json(BoardResponse {
                tasks,
                epics,
                updated_at,
                project_name,
            })
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e })),
    }
}

fn fetch_project_name(db_path: &str, project_id: i64) -> String {
    rusqlite::Connection::open(db_path)
        .ok()
        .and_then(|conn| {
            conn.query_row(
                "SELECT name FROM project WHERE id = ?1",
                sql_params![project_id],
                |row| row.get::<_, String>(0),
            )
            .ok()
        })
        .unwrap_or_default()
}

async fn fetch_board_data(
    storage: &SqliteTaskStorage,
    project_id: i64,
) -> Result<(Vec<TaskItem>, Vec<EpicItem>), String> {
    let tasks = storage
        .list_tasks_for_project(project_id)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|t| TaskItem {
            id: t.id.unwrap_or(0),
            project_id: t.project_id,
            epic_id: t.epic_id,
            title: t.title,
            source_prompt: t.source_prompt,
            assigned_to_agent: t.assigned_to_agent,
            status: t.status.as_str().to_string(),
            created_at: t.created_at,
            updated_at: t.updated_at,
        })
        .collect();

    let epics = storage
        .list_epics(project_id)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|e| EpicItem {
            id: e.id.unwrap_or(0),
            project_id: e.project_id,
            title: e.title,
            description: e.description,
            status: e.status.as_str().to_string(),
            created_by_agent: e.created_by_agent,
            created_by_task_id: e.created_by_task_id,
            created_at: e.created_at,
            updated_at: e.updated_at,
        })
        .collect();

    Ok((tasks, epics))
}

/// GET /api/agents/db-engineer/tasks/{task_id}/schema?version=N
#[get("/api/agents/db-engineer/tasks/{task_id}/schema")]
pub async fn get_task_schema(
    state: web::Data<AgentState>,
    path: web::Path<i64>,
    query: web::Query<SchemaPreviewQuery>,
) -> HttpResponse {
    let task_id = path.into_inner();

    let agent_storage = match SqliteAgentStorage::open(&state.db_path) {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("Storage error: {}", e) }));
        }
    };

    let session = match agent_storage.get_session_by_task(task_id, AGENT_TYPE).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(serde_json::json!({ "error": "No session for this task" }));
        }
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("Session lookup error: {}", e) }));
        }
    };

    let schema_storage = match SqliteSchemaStorage::open(&state.db_path) {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("Storage error: {}", e) }));
        }
    };

    match schema_storage
        .get_schema_for_session(session.id.unwrap_or(0), query.version)
        .await
    {
        Ok(Some((schema_json, version))) => match serde_json::from_str::<SchemaDef>(&schema_json) {
            Ok(schema) => HttpResponse::Ok().json(SchemaPreviewResponse { schema, version }),
            Err(e) => HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("Schema corrupt: {}", e) })),
        },
        Ok(None) => HttpResponse::NotFound()
            .json(serde_json::json!({ "error": "No schema generated for this task yet" })),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": format!("Query error: {}", e) })),
    }
}

/// GET /api/agents/db-engineer/tasks/{task_id}/codegen
#[get("/api/agents/db-engineer/tasks/{task_id}/codegen")]
pub async fn generate_task_schema_code(
    state: web::Data<AgentState>,
    path: web::Path<i64>,
) -> HttpResponse {
    let task_id = path.into_inner();

    let agent_storage = match SqliteAgentStorage::open(&state.db_path) {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("Storage error: {}", e) }));
        }
    };

    let session = match agent_storage.get_session_by_task(task_id, AGENT_TYPE).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(serde_json::json!({ "error": "No session for this task" }));
        }
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("Session lookup error: {}", e) }));
        }
    };

    let schema_storage = match SqliteSchemaStorage::open(&state.db_path) {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("Storage error: {}", e) }));
        }
    };

    let schema_def = match schema_storage
        .get_schema_for_session(session.id.unwrap_or(0), None)
        .await
    {
        Ok(Some((schema_json, _))) => match serde_json::from_str::<SchemaDef>(&schema_json) {
            Ok(s) => s,
            Err(e) => {
                return HttpResponse::InternalServerError()
                    .json(serde_json::json!({ "error": format!("Schema corrupt: {}", e) }));
            }
        },
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(serde_json::json!({ "error": "No schema generated for this task yet" }));
        }
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("Query error: {}", e) }));
        }
    };

    let result = schema_codegen::generate(&schema_def);
    HttpResponse::Ok().json(SchemaCodegenResponse {
        rust_code: result.rust_code,
        sql_ddl: result.sql_ddl,
    })
}
