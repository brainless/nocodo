use crate::agents_api::state::AgentState;
use actix_web::{get, post, web, HttpResponse, Responder};
use nocodo_agents::{
    build_project_manager, AgentConfig, AgentStorage, AgentType, CommentStorage,
    FinalizeSessionParams, MessageContent, PmUserSessionResult, PoSessionResult, ProductOwnerAgent,
    SqliteAgentStorage, SqliteCommentStorage, SqliteTaskStorage, SqliteUserChatStorage,
    SqliteUserStorage, StructuredQuestion, StructuredResponse, TaskStorage, UserChatMessageRow,
    UserChatSessionRow, UserChatStorage, UserStorage,
};
use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct CreateSessionRequest {
    project_id: i64,
    display_name: String,
    message: String,
}

#[derive(Serialize)]
struct CreateSessionResponse {
    session_id: i64,
    message_id: i64,
}

#[derive(Deserialize)]
struct AppendMessageRequest {
    user_id: i64,
    message: String,
    /// Defaults to "text". Pass "structured_response" with JSON in `message`
    /// when the user submits widget choices.
    content_type: Option<String>,
}

#[derive(Serialize)]
struct AppendMessageResponse {
    session_id: i64,
    message_id: i64,
}

#[derive(Serialize)]
struct GetMessagesResponse {
    session_id: i64,
    messages: Vec<UserChatMessageRow>,
}

#[derive(Deserialize)]
struct ListSessionsQuery {
    project_id: i64,
}

#[derive(Serialize)]
struct ListSessionsResponse {
    sessions: Vec<UserChatSessionRow>,
}

// ---------------------------------------------------------------------------
// POST /api/user-chats
// ---------------------------------------------------------------------------

#[post("/api/user-chats")]
pub async fn create_session(
    state: web::Data<AgentState>,
    request: web::Json<CreateSessionRequest>,
) -> impl Responder {
    let body = request.into_inner();

    let user_storage = match SqliteUserStorage::open(&state.db_path) {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Storage error: {}", e)
            }))
        }
    };

    let user_id = match user_storage.create_guest_user(body.display_name).await {
        Ok(id) => id,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to create user: {}", e)
            }))
        }
    };

    let chat_storage = match SqliteUserChatStorage::open(&state.db_path) {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Storage error: {}", e)
            }))
        }
    };

    let session_id = match chat_storage.create_session(body.project_id, user_id).await {
        Ok(id) => id,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to create session: {}", e)
            }))
        }
    };

    let message_id = match chat_storage
        .append_message(
            session_id,
            "user",
            Some(user_id),
            None,
            None,
            MessageContent::Text(body.message),
        )
        .await
    {
        Ok(id) => id,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to store message: {}", e)
            }))
        }
    };

    let db_path = state.db_path.clone();
    actix_web::rt::spawn(async move {
        run_concurrent_pm_po(db_path, session_id, message_id).await;
    });

    HttpResponse::Ok().json(CreateSessionResponse {
        session_id,
        message_id,
    })
}

// ---------------------------------------------------------------------------
// POST /api/user-chats/{session_id}/messages
// ---------------------------------------------------------------------------

#[post("/api/user-chats/{session_id}/messages")]
pub async fn append_message(
    state: web::Data<AgentState>,
    path: web::Path<i64>,
    request: web::Json<AppendMessageRequest>,
) -> impl Responder {
    let session_id = path.into_inner();
    let body = request.into_inner();

    let chat_storage = match SqliteUserChatStorage::open(&state.db_path) {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Storage error: {}", e)
            }))
        }
    };

    let session = match chat_storage.get_session(session_id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return HttpResponse::NotFound().json(serde_json::json!({
                "error": "Session not found"
            }))
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Storage error: {}", e)
            }))
        }
    };

    if session.status != "open" {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Session is already completed"
        }));
    }

    let user_content = match body.content_type.as_deref() {
        Some("structured_response") => {
            let response: StructuredResponse = match serde_json::from_str(&body.message) {
                Ok(r) => r,
                Err(e) => {
                    return HttpResponse::BadRequest().json(serde_json::json!({
                        "error": format!("Invalid structured_response JSON: {}", e)
                    }))
                }
            };

            match validate_structured_response(&chat_storage, session_id, &response).await {
                Ok(()) => MessageContent::StructuredResponse(response),
                Err(e) => {
                    return HttpResponse::BadRequest().json(serde_json::json!({
                        "error": e
                    }))
                }
            }
        }
        _ => MessageContent::Text(body.message),
    };

    let message_id = match chat_storage
        .append_message(
            session_id,
            "user",
            Some(body.user_id),
            None,
            None,
            user_content,
        )
        .await
    {
        Ok(id) => id,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to store message: {}", e)
            }))
        }
    };

    let db_path = state.db_path.clone();
    actix_web::rt::spawn(async move {
        run_concurrent_pm_po(db_path, session_id, message_id).await;
    });

    HttpResponse::Ok().json(AppendMessageResponse {
        session_id,
        message_id,
    })
}

