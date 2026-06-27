use std::path::Path;

use actix_web::{post, web, HttpResponse, Responder};
use nocodo_agents::{build_rust_engineer, PersonaNote, SpecGap};
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

// ---------------------------------------------------------------------------
// POST /api/rust-engineer/praxis-auth
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct PraxisAuthRequest {
    pub project_id: i64,
    pub personas: Vec<PersonaNote>,
    /// When `true`, write generated code to disk under `backend/src/praxis/specs/`.
    #[serde(default)]
    pub apply: bool,
    /// When `true` and gaps are found, create a gap_clarification PO session.
    #[serde(default)]
    pub create_gap_session: bool,
    /// User ID for the gap_clarification session. Defaults to 1.
    #[serde(default = "default_user_id")]
    pub user_id: i64,
}

fn default_user_id() -> i64 {
    1
}

#[derive(Serialize)]
pub struct PraxisAuthResponse {
    pub system_prompt: String,
    pub prompt: String,
    pub raw_response: String,
    pub code: Option<String>,
    pub files_written: Vec<String>,
    pub gaps: Vec<SpecGap>,
    /// Session ID of the created gap_clarification session (if requested and gaps found).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gap_session_id: Option<i64>,
}

/// POST /api/rust-engineer/praxis-auth
///
/// Generates `nocodo_praxis` persona static definitions from supplied `PersonaNote` data.
/// When `apply` is true, writes the result to `backend/src/praxis/specs/personas.rs`.
#[post("/api/rust-engineer/praxis-auth")]
pub async fn run_praxis_auth(
    state: web::Data<AgentState>,
    body: web::Json<PraxisAuthRequest>,
) -> impl Responder {
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

    if body.personas.is_empty() {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({ "error": "personas must not be empty" }));
    }

    let agent = match build_rust_engineer(&project_path) {
        Ok(a) => a,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("Failed to build agent: {}", e) }));
        }
    };

    match agent.run_praxis_auth(&body.personas, body.apply).await {
        Ok(output) => {
            let mut gap_session_id = None;

            // If gaps were found and a gap session was requested, create one.
            if body.create_gap_session && !output.gaps.is_empty() {
                let chat_storage = match nocodo_agents::SqliteUserChatStorage::open(&state.db_path)
                {
                    Ok(s) => s,
                    Err(e) => {
                        log::warn!("[praxis_auth] open chat storage for gap session: {}", e);
                        return HttpResponse::InternalServerError()
                            .json(serde_json::json!({ "error": format!("Failed to open chat storage: {}", e) }));
                    }
                };

                match crate::agents_api::user_chat::handlers::create_gap_clarification_session(
                    &state.db_path,
                    body.project_id,
                    body.user_id,
                    &output.gaps,
                    &chat_storage,
                    &state.chat_notify,
                )
                .await
                {
                    Ok(sid) => {
                        log::info!("[praxis_auth] created gap_clarification session {}", sid);
                        gap_session_id = Some(sid);
                    }
                    Err(e) => {
                        log::warn!("[praxis_auth] create gap session error: {}", e);
                    }
                }
            }

            HttpResponse::Ok().json(PraxisAuthResponse {
                system_prompt: output.system_prompt,
                prompt: output.prompt,
                raw_response: output.raw_response,
                code: output.code,
                files_written: output.files_written,
                gaps: output.gaps,
                gap_session_id,
            })
        }
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

/// Build a RustEngineerAgent at a specific project path.
/// Used by the gap clarification flow to re-run Praxis Writer.
pub fn build_rust_engineer_at(
    project_path: &str,
) -> Result<nocodo_agents::RustEngineerAgent, String> {
    nocodo_agents::build_rust_engineer(project_path).map_err(|e| e.to_string())
}
