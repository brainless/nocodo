use crate::agents_api::state::AgentState;
use actix_web::{get, post, web, HttpResponse, Responder};
use nocodo_agents::{
    build_project_manager, AgentConfig, AgentStorage, AgentType, CommentStorage,
    FinalizeSessionParams, MessageContent, PmUserSessionResult, PoSessionResult, ProductOwnerAgent,
    ProjectNoteStorage, SqliteAgentStorage, SqliteCommentStorage, SqliteProjectNoteStorage,
    SqliteTaskStorage, SqliteUserChatStorage, SqliteUserStorage, StructuredQuestion,
    StructuredResponse, TaskStorage, UserChatMessageRow, UserChatSessionRow, UserChatStorage,
    UserStorage,
};
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, Notify};

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// Merge consecutive messages with the same role into one, joining their text with a newline.
/// This is required because the DB stores e.g. a greeting and a structured question as separate
/// rows (both "assistant"), but LLM APIs require strictly alternating user/assistant turns.
fn merge_consecutive_roles(messages: Vec<(String, String)>) -> Vec<(String, String)> {
    let mut result: Vec<(String, String)> = Vec::new();
    for (role, text) in messages {
        match result.last_mut() {
            Some(last) if last.0 == role => {
                last.1.push('\n');
                last.1.push_str(&text);
            }
            _ => result.push((role, text)),
        }
    }
    result
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
    handoff_session_id: Option<i64>,
}

#[derive(Deserialize)]
struct PollQuery {
    after: i64,
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
    let chat_notify = state.chat_notify.clone();
    actix_web::rt::spawn(async move {
        run_po_intake(db_path, session_id, user_id, chat_notify).await;
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

    let user_id = session.created_by_user_id;
    let db_path = state.db_path.clone();
    let chat_notify = state.chat_notify.clone();
    actix_web::rt::spawn(async move {
        run_po_intake(db_path, session_id, user_id, chat_notify).await;
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
        handoff_session_id: session.handoff_session_id,
    })
}

// ---------------------------------------------------------------------------
// GET /api/user-chats/{session_id}/poll?after=X
// Long-poll: holds the connection until a message newer than `after` arrives,
// or 30 s elapses.
// ---------------------------------------------------------------------------

#[get("/api/user-chats/{session_id}/poll")]
pub async fn poll_messages(
    state: web::Data<AgentState>,
    path: web::Path<i64>,
    query: web::Query<PollQuery>,
) -> impl Responder {
    let session_id = path.into_inner();
    let after = query.after;

    let notify = state.get_session_notify(session_id).await;
    let notified = notify.notified();
    tokio::pin!(notified);
    notified.as_mut().enable();

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

    let messages = match chat_storage.get_messages(session_id).await {
        Ok(m) => m,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to load messages: {}", e)
            }))
        }
    };

    // Return immediately if there are already new messages.
    let has_new = messages.iter().any(|m| m.id > after);
    if has_new {
        return HttpResponse::Ok().json(GetMessagesResponse {
            session_id,
            messages,
            handoff_session_id: session.handoff_session_id,
        });
    }

    let _ = tokio::time::timeout(Duration::from_secs(30), notified).await;

    // Re-fetch after wake or timeout.
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
        handoff_session_id: session.handoff_session_id,
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
        "SELECT id, project_id, created_by_user_id, status, created_at, updated_at, completed_at, handoff_session_id \
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
            handoff_session_id: row.get(7)?,
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
// Static greetings — shown once per session before LLM responds
// ---------------------------------------------------------------------------

const PO_GREETING: &str = "Hi! I'm the Product Owner at nocodo — we help small and medium \
    businesses get custom software built around the way they actually work. My job is to \
    understand what you want to build and shape it into a clear brief for our development \
    team. I'll ask you a few focused questions to get started.";

const PM_GREETING: &str = "Hi! I'm the Project Manager at nocodo. I've received the \
    requirements brief from our Product Owner and I'll be turning that into a concrete \
    development plan. I may have one or two quick follow-up questions before I finalise \
    the work.";

// ---------------------------------------------------------------------------
// Background task: PO handles intake session
// ---------------------------------------------------------------------------

async fn notify_session(
    chat_notify: &Arc<Mutex<HashMap<i64, Arc<Notify>>>>,
    session_id: i64,
) {
    let map = chat_notify.lock().await;
    if let Some(notify) = map.get(&session_id) {
        notify.notify_waiters();
    }
}