// ---------------------------------------------------------------------------
// GET /api/user-chats/{session_id}/messages
// ---------------------------------------------------------------------------

#[get("/api/user-chats/{session_id}/messages")]
pub async fn get_messages(state: web::Data<AgentState>, path: web::Path<i64>) -> impl Responder {
    let session_id = path.into_inner();

    let chat_storage = match SqliteUserChatStorage::open(&state.db_path) {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Storage error: {}", e)
            }))
        }
    };

    let messages = match chat_storage.get_messages(session_id).await {
        Ok(m) => m,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to load messages: {}", e)
            }))
        }
    };

    HttpResponse::Ok().json(GetMessagesResponse {
        session_id,
        messages,
    })
}

// ---------------------------------------------------------------------------
// GET /api/user-chats?project_id=X
// ---------------------------------------------------------------------------

#[get("/api/user-chats")]
pub async fn list_sessions(
    state: web::Data<AgentState>,
    query: web::Query<ListSessionsQuery>,
) -> impl Responder {
    let project_id = query.project_id;

    let conn = match rusqlite::Connection::open(&state.db_path) {
        Ok(c) => c,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("DB error: {}", e)
            }))
        }
    };

    let mut stmt = match conn.prepare(
        "SELECT id, project_id, created_by_user_id, status, created_at, updated_at, completed_at \
         FROM user_chat_session WHERE project_id = ?1 ORDER BY created_at DESC",
    ) {
        Ok(s) => s,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("DB error: {}", e)
            }))
        }
    };

    let sessions = match stmt.query_map(rusqlite::params![project_id], |row| {
        Ok(UserChatSessionRow {
            id: row.get(0)?,
            project_id: row.get(1)?,
            created_by_user_id: row.get(2)?,
            status: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
            completed_at: row.get(6)?,
        })
    }) {
        Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("DB error: {}", e)
            }))
        }
    };

    HttpResponse::Ok().json(ListSessionsResponse { sessions })
}

// ---------------------------------------------------------------------------
// Background task: run PM and PO concurrently on the session
// ---------------------------------------------------------------------------

