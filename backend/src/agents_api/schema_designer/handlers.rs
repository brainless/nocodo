use crate::agents_api::state::AgentState;
use crate::agents_api::schema_designer::types::{
    AgentResponsePayload, ChatHistoryMessage, ChatHistoryResponse, ChatRequest, ChatResponse,
    ListSessionsQuery, ListSessionsResponse, MessageResponse, SchemaPreviewResponse, SessionItem,
};
use actix_web::{get, post, web, HttpResponse, Responder};
use nocodo_agents::{build_schema_designer, AgentResponse, AgentStorage, SqliteAgentStorage, SqliteSchemaStorage};
use shared_types::SchemaDef;
use std::time::Duration;

/// POST /api/agents/schema-designer/chat
/// Send a message to the schema designer agent.
/// Creates a new session if session_id is None, or continues existing session.
/// Returns immediately with session_id and message_id.
#[post("/api/agents/schema-designer/chat")]
pub async fn send_chat_message(
    state: web::Data<AgentState>,
    request: web::Json<ChatRequest>,
) -> impl Responder {
    const AGENT_TYPE: &str = "schema_designer";
    let ChatRequest {
        project_id,
        session_id,
        message,
    } = request.into_inner();

    // Get or create session to determine the actual session_id
    let agent_storage = match SqliteAgentStorage::open(&state.db_path) {
        Ok(storage) => storage,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to open storage: {}", e)
            }))
        }
    };

    let target_session_id = match session_id {
        Some(existing_session_id) => {
            match agent_storage.get_session_by_id(existing_session_id).await {
                Ok(Some(session))
                    if session.project_id == project_id && session.agent_type == AGENT_TYPE =>
                {
                    existing_session_id
                }
                Ok(Some(_)) => {
                    return HttpResponse::BadRequest().json(serde_json::json!({
                        "error": "Invalid session_id for this project/agent"
                    }))
                }
                Ok(None) => {
                    return HttpResponse::BadRequest().json(serde_json::json!({
                        "error": "session_id not found"
                    }))
                }
                Err(e) => {
                    return HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": format!("Session error: {}", e)
                    }))
                }
            }
        }
        None => match agent_storage.create_session(project_id, AGENT_TYPE).await {
            Ok(session) => session.id.unwrap_or(0),
            Err(e) => {
                return HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": format!("Session create error: {}", e)
                }))
            }
        },
    };

    // Store user message and get message ID
    let user_msg_id = match agent_storage
        .create_message(nocodo_agents::storage::ChatMessage {
            id: None,
            session_id: target_session_id,
            role: "user".to_string(),
            content: message.clone(),
            tool_call_id: None,
            created_at: 0,
        })
        .await
    {
        Ok(id) => id,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to store message: {}", e)
            }))
        }
    };

    // Mark as pending in response storage
    state.response_storage.store_pending(user_msg_id).await;

    // Spawn the agent processing in a background task
    let response_storage = state.response_storage.clone();
    let config = state.config.clone();
    let db_path = state.db_path.clone();

    actix_web::rt::spawn(async move {
        // Build the agent
        let agent = match build_schema_designer(&config, &db_path, project_id) {
            Ok(agent) => agent,
            Err(e) => {
                response_storage
                    .store_text(user_msg_id, format!("Error: {}", e))
                    .await;
                return;
            }
        };

        match agent.chat_with_session(target_session_id, false).await {
            Ok(response) => match response {
                AgentResponse::Text(text) => {
                    response_storage.store_text(user_msg_id, text).await;
                }
                AgentResponse::SchemaGenerated { text, schema, .. } => {
                    let schema_json = serde_json::to_string(&schema).unwrap_or_default();
                    response_storage
                        .store_schema(user_msg_id, text, schema_json)
                        .await;
                }
                AgentResponse::Stopped(text) => {
                    response_storage.store_stopped(user_msg_id, text).await;
                }
            },
            Err(e) => {
                response_storage
                    .store_text(user_msg_id, format!("Error: {}", e))
                    .await;
            }
        }
    });

    HttpResponse::Ok().json(ChatResponse {
        session_id: target_session_id,
        message_id: user_msg_id,
        status: "pending".to_string(),
    })
}

/// GET /api/agents/sessions?project_id=X[&agent_type=Y]
/// List all sessions for a project, optionally filtered by agent type.
#[get("/api/agents/sessions")]
pub async fn list_sessions(
    state: web::Data<AgentState>,
    query: web::Query<ListSessionsQuery>,
) -> impl Responder {
    let agent_storage = match SqliteAgentStorage::open(&state.db_path) {
        Ok(storage) => storage,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to open storage: {}", e)
            }));
        }
    };

    match agent_storage
        .list_sessions(query.project_id, query.agent_type.as_deref())
        .await
    {
        Ok(sessions) => HttpResponse::Ok().json(ListSessionsResponse {
            sessions: sessions
                .into_iter()
                .map(|s| SessionItem {
                    id: s.id.unwrap_or(0),
                    project_id: s.project_id,
                    agent_type: s.agent_type,
                    created_at: s.created_at,
                })
                .collect(),
        }),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to list sessions: {}", e)
        })),
    }
}

