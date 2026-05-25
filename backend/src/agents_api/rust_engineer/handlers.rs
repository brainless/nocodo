use std::path::Path;

use actix_web::{post, web, HttpResponse, Responder};
use nocodo_agents::build_rust_engineer;
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};

use crate::agents_api::state::AgentState;

#[derive(Deserialize)]
pub struct RunRequest {
    pub project_id: i64,
    pub struct_name: String,
    pub fn_name: String,
}

#[derive(Serialize)]
pub struct RunResponse {
    pub prompt: String,
    pub raw_response: String,
    pub code: Option<String>,
}

/// POST /api/rust-engineer/run
///
/// Runs the Rust engineer agent in diesel_model mode. Returns the full prompt
/// that was sent to the model and the raw + extracted response for display.
#[post("/api/rust-engineer/run")]
pub async fn run(
    state: web::Data<AgentState>,
    body: web::Json<RunRequest>,
) -> impl Responder {
    let project_path = match get_project_path(&state.db_path, body.project_id) {
        Ok(Some(p)) => p,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(serde_json::json!({ "error": "Project not found" }));
        }
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": e }));
        }
    };

    if !Path::new(&project_path).is_dir() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": format!("Project path is not a readable directory: {}", project_path)
        }));
    }

    let agent = match build_rust_engineer(&project_path) {
        Ok(a) => a,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("Failed to build agent: {}", e) }));
        }
    };

    match agent.diesel_model_fn(&body.struct_name, &body.fn_name).await {
        Ok(output) => HttpResponse::Ok().json(RunResponse {
            prompt: output.prompt,
            raw_response: output.raw_response,
            code: output.code,
        }),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": format!("{}", e) })),
    }
}

fn get_project_path(db_path: &str, project_id: i64) -> Result<Option<String>, String> {
    let conn = rusqlite::Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT path FROM project WHERE id = ?1",
        [project_id],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .map_err(|e| e.to_string())
}
