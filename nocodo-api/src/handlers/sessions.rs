use crate::models::ErrorResponse;
use crate::DbConnection;
use actix_web::{get, post, web, HttpResponse, Responder};
use rusqlite::{params, Connection};
use shared_types::{
    SessionListItem, SessionListResponse, SessionMessage, SessionResponse, SessionToolCall,
};
use tracing::{error, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use nocodo_agents::Agent;

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
        "SELECT id, agent_name, provider, model, system_prompt, user_prompt, config, status, result
         FROM agent_sessions WHERE id = ?1",
        params![session_id],
        |row| {
            let config_str: Option<String> = row.get(6)?;
            let config = config_str.and_then(|s| serde_json::from_str(&s).ok());

            Ok(SessionResponse {
                id: row.get(0)?,
                agent_name: row.get(1)?,
                provider: row.get(2)?,
                model: row.get(3)?,
                system_prompt: row.get(4)?,
                user_prompt: row.get(5)?,
                config,
                status: row.get(7)?,
                result: row.get(8)?,
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

#[get("/agents/sessions")]
pub async fn list_sessions(db: web::Data<DbConnection>) -> impl Responder {
    info!("Retrieving recent sessions");

    let conn = db.lock().unwrap();

    let sessions = match get_sessions_from_db(&conn) {
        Ok(sessions) => sessions,
        Err(e) => {
            error!(error = %e, "Failed to retrieve sessions");
            return HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to retrieve sessions: {}", e),
            });
        }
    };

    HttpResponse::Ok().json(SessionListResponse { sessions })
}

fn get_sessions_from_db(conn: &Connection) -> Result<Vec<SessionListItem>, anyhow::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, agent_name, user_prompt, started_at
         FROM agent_sessions
         ORDER BY started_at DESC
         LIMIT 50",
    )?;

    let sessions = stmt
        .query_map([], |row| {
            Ok(SessionListItem {
                id: row.get(0)?,
                agent_name: row.get(1)?,
                user_prompt: row.get(2)?,
                started_at: row.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(sessions)
}

#[derive(Serialize)]
pub struct QuestionsResponse {
    pub questions: Vec<shared_types::user_interaction::UserQuestion>,
}

#[get("/agents/sessions/{session_id}/questions")]
pub async fn get_pending_questions(
    session_id: web::Path<i64>,
    database: web::Data<std::sync::Arc<nocodo_agents::database::Database>>,
) -> impl Responder {
    let id = session_id.into_inner();
    info!(session_id = id, "Retrieving pending questions");

    match database.get_pending_questions(id) {
        Ok(questions) => {
            info!(session_id = id, question_count = questions.len(), "Retrieved pending questions");
            HttpResponse::Ok().json(QuestionsResponse { questions })
        }
        Err(e) => {
            error!(error = %e, session_id = id, "Failed to retrieve questions");
            HttpResponse::InternalServerError().json(ErrorResponse {
                error: format!("Failed to retrieve questions: {}", e),
            })
        }
    }
}

#[derive(Deserialize)]
pub struct SubmitAnswersRequest {
    pub answers: HashMap<String, String>,
}

#[post("/agents/sessions/{session_id}/answers")]
pub async fn submit_answers(
    session_id: web::Path<i64>,
    req: web::Json<SubmitAnswersRequest>,
    database: web::Data<std::sync::Arc<nocodo_agents::database::Database>>,
    llm_client: web::Data<std::sync::Arc<dyn nocodo_llm_sdk::client::LlmClient>>,
) -> impl Responder {
    let id = session_id.into_inner();
    info!(session_id = id, answer_count = req.answers.len(), "Submitting answers");

    // Store answers in database
    if let Err(e) = database.store_answers(id, &req.answers) {
        error!(error = %e, session_id = id, "Failed to store answers");
        return HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to store answers: {}", e),
        });
    }

    // Build a message with the answered questions for the agent
    let mut answers_text = String::from("User provided the following answers:\n\n");

    // Get the answered questions from database (all questions for this session)
    {
        let conn = database.connection.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT question_id, question, answer
                 FROM project_requirements_qna
                 WHERE session_id = ?1 AND answer IS NOT NULL
                 ORDER BY created_at ASC",
            )
            .map_err(|e| {
                error!(error = %e, session_id = id, "Failed to prepare query");
                e
            })
            .ok();

        if let Some(ref mut stmt) = stmt {
            let answered_questions = stmt
                .query_map([id], |row| {
                    let question: String = row.get(1)?;
                    let answer: String = row.get(2)?;
                    Ok((question, answer))
                })
                .ok();

            if let Some(rows) = answered_questions {
                for row in rows.flatten() {
                    answers_text.push_str(&format!("Q: {}\nA: {}\n\n", row.0, row.1));
                }
            }
        }
    } // conn and stmt are dropped here

    // Add the answers as a user message
    if let Err(e) = database.create_message(id, "user", &answers_text) {
        error!(error = %e, session_id = id, "Failed to create message with answers");
        return HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to create message: {}", e),
        });
    }

    // Resume the session
    if let Err(e) = database.resume_session(id) {
        error!(error = %e, session_id = id, "Failed to resume session");
        return HttpResponse::InternalServerError().json(ErrorResponse {
            error: format!("Failed to resume session: {}", e),
        });
    }

    // Spawn a background task to continue agent execution
    let database_clone = database.get_ref().clone();
    let llm_client_clone = llm_client.get_ref().clone();

    tokio::spawn(async move {
        info!(session_id = id, "Resuming agent execution with user answers");

        // Create the agent
        let agent = match crate::helpers::agents::create_user_clarification_agent(&llm_client_clone, &database_clone) {
            Ok(agent) => agent,
            Err(e) => {
                error!(error = %e, session_id = id, "Failed to create agent for resumption");
                let _ = database_clone.fail_session(id, &format!("Failed to create agent: {}", e));
                return;
            }
        };

        // Get the original user prompt from the session
        let original_prompt = {
            let conn = database_clone.connection.lock().unwrap();
            conn.query_row(
                "SELECT user_prompt FROM agent_sessions WHERE id = ?1",
                params![id],
                |row| row.get::<_, String>(0),
            ).ok()
        };

        if let Some(prompt) = original_prompt {
            // Continue execution - the agent will see the answers in the message history
            match agent.execute(&prompt, id).await {
                Ok(result) => {
                    info!(session_id = id, "Agent resumed successfully");
                    // Don't complete here if still waiting for more input
                    if !result.contains("Waiting for user") {
                        if let Err(e) = database_clone.complete_session(id, &result) {
                            error!(error = %e, session_id = id, "Failed to complete session after resumption");
                        }
                    }
                }
                Err(e) => {
                    error!(error = %e, session_id = id, "Agent execution failed after resumption");
                    let _ = database_clone.fail_session(id, &format!("Execution failed: {}", e));
                }
            }
        } else {
            error!(session_id = id, "Failed to retrieve original prompt");
            let _ = database_clone.fail_session(id, "Failed to retrieve original prompt");
        }
    });

    HttpResponse::Ok().json(serde_json::json!({
        "status": "resumed",
        "message": "Session resumed with user answers"
    }))
}