async fn run_concurrent_pm_po(db_path: String, session_id: i64, _message_id: i64) {
    let chat_storage = match SqliteUserChatStorage::open(&db_path) {
        Ok(s) => s,
        Err(e) => {
            log::warn!("user_chat: open storage: {}", e);
            return;
        }
    };

    let session = match chat_storage.get_session(session_id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            log::warn!("user_chat: session {} not found", session_id);
            return;
        }
        Err(e) => {
            log::warn!("user_chat: get session: {}", e);
            return;
        }
    };
    let project_id = session.project_id;

    let messages = match chat_storage.get_messages(session_id).await {
        Ok(m) => m,
        Err(e) => {
            log::warn!("user_chat: get messages: {}", e);
            return;
        }
    };

    let llm_messages: Vec<(String, String)> = messages
        .iter()
        .map(|m| {
            let role = match m.author_type.as_str() {
                "user" => "user",
                _ => "assistant",
            };
            let text = MessageContent::from_row(&m.content_type, &m.content).to_llm_text();
            (role.to_string(), text)
        })
        .collect();

    let config = match AgentConfig::load() {
        Ok(c) => c,
        Err(e) => {
            log::warn!("user_chat: load config: {}", e);
            return;
        }
    };

    let pm = match build_project_manager(&config, &db_path, project_id) {
        Ok(a) => a,
        Err(e) => {
            log::warn!("user_chat: build PM: {}", e);
            return;
        }
    };

    let po_storage: Arc<dyn AgentStorage> = match SqliteAgentStorage::open(&db_path) {
        Ok(s) => Arc::new(s),
        Err(e) => {
            log::warn!("user_chat: open agent storage: {}", e);
            return;
        }
    };
    let po_task_storage: Arc<dyn TaskStorage> = match SqliteTaskStorage::open(&db_path) {
        Ok(s) => Arc::new(s),
        Err(e) => {
            log::warn!("user_chat: open task storage: {}", e);
            return;
        }
    };
    let po_comment_storage: Arc<dyn CommentStorage> = match SqliteCommentStorage::open(&db_path) {
        Ok(s) => Arc::new(s),
        Err(e) => {
            log::warn!("user_chat: open comment storage: {}", e);
            return;
        }
    };
    let po = match ProductOwnerAgent::new(
        po_storage,
        po_task_storage,
        po_comment_storage,
        AgentConfig::load().unwrap(),
    ) {
        Ok(a) => a,
        Err(e) => {
            log::warn!("user_chat: build PO: {}", e);
            return;
        }
    };

    let (pm_result, po_result) = tokio::join!(
        pm.chat_for_user_session(session_id, llm_messages.clone()),
        po.respond_in_session(llm_messages.clone()),
    );

    match pm_result {
        Ok(PmUserSessionResult::Finalized { params }) => {
            handle_pm_finalized(&db_path, session_id, project_id, params, &chat_storage).await;
        }
        Ok(PmUserSessionResult::Questions(questions)) => {
            for q in questions {
                if let Err(e) = chat_storage
                    .append_message(
                        session_id,
                        "agent",
                        None,
                        Some(AgentType::ProjectManager),
                        None,
                        MessageContent::StructuredQuestion(q),
                    )
                    .await
                {
                    log::warn!("user_chat: store PM question: {}", e);
                }
            }
        }
        Ok(PmUserSessionResult::Text(t)) => {
            if let Err(e) = chat_storage
                .append_message(
                    session_id,
                    "agent",
                    None,
                    Some(AgentType::ProjectManager),
                    None,
                    MessageContent::Text(t),
                )
                .await
            {
                log::warn!("user_chat: store PM response: {}", e);
            }
        }
        Err(e) => {
            log::warn!("user_chat: PM error: {}", e);
        }
    }

    match po_result {
        Ok(PoSessionResult::Text(t)) if !t.trim().is_empty() => {
            if let Err(e) = chat_storage
                .append_message(
                    session_id,
                    "agent",
                    None,
                    Some(AgentType::ProductOwner),
                    None,
                    MessageContent::Text(t),
                )
                .await
            {
                log::warn!("user_chat: store PO response: {}", e);
            }
        }
        Ok(PoSessionResult::Text(_)) => {}
        Ok(PoSessionResult::Questions(questions)) => {
            for q in questions {
                if let Err(e) = chat_storage
                    .append_message(
                        session_id,
                        "agent",
                        None,
                        Some(AgentType::ProductOwner),
                        None,
                        MessageContent::StructuredQuestion(q),
                    )
                    .await
                {
                    log::warn!("user_chat: store PO question: {}", e);
                }
            }
        }
        Ok(PoSessionResult::Silent) => {}
        Err(e) => {
            log::warn!("user_chat: PO error: {}", e);
        }
    }
}

async fn validate_structured_response(
    chat_storage: &SqliteUserChatStorage,
    session_id: i64,
    response: &StructuredResponse,
) -> Result<(), String> {
    if response.selected.is_empty() {
        return Err("Structured response requires at least one selected option".to_string());
    }

    let messages = chat_storage
        .get_messages(session_id)
        .await
        .map_err(|e| format!("Failed to load session messages: {}", e))?;

    let question_row = messages
        .iter()
        .find(|m| m.id == response.question_message_id)
        .ok_or_else(|| "question_message_id not found in this session".to_string())?;

    if question_row.content_type != "structured_question" {
        return Err("question_message_id does not reference a structured_question".to_string());
    }

    let question: StructuredQuestion = serde_json::from_str(&question_row.content)
        .map_err(|e| format!("Invalid stored structured_question JSON: {}", e))?;

    let allowed_options = match &question.kind {
        nocodo_agents::QuestionKind::SingleChoice { options }
        | nocodo_agents::QuestionKind::MultipleChoice { options } => options,
    };

    for selected in &response.selected {
        if !allowed_options.iter().any(|opt| opt == selected) {
            return Err(format!(
                "Selected option '{}' is not valid for question_message_id={}",
                selected, response.question_message_id
            ));
        }
    }

    if matches!(
        question.kind,
        nocodo_agents::QuestionKind::SingleChoice { .. }
    ) && response.selected.len() != 1
    {
        return Err("Single-choice question requires exactly one selected option".to_string());
    }

    Ok(())
}

