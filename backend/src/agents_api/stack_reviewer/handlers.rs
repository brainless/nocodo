use std::path::Path;

use actix_web::{get, post, web, HttpResponse, Responder};
use nocodo_agents::{build_stack_reviewer, AgentConfig, SqliteStackNoteStorage, StackNoteStorage};
use rusqlite::OptionalExtension;
use serde::Deserialize;

use crate::agents_api::state::AgentState;

#[derive(Deserialize)]
pub struct RunReviewRequest {
    pub project_id: i64,
}

#[derive(Deserialize)]
pub struct NotesQuery {
    pub project_id: i64,
}

/// POST /api/stack-reviewer/run
#[post("/api/stack-reviewer/run")]
pub async fn run_review(
    state: web::Data<AgentState>,
    body: web::Json<RunReviewRequest>,
) -> impl Responder {
    let project_id = body.project_id;

    let project_path = match get_project_path(&state.db_path, project_id) {
        Ok(Some(path)) => path,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(serde_json::json!({ "error": "Project not found" }));
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({ "error": e }));
        }
    };

    let path_ref = Path::new(&project_path);
    if !path_ref.is_dir() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": format!("Project path is not a readable directory: {}", project_path)
        }));
    }
    if let Err(e) = std::fs::read_dir(path_ref) {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": format!("Project path is not readable: {} ({})", project_path, e)
        }));
    }

    let db_path = state.db_path.clone();
    let config: AgentConfig = state.config.clone();

    tokio::spawn(async move {
        let agent = match build_stack_reviewer(&config, &db_path, project_id, &project_path) {
            Ok(a) => a,
            Err(e) => {
                log::error!("[stack_reviewer] Failed to build agent: {}", e);
                return;
            }
        };
        match agent.run().await {
            Ok(result) => {
                log::info!(
                    "[stack_reviewer] done: {} notes emitted. {}",
                    result.emitted_ids.len(),
                    result.summary
                );
            }
            Err(e) => {
                log::error!("[stack_reviewer] Agent run failed: {}", e);
            }
        }
    });

    HttpResponse::Accepted().json(serde_json::json!({
        "status": "started",
        "project_id": project_id,
    }))
}

/// GET /api/stack-reviewer/notes?project_id=N
#[get("/api/stack-reviewer/notes")]
pub async fn list_notes(
    state: web::Data<AgentState>,
    query: web::Query<NotesQuery>,
) -> impl Responder {
    let project_id = query.project_id;

    let storage = match SqliteStackNoteStorage::open(&state.db_path) {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("{}", e) }));
        }
    };

    match storage.list_current_notes(project_id).await {
        Ok(notes) => HttpResponse::Ok().json(serde_json::json!({ "notes": notes })),
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
