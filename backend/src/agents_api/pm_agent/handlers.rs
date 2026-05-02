use crate::agents_api::state::AgentState;
use crate::agents_api::pm_agent::types::{
    PmChatHistoryMessage, PmChatHistoryResponse, PmChatRequest, PmChatResponse,
    PmMessageResponse, PmResponsePayload,
};
use actix_web::{get, post, web, HttpResponse, Responder};
use nocodo_agents::{
    build_pm_agent, AgentConfig, AgentStorage, PmResponse, SqliteAgentStorage, SqliteTaskStorage,
    Task, TaskStatus, TaskStorage,
};
use std::time::Duration;

const AGENT_TYPE: &str = "project_manager";

/// POST /api/agents/pm/chat
#[post("/api/agents/pm/chat")]
pub async fn send_pm_chat_message(
    state: web::Data<AgentState>,
    request: web::Json<PmChatRequest>,
) -> impl Responder {
    let PmChatRequest { project_id, task_id, message } = request.into_inner();

    let task_storage = match SqliteTaskStorage::open(&state.db_path) {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to open task storage: {}", e)
            }))
        }
    };

    let agent_storage = match SqliteAgentStorage::open(&state.db_path) {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to open agent storage: {}", e)
            }))
        }
    };

    let (actual_task_id, session_id) = match task_id {
        None => {
            let title: String = message.chars().take(100).collect();
            let tid = match task_storage
                .create_task(Task {
                    id: None,
                    project_id,
                    epic_id: None,
                    title,
                    description: message.clone(),
                    source_prompt: message.clone(),
                    assigned_to_agent: AGENT_TYPE.to_string(),
                    status: TaskStatus::InProgress,
                    depends_on_task_id: None,
                    created_by_agent: "user".to_string(),
                    created_at: 0,
                    updated_at: 0,
                })
                .await
            {
                Ok(id) => id,
                Err(e) => {
                    return HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": format!("Failed to create task: {}", e)
                    }))
                }
            };

            let session = match agent_storage
                .create_task_session(project_id, tid, AGENT_TYPE)
                .await
            {
                Ok(s) => s,
                Err(e) => {
                    return HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": format!("Failed to create session: {}", e)
                    }))
                }
            };

            (tid, session.id.unwrap_or(0))
        }
        Some(tid) => {
            let session = match agent_storage.get_session_by_task(tid, AGENT_TYPE).await {
                Ok(Some(s)) => s,
                Ok(None) => {
                    let task = match task_storage.get_task(tid).await {
                        Ok(Some(t)) => t,
                        Ok(None) => {
                            return HttpResponse::BadRequest().json(serde_json::json!({
                                "error": "task_id not found"
                            }))
                        }
                        Err(e) => {
                            return HttpResponse::InternalServerError().json(serde_json::json!({
                                "error": format!("Failed to load task: {}", e)
                            }))
                        }
                    };
                    match agent_storage
                        .create_task_session(task.project_id, tid, AGENT_TYPE)
                        .await
                    {
                        Ok(s) => s,
                        Err(e) => {
                            return HttpResponse::InternalServerError().json(serde_json::json!({
                                "error": format!("Failed to create session: {}", e)
                            }))
                        }
                    }
                }
                Err(e) => {
                    return HttpResponse::InternalServerError().json(serde_json::json!({
                        "error": format!("Session lookup error: {}", e)
                    }))
                }
            };
            (tid, session.id.unwrap_or(0))
        }
    };

    let user_msg_id = match agent_storage
        .create_message(nocodo_agents::ChatMessage {
            id: None,
            session_id,
            role: "user".to_string(),
            agent_type: None,
            content: message.clone(),
            tool_call_id: None,
            tool_name: None,
            turn_id: None,
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

    state.response_storage.store_pending(user_msg_id).await;

    let response_storage = state.response_storage.clone();
    let db_path = state.db_path.clone();

    actix_web::rt::spawn(async move {
        let config = match AgentConfig::load_pm() {
            Ok(c) => c,
            Err(e) => {
                response_storage
                    .store_text(user_msg_id, format!("Config error: {}", e))
                    .await;
                return;
            }
        };

        let agent = match build_pm_agent(&config, &db_path, project_id) {
            Ok(a) => a,
            Err(e) => {
                response_storage
                    .store_text(user_msg_id, format!("Error: {}", e))
                    .await;
                return;
            }
        };

        match agent.chat_with_session(session_id, actual_task_id).await {
            Ok(response) => match response {
                PmResponse::Text(text) => {
                    response_storage.store_text(user_msg_id, text).await;
                }
                PmResponse::Stopped(text) => {
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

    HttpResponse::Ok().json(PmChatResponse {
        task_id: actual_task_id,
        message_id: user_msg_id,
        status: "pending".to_string(),
    })
}

/// GET /api/agents/pm/messages/{message_id}/response
#[get("/api/agents/pm/messages/{message_id}/response")]
pub async fn get_pm_message_response(
    state: web::Data<AgentState>,
    path: web::Path<i64>,
) -> HttpResponse {
    let message_id = path.into_inner();
    let max_wait = Duration::from_secs(60);
    let poll_interval = Duration::from_millis(500);
    let start_time = std::time::Instant::now();

    loop {
        match state.response_storage.get(message_id).await {
            Some(stored) => {
                if stored.response_type == "pending" {
                    if start_time.elapsed() >= max_wait {
                        return HttpResponse::Ok().json(PmMessageResponse {
                            message_id,
                            response: PmResponsePayload::Pending,
                        });
                    }
                    actix_web::rt::time::sleep(poll_interval).await;
                    continue;
                }

                let payload = match stored.response_type.as_str() {
                    "text" => PmResponsePayload::Text { text: stored.text },
                    "stopped" => PmResponsePayload::Stopped { text: stored.text },
                    _ => PmResponsePayload::Pending,
                };

                return HttpResponse::Ok()
                    .json(PmMessageResponse { message_id, response: payload });
            }
            None => {
                if start_time.elapsed() >= max_wait {
                    return HttpResponse::Ok().json(PmMessageResponse {
                        message_id,
                        response: PmResponsePayload::Pending,
                    });
                }
            }
        }
        actix_web::rt::time::sleep(poll_interval).await;
    }
}

/// GET /api/agents/pm/tasks/{task_id}/messages
#[get("/api/agents/pm/tasks/{task_id}/messages")]
pub async fn get_pm_task_messages(
    state: web::Data<AgentState>,
    path: web::Path<i64>,
) -> impl Responder {
    let task_id = path.into_inner();

    let agent_storage = match SqliteAgentStorage::open(&state.db_path) {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to open storage: {}", e)
            }));
        }
    };

    let session = match agent_storage.get_session_by_task(task_id, AGENT_TYPE).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return HttpResponse::Ok()
                .json(PmChatHistoryResponse { task_id, messages: vec![] });
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Session lookup error: {}", e)
            }));
        }
    };

    let session_id = session.id.unwrap_or(0);

    match agent_storage.get_messages(session_id).await {
        Ok(msgs) => {
            let messages = msgs
                .into_iter()
                .filter_map(|m| {
                    // Skip raw tool invocation rows (role=assistant with tool_call_id) —
                    // the tool result row carries the human-readable outcome.
                    if m.role == "assistant" && m.tool_call_id.is_some() {
                        return None;
                    }
                    // Skip tool result rows for list_pending_review_tasks — internal triage
                    if m.role == "tool"
                        && m.tool_name.as_deref() == Some("list_pending_review_tasks")
                    {
                        return None;
                    }

                    let content = if m.role == "tool" {
                        m.content.clone()
                    } else {
                        m.content.clone()
                    };

                    Some(PmChatHistoryMessage {
                        id: m.id.unwrap_or(0),
                        role: m.role,
                        content,
                        tool_name: m.tool_name,
                        created_at: m.created_at,
                    })
                })
                .collect();

            HttpResponse::Ok().json(PmChatHistoryResponse { task_id, messages })
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to load messages: {}", e)
        })),
    }
}