async fn handle_pm_finalized(
    db_path: &str,
    session_id: i64,
    project_id: i64,
    params: FinalizeSessionParams,
    _chat_storage: &SqliteUserChatStorage,
) {
    let ts = now();
    let FinalizeSessionParams {
        final_message,
        epic_title,
        epic_description,
        tasks,
    } = params;

    let mut conn = match rusqlite::Connection::open(db_path) {
        Ok(c) => c,
        Err(e) => {
            log::warn!("user_chat: open db: {}", e);
            return;
        }
    };

    let tx = match conn.transaction() {
        Ok(t) => t,
        Err(e) => {
            log::warn!("user_chat: begin tx: {}", e);
            return;
        }
    };

    if let Err(e) = tx.execute(
        "INSERT INTO user_chat_message (session_id, author_type, agent_type, content_type, content, created_at) \
         VALUES (?1, 'agent', 'project_manager', 'text', ?2, ?3)",
        rusqlite::params![session_id, final_message, ts],
    ) {
        log::warn!("user_chat: insert PM message: {}", e);
        return;
    }

    if let Err(e) = tx.execute(
        "INSERT INTO epic (project_id, title, description, created_by_agent, status, created_at, updated_at) \
         VALUES (?1, ?2, ?3, 'project_manager', 'open', ?4, ?5)",
        rusqlite::params![project_id, epic_title, epic_description, ts, ts],
    ) {
        log::warn!("user_chat: insert epic: {}", e);
        return;
    }
    let epic_id = tx.last_insert_rowid();

    let mut task_ids = Vec::new();
    for task_def in &tasks {
        match tx.execute(
            "INSERT INTO task (project_id, epic_id, title, description, source_prompt, assigned_to_agent, \
             status, source_session_id, created_by_agent, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'draft', ?7, 'project_manager', ?8, ?9)",
            rusqlite::params![
                project_id,
                epic_id,
                task_def.title,
                task_def.description,
                task_def.description,
                task_def.assigned_to_agent,
                session_id,
                ts,
                ts
            ],
        ) {
            Ok(_) => {
                task_ids.push(tx.last_insert_rowid());
            }
            Err(e) => {
                log::warn!("user_chat: insert task: {}", e);
                return;
            }
        }
    }

    if let Err(e) = tx.execute(
        "UPDATE user_chat_session SET status = 'completed', completed_at = ?1, updated_at = ?1 WHERE id = ?2",
        rusqlite::params![ts, session_id],
    ) {
        log::warn!("user_chat: complete session: {}", e);
        return;
    }

    if let Err(e) = tx.commit() {
        log::warn!("user_chat: commit tx: {}", e);
        return;
    }

    let val_po_storage: Arc<dyn AgentStorage> = match SqliteAgentStorage::open(db_path) {
        Ok(s) => Arc::new(s),
        Err(e) => {
            log::warn!("user_chat: PO validate open agent storage: {}", e);
            return;
        }
    };
    let val_po_task_storage: Arc<dyn TaskStorage> = match SqliteTaskStorage::open(db_path) {
        Ok(s) => Arc::new(s),
        Err(e) => {
            log::warn!("user_chat: PO validate open task storage: {}", e);
            return;
        }
    };
    let val_po_comment_storage: Arc<dyn CommentStorage> = match SqliteCommentStorage::open(db_path)
    {
        Ok(s) => Arc::new(s),
        Err(e) => {
            log::warn!("user_chat: PO validate open comment storage: {}", e);
            return;
        }
    };
    let val_config = match AgentConfig::load() {
        Ok(c) => c,
        Err(e) => {
            log::warn!("user_chat: PO validate load config: {}", e);
            return;
        }
    };
    let val_po = match ProductOwnerAgent::new(
        val_po_storage,
        val_po_task_storage,
        val_po_comment_storage,
        val_config,
    ) {
        Ok(a) => a,
        Err(e) => {
            log::warn!("user_chat: PO validate build: {}", e);
            return;
        }
    };

    if let Err(e) = val_po.validate_tasks(task_ids).await {
        log::warn!("user_chat: PO validate tasks: {}", e);
    }
}
