use crate::models::{ErrorResponse, SessionMessage, SessionResponse, SessionToolCall};
use actix_web::{get, web, HttpResponse, Responder};
use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex};
use tracing::{error, info, warn};

pub type DbConnection = Arc<Mutex<Connection>>;

#[get("/agents/sessions/{session_id}")]
pub async fn get_session(
    session_id: web::Path<i64>,
    db: web::Data<DbConnection>,
) -> impl Responder {
    let id = session_id.into_inner();
    info!(session_id = id, "Retrieving session");

    let conn = db.lock().unwrap();

    let session = match get_session_from_db(&conn, id) {
        Ok(Some(session)) => session,
        Ok(None) => {
            warn!(session_id = id, "Session not found");
            return HttpResponse::NotFound().json(ErrorResponse {
                error: format!("Session {} not found", id),
            });
        }
        Err(e) => {
            error!(error = %e, session_id = id, "Failed to retrieve session");
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to retrieve session: {}", e),
            });
        }
    };

    HttpResponse::Ok().json(session)
}

fn get_session_from_db(
    conn: &Connection,
    session_id: i64,
) -> Result<Option<SessionResponse>, anyhow::Error> {
    let session = conn.query_row(
        "SELECT id, agent_name, provider, model, system_prompt, user_prompt, status, result
         FROM agent_sessions WHERE id = ?1",
        params![session_id],
        |row| {
            Ok(SessionResponse {
                id: row.get(0)?,
                agent_name: row.get(1)?,
                provider: row.get(2)?,
                model: row.get(3)?,
                system_prompt: row.get(4)?,
                user_prompt: row.get(5)?,
                status: row.get(6)?,
                result: row.get(7)?,
                messages: Vec::new(),
                tool_calls: Vec::new(),
            })
        },
    );

    let mut session = match session {
        Ok(s) => s,
        Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
        Err(e) => return Err(e.into()),
    };

    let messages = conn
        .prepare(
            "SELECT role, content, created_at
             FROM agent_messages
             WHERE session_id = ?1
             ORDER BY created_at ASC",
        )?
        .query_map([session_id], |row| {
            Ok(SessionMessage {
                role: row.get(0)?,
                content: row.get(1)?,
                created_at: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    session.messages = messages;

    let tool_calls = conn
        .prepare(
            "SELECT tool_name, request, response, status, execution_time_ms
             FROM agent_tool_calls
             WHERE session_id = ?1
             ORDER BY created_at ASC",
        )?
        .query_map([session_id], |row| {
            let request_str: String = row.get(1)?;
            let request: serde_json::Value = serde_json::from_str(&request_str).map_err(|_e| {
                rusqlite::Error::InvalidColumnType(
                    1,
                    request_str.clone(),
                    rusqlite::types::Type::Text,
                )
            })?;

            let response_str: Option<String> = row.get(2)?;
            let response = response_str.and_then(|s| serde_json::from_str(&s).ok());

            Ok(SessionToolCall {
                tool_name: row.get(0)?,
                request,
                response,
                status: row.get(3)?,
                execution_time_ms: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    session.tool_calls = tool_calls;

    Ok(Some(session))
}