/// GET /api/agents/schema-designer/sessions/{session_id}/messages
/// Returns the persisted chat history for a session.
#[get("/api/agents/schema-designer/sessions/{session_id}/messages")]
pub async fn get_session_messages(
    state: web::Data<AgentState>,
    path: web::Path<i64>,
) -> impl Responder {
    let session_id = path.into_inner();

    let agent_storage = match SqliteAgentStorage::open(&state.db_path) {
        Ok(storage) => storage,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to open storage: {}", e)
            }));
        }
    };

    let messages = match agent_storage.get_messages(session_id).await {
        Ok(messages) => messages
            .into_iter()
            .map(|m| ChatHistoryMessage {
                id: m.id.unwrap_or(0),
                role: m.role,
                content: m.content,
                created_at: m.created_at,
            })
            .collect(),
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to load messages: {}", e)
            }));
        }
    };

    HttpResponse::Ok().json(ChatHistoryResponse {
        session_id,
        messages,
    })
}

/// GET /api/agents/schema-designer/messages/{message_id}/response
/// Long-poll for the agent's response to a specific message.
/// Returns immediately if response is ready, otherwise waits up to browser max timeout.
#[get("/api/agents/schema-designer/messages/{message_id}/response")]
pub async fn get_message_response(
    state: web::Data<AgentState>,
    path: web::Path<i64>,
) -> HttpResponse {
    let message_id = path.into_inner();
    let max_wait = Duration::from_secs(60); // Browser-friendly timeout
    let poll_interval = Duration::from_millis(500);
    let start_time = std::time::Instant::now();

    log::info!("Polling for response for message_id: {}", message_id);

    // Poll for response with timeout
    loop {
        match state.response_storage.get(message_id).await {
            Some(stored) => {
                if stored.response_type == "pending" {
                    // Keep the HTTP request open while work is still pending.
                    if start_time.elapsed() >= max_wait {
                        log::info!("Polling timeout for pending message_id: {}", message_id);
                        return HttpResponse::Ok().json(MessageResponse {
                            message_id,
                            response: AgentResponsePayload::Pending,
                        });
                    }
                    actix_web::rt::time::sleep(poll_interval).await;
                    continue;
                }

                log::info!(
                    "Found stored response for message_id: {} with type: {}",
                    message_id,
                    stored.response_type
                );
                let payload = match stored.response_type.as_str() {
                    "text" => AgentResponsePayload::Text { text: stored.text },
                    "schema_generated" => {
                        let schema = match stored
                            .schema_json
                            .and_then(|s| serde_json::from_str::<SchemaDef>(&s).ok())
                        {
                            Some(s) => s,
                            None => {
                                log::error!(
                                    "Failed to deserialize stored schema for message_id: {}",
                                    message_id
                                );
                                return HttpResponse::InternalServerError().json(
                                    serde_json::json!({ "error": "Stored schema is corrupt" }),
                                );
                            }
                        };
                        AgentResponsePayload::SchemaGenerated {
                            text: stored.text,
                            schema,
                            preview: true,
                        }
                    }
                    "stopped" => AgentResponsePayload::Stopped { text: stored.text },
                    _ => AgentResponsePayload::Pending,
                };

                return HttpResponse::Ok().json(MessageResponse {
                    message_id,
                    response: payload,
                });
            }
            None => {
                log::debug!("No response yet for message_id: {}", message_id);
                // No response yet, check timeout
                if start_time.elapsed() >= max_wait {
                    log::info!("Polling timeout for message_id: {}", message_id);
                    return HttpResponse::Ok().json(MessageResponse {
                        message_id,
                        response: AgentResponsePayload::Pending,
                    });
                }
            }
        }

        // Wait before next poll
        actix_web::rt::time::sleep(poll_interval).await;
    }
}

/// GET /api/agents/schema-designer/sessions/{session_id}/schema
/// Returns the latest persisted schema for a session (or 404 if none).
#[get("/api/agents/schema-designer/sessions/{session_id}/schema")]
pub async fn get_session_schema(
    state: web::Data<AgentState>,
    path: web::Path<i64>,
) -> HttpResponse {
    let session_id = path.into_inner();

    let schema_storage = match SqliteSchemaStorage::open(&state.db_path) {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(serde_json::json!({ "error": format!("Storage error: {}", e) }));
        }
    };

    match schema_storage.get_latest_schema_for_session(session_id).await {
        Ok(Some((schema_json, version))) => {
            match serde_json::from_str::<SchemaDef>(&schema_json) {
                Ok(schema) => HttpResponse::Ok().json(SchemaPreviewResponse { schema, version }),
                Err(e) => HttpResponse::InternalServerError()
                    .json(serde_json::json!({ "error": format!("Schema corrupt: {}", e) })),
            }
        }
        Ok(None) => HttpResponse::NotFound()
            .json(serde_json::json!({ "error": "No schema generated for this session yet" })),
        Err(e) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "error": format!("Query error: {}", e) })),
    }
}
