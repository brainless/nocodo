use std::path::Path;

use actix_web::{post, web, HttpResponse, Responder};
use nocodo_agents::build_rust_engineer;
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};

use crate::agents_api::state::AgentState;

#[derive(Deserialize)]
pub struct RunRequest {
    pub project_id: i64,
    pub mode: Option<String>,
    pub struct_name: Option<String>,
    pub fn_name: Option<String>,
    pub prompt: Option<String>,
    /// When `true`, write generated code to disk. Defaults to `false`.
    #[serde(default)]
    pub apply: bool,
}

#[derive(Serialize)]
pub struct RunResponse {
    pub system_prompt: Option<String>,
    pub prompt: String,
    pub raw_response: String,
    pub code: Option<String>,
    /// Relative file path of the written file when `apply` is `true`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
}

/// POST /api/rust-engineer/run
///
/// Runs the Rust engineer agent. Defaults to diesel_model_fn mode for backward
/// compatibility. Returns prompt(s), raw response, and extracted code.
#[post("/api/rust-engineer/run")]
pub async fn run(state: web::Data<AgentState>, body: web::Json<RunRequest>) -> impl Responder {
    let project_path = match get_project_path(&state.db_path, body.project_id) {
        Ok(Some(p)) => p,
        Ok(None) => {
            return HttpResponse::NotFound()
                .json(serde_json::json!({ "error": "Project not found" }));
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({ "error": e }));
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

    let mode = body.mode.as_deref().unwrap_or("diesel_model_fn");
    match mode {
        "diesel_model_struct" => {
            let Some(prompt) = body
                .prompt
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            else {
                return HttpResponse::BadRequest()
                    .json(serde_json::json!({ "error": "prompt is required for diesel_model_struct mode" }));
            };

            if body.apply {
                match agent.diesel_model_struct_write(prompt).await {
                    Ok(output) => HttpResponse::Ok().json(RunResponse {
                        system_prompt: Some(output.system_prompt),
                        prompt: output.prompt,
                        raw_response: output.raw_response,
                        code: output.code,
                        file_path: output.file_path,
                    }),
                    Err(e) => HttpResponse::InternalServerError()
                        .json(serde_json::json!({ "error": format!("{}", e) })),
                }
            } else {
                match agent.diesel_model_struct(prompt).await {
                    Ok(output) => HttpResponse::Ok().json(RunResponse {
                        system_prompt: Some(output.system_prompt),
                        prompt: output.prompt,
                        raw_response: output.raw_response,
                        code: output.code,
                        file_path: None,
                    }),
                    Err(e) => HttpResponse::InternalServerError()
                        .json(serde_json::json!({ "error": format!("{}", e) })),
                }
            }
        }
        "diesel_schema" => {
            let Some(prompt) = body
                .prompt
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            else {
                return HttpResponse::BadRequest().json(
                    serde_json::json!({ "error": "prompt is required for diesel_schema mode" }),
                );
            };

            if body.apply {
                match agent.diesel_schema_write(prompt).await {
                    Ok(output) => HttpResponse::Ok().json(RunResponse {
                        system_prompt: Some(output.system_prompt),
                        prompt: output.prompt,
                        raw_response: output.raw_response,
                        code: output.code,
                        file_path: output.file_path,
                    }),
                    Err(e) => HttpResponse::InternalServerError()
                        .json(serde_json::json!({ "error": format!("{}", e) })),
                }
            } else {
                match agent.diesel_schema(prompt).await {
                    Ok(output) => HttpResponse::Ok().json(RunResponse {
                        system_prompt: Some(output.system_prompt),
                        prompt: output.prompt,
                        raw_response: output.raw_response,
                        code: output.code,
                        file_path: None,
                    }),
                    Err(e) => HttpResponse::InternalServerError()
                        .json(serde_json::json!({ "error": format!("{}", e) })),
                }
            }
        }
        "diesel_model_fn" | "diesel_model" => {
            let Some(struct_name) = body
                .struct_name
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            else {
                return HttpResponse::BadRequest()
                    .json(serde_json::json!({ "error": "struct_name is required for diesel_model_fn mode" }));
            };
            let Some(fn_name) = body
                .fn_name
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            else {
                return HttpResponse::BadRequest().json(
                    serde_json::json!({ "error": "fn_name is required for diesel_model_fn mode" }),
                );
            };

            match agent.diesel_model_fn(struct_name, fn_name).await {
                Ok(output) => HttpResponse::Ok().json(RunResponse {
                    system_prompt: None,
                    prompt: output.prompt,
                    raw_response: output.raw_response,
                    code: output.code,
                    file_path: None,
                }),
                Err(e) => HttpResponse::InternalServerError()
                    .json(serde_json::json!({ "error": format!("{}", e) })),
            }
        }
        _ => HttpResponse::BadRequest()
            .json(serde_json::json!({ "error": format!("unknown rust engineer mode: {}", mode) })),
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
