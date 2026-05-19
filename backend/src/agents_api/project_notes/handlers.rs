use actix_web::{get, web, HttpResponse, Responder};
use nocodo_agents::{ProjectNoteStorage, SqliteProjectNoteStorage};
use serde::Deserialize;

use crate::agents_api::state::AgentState;

#[derive(Deserialize)]
pub struct NotesQuery {
    pub project_id: i64,
}

/// GET /api/project-notes?project_id=N
#[get("/api/project-notes")]
pub async fn list_notes(
    state: web::Data<AgentState>,
    query: web::Query<NotesQuery>,
) -> impl Responder {
    let storage = match SqliteProjectNoteStorage::open(&state.db_path) {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("Storage error: {}", e) }));
        }
    };

    match storage.list_current_notes(query.project_id).await {
        Ok(notes) => HttpResponse::Ok().json(serde_json::json!({ "notes": notes })),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": format!("Failed to load notes: {}", e) })),
    }
}
