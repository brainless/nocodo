use crate::agents_api::state::AgentState;
use crate::agents_api::schema_designer::types::{
    AgentResponsePayload, ChatHistoryMessage, ChatHistoryResponse, ChatRequest, ChatResponse,
    ListTasksQuery, ListTasksResponse, MessageResponse, SchemaCodegenResponse, SchemaPreviewQuery,
    SchemaPreviewResponse, TaskItem,
};
use actix_web::{get, post, web, HttpResponse, Responder};
use nocodo_agents::{
    build_schema_designer, AgentResponse, AgentStorage, SchemaStorage, SqliteAgentStorage,
    SqliteSchemaStorage, SqliteTaskStorage, Task, TaskStatus, TaskStorage,
};
use shared_types::SchemaDef;
use std::time::Duration;

const AGENT_TYPE: &str = "schema_designer";

/// POST /api/agents/schema-designer/chat
/// If task_id is None, creates a new task + session from the prompt.
/// If task_id is Some, continues the existing task's session.
/// Returns immediately with task_id and message_id.
#[post("/api/agents/schema-designer/chat")]
pub async fn send_chat_message(
    state: web::Data<AgentState>,
    request: web::Json<ChatRequest>,
) -> impl Responder {
    let ChatRequest { project_id, task_id, message } = request.into_inner();

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

    // Resolve task_id and session_id — create both if this is a new prompt.
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
                    // Task exists but no session yet — create one.
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

    // Store the user message.
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
    let config = state.config.clone();
    let db_path = state.db_path.clone();

    actix_web::rt::spawn(async move {
        let agent = match build_schema_designer(&config, &db_path, project_id) {
            Ok(a) => a,
            Err(e) => {
                response_storage
                    .store_text(user_msg_id, format!("Error: {}", e))
                    .await;
                return;
            }
        };

        match agent.chat_with_session(session_id, false).await {
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
                AgentResponse::Question(text) => {
                    response_storage.store_question(user_msg_id, text).await;
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
        task_id: actual_task_id,
        message_id: user_msg_id,
        status: "pending".to_string(),
    })
}

/// GET /api/agents/tasks?project_id=X
/// List all tasks for a project.
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
                })
                .collect(),
        }),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to list tasks: {}", e)
        })),
    }
}

/// GET /api/agents/schema-designer/tasks/{task_id}/messages
#[get("/api/agents/schema-designer/tasks/{task_id}/messages")]
pub async fn get_task_messages(
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
            return HttpResponse::Ok().json(ChatHistoryResponse { task_id, messages: vec![] });
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Session lookup error: {}", e)
            }));
        }
    };

    let session_id = session.id.unwrap_or(0);

    let schema_storage = match SqliteSchemaStorage::open(&state.db_path) {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to open schema storage: {}", e)
            }));
        }
    };

    match agent_storage.get_messages(session_id).await {
        Ok(msgs) => {
            let mut enriched = Vec::new();
            for m in msgs {
                let mut schema_version: Option<i64> = None;
                let mut content = m.content.clone();

                if m.role == "assistant" {
                    match m.tool_name.as_deref() {
                        Some("generate_schema") => {
                            if let Ok(v) = schema_storage
                                .get_schema_version_by_json(session_id, &m.content)
                                .await
                            {
                                schema_version = v;
                            }
                            content = String::new();
                        }
                        Some("ask_user") => {
                            if let Ok(args) =
                                serde_json::from_str::<serde_json::Value>(&m.content)
                            {
                                if let Some(q) = args.get("question").and_then(|v| v.as_str()) {
                                    content = q.to_string();
                                }
                            }
                        }
                        Some("stop_agent") => {
                            if let Ok(args) =
                                serde_json::from_str::<serde_json::Value>(&m.content)
                            {
                                if let Some(r) = args.get("reply").and_then(|v| v.as_str()) {
                                    content = r.to_string();
                                }
                            }
                        }
                        _ => {}
                    }
                }

                enriched.push(ChatHistoryMessage {
                    id: m.id.unwrap_or(0),
                    role: m.role,
                    content,
                    created_at: m.created_at,
                    schema_version,
                });
            }
            HttpResponse::Ok().json(ChatHistoryResponse { task_id, messages: enriched })
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": format!("Failed to load messages: {}", e)
        })),
    }
}

/// GET /api/agents/schema-designer/messages/{message_id}/response
#[get("/api/agents/schema-designer/messages/{message_id}/response")]
pub async fn get_message_response(
    state: web::Data<AgentState>,
    path: web::Path<i64>,
) -> HttpResponse {
    let message_id = path.into_inner();
    let max_wait = Duration::from_secs(60);
    let poll_interval = Duration::from_millis(500);
    let start_time = std::time::Instant::now();

    log::info!("Polling for response for message_id: {}", message_id);

    loop {
        match state.response_storage.get(message_id).await {
            Some(stored) => {
                if stored.response_type == "pending" {
                    if start_time.elapsed() >= max_wait {
                        return HttpResponse::Ok().json(MessageResponse {
                            message_id,
                            response: AgentResponsePayload::Pending,
                        });
                    }
                    actix_web::rt::time::sleep(poll_interval).await;
                    continue;
                }

                let payload = match stored.response_type.as_str() {
                    "text" => AgentResponsePayload::Text { text: stored.text },
                    "schema_generated" => {
                        let schema = match stored
                            .schema_json
                            .and_then(|s| serde_json::from_str::<SchemaDef>(&s).ok())
                        {
                            Some(s) => s,
                            None => {
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
                    "question" => AgentResponsePayload::Question { text: stored.text },
                    _ => AgentResponsePayload::Pending,
                };

                return HttpResponse::Ok().json(MessageResponse { message_id, response: payload });
            }
            None => {
                if start_time.elapsed() >= max_wait {
                    return HttpResponse::Ok().json(MessageResponse {
                        message_id,
                        response: AgentResponsePayload::Pending,
                    });
                }
            }
        }
        actix_web::rt::time::sleep(poll_interval).await;
    }
}

/// GET /api/agents/schema-designer/tasks/{task_id}/schema?version=N
#[get("/api/agents/schema-designer/tasks/{task_id}/schema")]
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

/// GET /api/agents/schema-designer/tasks/{task_id}/codegen
#[get("/api/agents/schema-designer/tasks/{task_id}/codegen")]
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