async fn run_po_intake(
    db_path: String,
    session_id: i64,
    user_id: i64,
    chat_notify: Arc<Mutex<HashMap<i64, Arc<Notify>>>>,
) {
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

    // Detect planning sessions: seeded by PO (first msg is from product_owner).
    // These should be handled by PM, not PO.
    let is_planning_session = messages
        .first()
        .map(|m| m.author_type == "agent" && m.agent_type.as_deref() == Some("product_owner"))
        .unwrap_or(false);
    if is_planning_session {
        drop(chat_storage);
        return run_pm_planning(db_path, session_id, chat_notify).await;
    }

    // Don't fire agents while there are unanswered structured questions.
    // Agents will run once the user has answered all pending questions.
    let answered_ids: std::collections::HashSet<i64> = messages
        .iter()
        .filter(|m| m.content_type == "structured_response")
        .filter_map(|m| {
            serde_json::from_str::<serde_json::Value>(&m.content)
                .ok()
                .and_then(|v| v["question_message_id"].as_i64())
        })
        .collect();
    let has_unanswered = messages
        .iter()
        .any(|m| m.content_type == "structured_question" && !answered_ids.contains(&m.id));
    log::info!("[PO:session={}] answered_ids={:?} has_unanswered={}", session_id, answered_ids, has_unanswered);
    if has_unanswered {
        log::info!("[PO:session={}] returning early — unanswered questions present", session_id);
        return;
    }

    // If PO hasn't spoken yet, store a static greeting now so the user sees it
    // immediately. Also add it to llm_messages so the LLM knows not to re-introduce.
    let po_has_spoken = messages
        .iter()
        .any(|m| m.author_type == "agent" && m.agent_type.as_deref() == Some("product_owner"));
    if !po_has_spoken {
        if let Err(e) = chat_storage
            .append_message(
                session_id,
                "agent",
                None,
                Some(AgentType::ProductOwner),
                None,
                MessageContent::Text(PO_GREETING.to_string()),
            )
            .await
        {
            log::warn!("user_chat: store PO greeting: {}", e);
        } else {
            notify_session(&chat_notify, session_id).await;
        }
    }

    let raw_messages: Vec<(String, String)> = messages
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
    // If we just injected a greeting, include it as a prior assistant turn so the
    // LLM doesn't repeat the introduction.
    let raw_messages = if !po_has_spoken {
        let mut v = raw_messages;
        v.push(("assistant".to_string(), PO_GREETING.to_string()));
        v
    } else {
        raw_messages
    };
    // Merge consecutive same-role messages — the DB stores greeting and structured
    // questions as separate rows but the LLM requires strictly alternating turns.
    let llm_messages = merge_consecutive_roles(raw_messages);

    log::info!("[PO:session={}] sending {} messages to LLM:", session_id, llm_messages.len());
    for (i, (role, text)) in llm_messages.iter().enumerate() {
        log::info!("[PO:session={}]   [{i}] {role}: {}", session_id, &text[..text.len().min(120)]);
    }

    let config = match AgentConfig::load() {
        Ok(c) => c,
        Err(e) => {
            log::warn!("user_chat: load config: {}", e);
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
    let po_note_storage: Arc<dyn ProjectNoteStorage> =
        match SqliteProjectNoteStorage::open(&db_path) {
            Ok(s) => Arc::new(s),
            Err(e) => {
                log::warn!("user_chat: open project note storage: {}", e);
                return;
            }
        };
    let po = match ProductOwnerAgent::new(
        po_storage,
        po_task_storage,
        po_comment_storage,
        po_note_storage,
        config,
        project_id,
    ) {
        Ok(a) => a,
        Err(e) => {
            log::warn!("user_chat: build PO: {}", e);
            return;
        }
    };

    log::info!("[PO:session={}] calling respond_in_session (requirements_gathering)", session_id);
    let po_result = po.respond_in_session(session_id, llm_messages.clone(), false).await;
    log::info!("[PO:session={}] respond_in_session returned: {:?}", session_id, po_result.as_ref().map(|r| format!("{:?}", r)).unwrap_or_else(|e| format!("Err({:?})", e)));
    match po_result {
        Ok(PoSessionResult::RequirementsComplete { closing_message }) => {
            // Show the closing message to the user.
            if let Err(e) = chat_storage
                .append_message(
                    session_id,
                    "agent",
                    None,
                    Some(AgentType::ProductOwner),
                    None,
                    MessageContent::Text(closing_message),
                )
                .await
            {
                log::warn!("user_chat: store PO closing message: {}", e);
                return;
            }
            notify_session(&chat_notify, session_id).await;

            // Second call: project naming mode — PO derives a name from the conversation.
            log::info!("[PO:session={}] calling respond_in_session (project_naming)", session_id);
            let naming_result = po.respond_in_session(session_id, llm_messages, true).await;
            log::info!("[PO:session={}] project_naming returned: {:?}", session_id, naming_result.as_ref().map(|r| format!("{:?}", r)).unwrap_or_else(|e| format!("Err({:?})", e)));
            match naming_result {
                Ok(PoSessionResult::Named) | Ok(PoSessionResult::Silent) => {
                    handle_po_complete(
                        &db_path,
                        session_id,
                        project_id,
                        user_id,
                        &chat_storage,
                        &chat_notify,
                    )
                    .await;
                }
                Ok(_) => {
                    log::warn!("[PO:session={}] unexpected result from project_naming mode", session_id);
                    handle_po_complete(&db_path, session_id, project_id, user_id, &chat_storage, &chat_notify).await;
                }
                Err(e) => {
                    log::warn!("user_chat: PO project_naming error: {}", e);
                }
            }
        }
        Ok(PoSessionResult::Questions { message, questions }) => {
            if !message.trim().is_empty() {
                if let Err(e) = chat_storage
                    .append_message(
                        session_id,
                        "agent",
                        None,
                        Some(AgentType::ProductOwner),
                        None,
                        MessageContent::Text(message),
                    )
                    .await
                {
                    log::warn!("user_chat: store PO message: {}", e);
                }
            }
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
            notify_session(&chat_notify, session_id).await;
        }
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
            notify_session(&chat_notify, session_id).await;
        }
        Ok(PoSessionResult::Text(_)) | Ok(PoSessionResult::Silent) | Ok(PoSessionResult::Named) => {}
        Err(e) => {
            log::warn!("user_chat: PO error: {}", e);
        }
    }
}

async fn build_notes_seed(db_path: &str, project_id: i64) -> String {
    let note_storage = match SqliteProjectNoteStorage::open(db_path) {
        Ok(s) => s,
        Err(e) => {
            log::warn!("user_chat: open note storage for seed: {}", e);
            return "No project notes recorded.".to_string();
        }
    };
    let notes = match note_storage.list_current_notes(project_id).await {
        Ok(n) => n,
        Err(e) => {
            log::warn!("user_chat: list notes for seed: {}", e);
            return "No project notes recorded.".to_string();
        }
    };
    if notes.is_empty() {
        return "No project notes recorded yet.".to_string();
    }
    let mut out = String::from("## Requirements Brief (recorded by Product Owner)\n\n");
    let mut current_topic = String::new();
    for note in &notes {
        if note.topic != current_topic {
            current_topic = note.topic.clone();
            let heading = match current_topic.as_str() {
                "goal" => "### Goals",
                "constraint" => "### Constraints",
                "decision" => "### Decisions",
                "context" => "### Context",
                "assumption" => "### Assumptions",
                other => other,
            };
            out.push_str(&format!("{}\n", heading));
        }
        out.push_str(&format!("- **{}**: {}\n", note.title, note.note));
    }
    out
}

async fn handle_po_complete(
    db_path: &str,
    intake_session_id: i64,
    project_id: i64,
    user_id: i64,
    chat_storage: &SqliteUserChatStorage,
    chat_notify: &Arc<Mutex<HashMap<i64, Arc<Notify>>>>,
) {
    // Create the planning session for PM.
    let planning_session_id = match chat_storage.create_session(project_id, user_id).await {
        Ok(id) => id,
        Err(e) => {
            log::warn!("user_chat: create planning session: {}", e);
            return;
        }
    };

    // Build seed message from current project notes so PM has the requirements brief.
    let notes_seed = build_notes_seed(db_path, project_id).await;
    if let Err(e) = chat_storage
        .append_message(
            planning_session_id,
            "agent",
            None,
            Some(AgentType::ProductOwner),
            None,
            MessageContent::Text(notes_seed),
        )
        .await
    {
        log::warn!("user_chat: store notes seed in planning session: {}", e);
        return;
    }
    notify_session(chat_notify, planning_session_id).await;

    // Link intake → planning session and close intake.
    if let Err(e) = chat_storage
        .set_handoff_session_id(intake_session_id, planning_session_id)
        .await
    {
        log::warn!("user_chat: set handoff_session_id: {}", e);
        return;
    }
    if let Err(e) = chat_storage.complete_session(intake_session_id).await {
        log::warn!("user_chat: complete intake session: {}", e);
        return;
    }

    // Kick off PM in the planning session.
    let db_path = db_path.to_string();
    let chat_notify = chat_notify.clone();
    actix_web::rt::spawn(async move {
        run_pm_planning(db_path, planning_session_id, chat_notify).await;
    });
}

// ---------------------------------------------------------------------------
// Background task: PM handles planning session
// ---------------------------------------------------------------------------

async fn run_pm_planning(
    db_path: String,
    session_id: i64,
    chat_notify: Arc<Mutex<HashMap<i64, Arc<Notify>>>>,
) {
    let chat_storage = match SqliteUserChatStorage::open(&db_path) {
        Ok(s) => s,
        Err(e) => {
            log::warn!("user_chat: open storage (PM): {}", e);
            return;
        }
    };

    let session = match chat_storage.get_session(session_id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            log::warn!("user_chat: PM session {} not found", session_id);
            return;
        }
        Err(e) => {
            log::warn!("user_chat: get PM session: {}", e);
            return;
        }
    };
    let project_id = session.project_id;

    let messages = match chat_storage.get_messages(session_id).await {
        Ok(m) => m,
        Err(e) => {
            log::warn!("user_chat: get PM messages: {}", e);
            return;
        }
    };

    // If PM hasn't spoken yet, store a static greeting so the user sees it immediately.
    // Also append it to llm_messages so the LLM doesn't re-introduce itself.
    let pm_has_spoken = messages
        .iter()
        .any(|m| m.author_type == "agent" && m.agent_type.as_deref() == Some("project_manager"));
    if !pm_has_spoken {
        if let Err(e) = chat_storage
            .append_message(
                session_id,
                "agent",
                None,
                Some(AgentType::ProjectManager),
                None,
                MessageContent::Text(PM_GREETING.to_string()),
            )
            .await
        {
            log::warn!("user_chat: store PM greeting: {}", e);
        } else {
            notify_session(&chat_notify, session_id).await;
        }
    }

    // Load the parent intake session's messages so PM has the full requirements Q&A.
    // We look up the intake session by finding the one whose handoff_session_id points here.
    let intake_messages: Vec<UserChatMessageRow> =
        match rusqlite::Connection::open(&db_path) {
            Ok(conn) => {
                match conn.query_row(
                    "SELECT id FROM user_chat_session WHERE handoff_session_id = ?1",
                    rusqlite::params![session_id],
                    |row| row.get::<_, i64>(0),
                ) {
                    Ok(intake_id) => chat_storage.get_messages(intake_id).await.unwrap_or_default(),
                    Err(_) => vec![],
                }
            }
            Err(e) => {
                log::warn!("user_chat: open db for intake lookup: {}", e);
                vec![]
            }
        };

    let mut raw_messages: Vec<(String, String)> = Vec::new();

    // Intake conversation first (user ↔ PO), so PM sees exactly what was asked and answered.
    for m in &intake_messages {
        let role = match m.author_type.as_str() {
            "user" => "user",
            _ => "assistant",
        };
        let text = MessageContent::from_row(&m.content_type, &m.content).to_llm_text();
        if !text.trim().is_empty() {
            raw_messages.push((role.to_string(), text));
        }
    }

    // Planning session messages.
    // The seeded PO summary (first message, author_type="agent"/product_owner) is mapped as
    // "user" so PM reads it as a briefing it received, not as its own prior statement.
    for m in &messages {
        let is_po_seed = m.author_type == "agent"
            && m.agent_type.as_deref() == Some("product_owner");
        let role = if is_po_seed {
            "user"
        } else {
            match m.author_type.as_str() {
                "user" => "user",
                _ => "assistant",
            }
        };
        let text = MessageContent::from_row(&m.content_type, &m.content).to_llm_text();
        if !text.trim().is_empty() {
            raw_messages.push((role.to_string(), text));
        }
    }

    if !pm_has_spoken {
        raw_messages.push(("assistant".to_string(), PM_GREETING.to_string()));
    }

    // Merge consecutive same-role messages — the DB stores greeting and structured
    // questions as separate rows but the LLM requires strictly alternating turns.
    let llm_messages = merge_consecutive_roles(raw_messages);

    let config = match AgentConfig::load() {
        Ok(c) => c,
        Err(e) => {
            log::warn!("user_chat: load config (PM): {}", e);
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

    match pm.chat_for_user_session(session_id, llm_messages, true).await {
        Ok(PmUserSessionResult::Finalized { params }) => {
            handle_pm_finalized(&db_path, session_id, project_id, params, &chat_storage).await;
            notify_session(&chat_notify, session_id).await;
        }
        Ok(PmUserSessionResult::Questions { message, questions }) => {
            if !message.trim().is_empty() {
                if let Err(e) = chat_storage
                    .append_message(
                        session_id,
                        "agent",
                        None,
                        Some(AgentType::ProjectManager),
                        None,
                        MessageContent::Text(message),
                    )
                    .await
                {
                    log::warn!("user_chat: store PM greeting: {}", e);
                }
            }
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
            notify_session(&chat_notify, session_id).await;
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
            notify_session(&chat_notify, session_id).await;
        }
        Err(e) => {
            log::warn!("user_chat: PM error: {}", e);
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
        "INSERT INTO epic (project_id, title, description, source_prompt, created_by_agent, status, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?3, 'project_manager', 'open', ?4, ?5)",
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
    let val_po_note_storage: Arc<dyn ProjectNoteStorage> =
        match SqliteProjectNoteStorage::open(db_path) {
            Ok(s) => Arc::new(s),
            Err(e) => {
                log::warn!("user_chat: PO validate open note storage: {}", e);
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
        val_po_note_storage,
        val_config,
        project_id,
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
