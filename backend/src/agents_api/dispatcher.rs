use std::sync::Arc;
use tokio::sync::{mpsc, Notify};

use nocodo_agents::{
    build_backend_engineer, build_db_engineer, build_frontend_engineer, build_ui_designer,
    AgentConfig, AgentResponse, AgentStorage, BackendEngineerResponse, ChatMessage,
    FrontendEngineerResponse, SqliteAgentStorage, UiDesignerResponse,
};

// ---------------------------------------------------------------------------
// DispatchEvent
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct DispatchEvent {
    pub task_id: i64,
    pub project_id: i64,
    pub assigned_to_agent: String,
    pub source_prompt: String,
}

// ---------------------------------------------------------------------------
// AgentDispatcher — background task that receives events and spawns agents
// ---------------------------------------------------------------------------

pub struct AgentDispatcher {
    rx: mpsc::UnboundedReceiver<DispatchEvent>,
    db_path: String,
    board_notify: Arc<Notify>,
}

impl AgentDispatcher {
    pub fn new(
        rx: mpsc::UnboundedReceiver<DispatchEvent>,
        db_path: String,
        board_notify: Arc<Notify>,
    ) -> Self {
        Self {
            rx,
            db_path,
            board_notify,
        }
    }

    pub async fn run(mut self) {
        log::info!("[Dispatcher] Started");
        while let Some(event) = self.rx.recv().await {
            log::info!(
                "[Dispatcher] Received task_id={} agent={}",
                event.task_id,
                event.assigned_to_agent
            );
            let db_path = self.db_path.clone();
            let notify = self.board_notify.clone();
            tokio::spawn(async move {
                dispatch_task(event, db_path).await;
                notify.notify_waiters();
            });
        }
        log::warn!("[Dispatcher] Channel closed — exiting");
    }
}

// ---------------------------------------------------------------------------
// Per-task dispatch
// ---------------------------------------------------------------------------

async fn dispatch_task(event: DispatchEvent, db_path: String) {
    match event.assigned_to_agent.as_str() {
        "db_engineer" => dispatch_db_engineer(event, &db_path).await,
        "ui_designer" => dispatch_ui_designer(event, &db_path).await,
        "backend_engineer" => dispatch_backend_engineer(event, &db_path).await,
        "frontend_engineer" => dispatch_frontend_engineer(event, &db_path).await,
        other => log::warn!("[Dispatcher] No handler for agent type: {}", other),
    }
}

async fn dispatch_db_engineer(event: DispatchEvent, db_path: &str) {
    let task_id = event.task_id;

    let agent_storage = match SqliteAgentStorage::open(db_path) {
        Ok(s) => s,
        Err(e) => {
            log::error!(
                "[Dispatcher] db_engineer task={} storage error: {}",
                task_id,
                e
            );
            return;
        }
    };

    let session = match agent_storage
        .create_task_session(event.project_id, task_id, "db_engineer")
        .await
    {
        Ok(s) => s,
        Err(e) => {
            log::error!(
                "[Dispatcher] db_engineer task={} session error: {}",
                task_id,
                e
            );
            return;
        }
    };
    let session_id = session.id.unwrap_or(0);

    if let Err(e) = agent_storage
        .create_message(ChatMessage {
            id: None,
            session_id,
            role: "user".to_string(),
            agent_type: None,
            content: event.source_prompt.clone(),
            tool_call_id: None,
            tool_name: None,
            turn_id: None,
            created_at: 0,
        })
        .await
    {
        log::error!(
            "[Dispatcher] db_engineer task={} message error: {}",
            task_id,
            e
        );
        return;
    }

    let config = match AgentConfig::load_db_engineer() {
        Ok(c) => c,
        Err(e) => {
            log::error!(
                "[Dispatcher] db_engineer task={} config error: {}",
                task_id,
                e
            );
            return;
        }
    };

    let agent = match build_db_engineer(&config, db_path, event.project_id) {
        Ok(a) => a,
        Err(e) => {
            log::error!(
                "[Dispatcher] db_engineer task={} build error: {}",
                task_id,
                e
            );
            return;
        }
    };

    match agent.chat_with_session(session_id, task_id, false).await {
        Ok(AgentResponse::SchemaGenerated { text, .. }) => {
            log::info!(
                "[Dispatcher] db_engineer task={} schema generated: {}…",
                task_id,
                text.chars().take(80).collect::<String>()
            );
        }
        Ok(AgentResponse::Text(text)) => {
            log::info!(
                "[Dispatcher] db_engineer task={} text: {}…",
                task_id,
                text.chars().take(80).collect::<String>()
            );
        }
        Ok(AgentResponse::Question(q)) => {
            log::info!(
                "[Dispatcher] db_engineer task={} asked question: {}…",
                task_id,
                q.chars().take(80).collect::<String>()
            );
        }
        Ok(AgentResponse::Stopped(reason)) => {
            log::warn!(
                "[Dispatcher] db_engineer task={} stopped: {}",
                task_id,
                reason
            );
        }
        Err(e) => {
            log::error!("[Dispatcher] db_engineer task={} error: {}", task_id, e);
        }
    }
}

async fn dispatch_ui_designer(event: DispatchEvent, db_path: &str) {
    let task_id = event.task_id;

    let agent_storage = match SqliteAgentStorage::open(db_path) {
        Ok(s) => s,
        Err(e) => {
            log::error!(
                "[Dispatcher] ui_designer task={} storage error: {}",
                task_id,
                e
            );
            return;
        }
    };

    // The HTTP handler creates the session and stores the first message before
    // sending the dispatch event. For startup reconciliation (no session yet),
    // create session + store source_prompt here.
    let session_id = match agent_storage
        .get_session_by_task(task_id, "ui_designer")
        .await
    {
        Ok(Some(s)) => s.id.unwrap_or(0),
        Ok(None) => {
            let session = match agent_storage
                .create_task_session(event.project_id, task_id, "ui_designer")
                .await
            {
                Ok(s) => s,
                Err(e) => {
                    log::error!(
                        "[Dispatcher] ui_designer task={} session error: {}",
                        task_id,
                        e
                    );
                    return;
                }
            };
            let sid = session.id.unwrap_or(0);
            if let Err(e) = agent_storage
                .create_message(ChatMessage {
                    id: None,
                    session_id: sid,
                    role: "user".to_string(),
                    agent_type: None,
                    content: event.source_prompt.clone(),
                    tool_call_id: None,
                    tool_name: None,
                    turn_id: None,
                    created_at: 0,
                })
                .await
            {
                log::error!(
                    "[Dispatcher] ui_designer task={} message error: {}",
                    task_id,
                    e
                );
                return;
            }
            sid
        }
        Err(e) => {
            log::error!(
                "[Dispatcher] ui_designer task={} session lookup error: {}",
                task_id,
                e
            );
            return;
        }
    };

    let config = match AgentConfig::load_ui_designer() {
        Ok(c) => c,
        Err(e) => {
            log::error!(
                "[Dispatcher] ui_designer task={} config error: {}",
                task_id,
                e
            );
            return;
        }
    };

    let agent = match build_ui_designer(&config, db_path, event.project_id) {
        Ok(a) => a,
        Err(e) => {
            log::error!(
                "[Dispatcher] ui_designer task={} build error: {}",
                task_id,
                e
            );
            return;
        }
    };

    match agent.run_for_task(session_id, task_id).await {
        Ok(UiDesignerResponse::FormGenerated(form)) => {
            log::info!(
                "[Dispatcher] ui_designer task={} form generated for entity '{}'",
                task_id,
                form.entity
            );
        }
        Ok(UiDesignerResponse::Stopped(reason)) => {
            log::warn!(
                "[Dispatcher] ui_designer task={} stopped: {}",
                task_id,
                reason
            );
        }
        Err(e) => {
            log::error!("[Dispatcher] ui_designer task={} error: {}", task_id, e);
        }
    }
}

async fn dispatch_backend_engineer(event: DispatchEvent, db_path: &str) {
    dispatch_engineer_agent(event, db_path, "backend_engineer").await;
}

async fn dispatch_frontend_engineer(event: DispatchEvent, db_path: &str) {
    dispatch_engineer_agent(event, db_path, "frontend_engineer").await;
}

async fn dispatch_engineer_agent(event: DispatchEvent, db_path: &str, agent_type: &str) {
    let task_id = event.task_id;

    let agent_storage = match SqliteAgentStorage::open(db_path) {
        Ok(s) => s,
        Err(e) => {
            log::error!(
                "[Dispatcher] {} task={} storage error: {}",
                agent_type,
                task_id,
                e
            );
            return;
        }
    };

    // The HTTP handler may have already created a session. Check first.
    let session_id = match agent_storage.get_session_by_task(task_id, agent_type).await {
        Ok(Some(s)) => s.id.unwrap_or(0),
        Ok(None) => {
            let session = match agent_storage
                .create_task_session(event.project_id, task_id, agent_type)
                .await
            {
                Ok(s) => s,
                Err(e) => {
                    log::error!(
                        "[Dispatcher] {} task={} session error: {}",
                        agent_type,
                        task_id,
                        e
                    );
                    return;
                }
            };
            let sid = session.id.unwrap_or(0);
            if let Err(e) = agent_storage
                .create_message(ChatMessage {
                    id: None,
                    session_id: sid,
                    role: "user".to_string(),
                    agent_type: None,
                    content: event.source_prompt.clone(),
                    tool_call_id: None,
                    tool_name: None,
                    turn_id: None,
                    created_at: 0,
                })
                .await
            {
                log::error!(
                    "[Dispatcher] {} task={} message error: {}",
                    agent_type,
                    task_id,
                    e
                );
                return;
            }
            sid
        }
        Err(e) => {
            log::error!(
                "[Dispatcher] {} task={} session lookup error: {}",
                agent_type,
                task_id,
                e
            );
            return;
        }
    };

    let config = match AgentConfig::load_context_agent() {
        Ok(c) => c,
        Err(e) => {
            log::error!(
                "[Dispatcher] {} task={} config error: {}",
                agent_type,
                task_id,
                e
            );
            return;
        }
    };

    let project_path = event.source_prompt.clone();

    if agent_type == "backend_engineer" {
        let agent = match build_backend_engineer(&config, db_path, event.project_id, &project_path)
        {
            Ok(a) => a,
            Err(e) => {
                log::error!(
                    "[Dispatcher] {} task={} build error: {}",
                    agent_type,
                    task_id,
                    e
                );
                return;
            }
        };
        match agent.run_for_task(session_id, task_id).await {
            Ok(BackendEngineerResponse::ContextSaved { context }) => {
                log::info!(
                    "[Dispatcher] {} task={} context saved ({} chars)",
                    agent_type,
                    task_id,
                    context.len()
                );
            }
            Ok(BackendEngineerResponse::Stopped(reason)) => {
                log::warn!(
                    "[Dispatcher] {} task={} stopped: {}",
                    agent_type,
                    task_id,
                    reason
                );
            }
            Err(e) => {
                log::error!("[Dispatcher] {} task={} error: {}", agent_type, task_id, e);
            }
        }
    } else {
        let agent = match build_frontend_engineer(&config, db_path, event.project_id, &project_path)
        {
            Ok(a) => a,
            Err(e) => {
                log::error!(
                    "[Dispatcher] {} task={} build error: {}",
                    agent_type,
                    task_id,
                    e
                );
                return;
            }
        };
        match agent.run_for_task(session_id, task_id).await {
            Ok(FrontendEngineerResponse::ContextSaved { context }) => {
                log::info!(
                    "[Dispatcher] {} task={} context saved ({} chars)",
                    agent_type,
                    task_id,
                    context.len()
                );
            }
            Ok(FrontendEngineerResponse::Stopped(reason)) => {
                log::warn!(
                    "[Dispatcher] {} task={} stopped: {}",
                    agent_type,
                    task_id,
                    reason
                );
            }
            Err(e) => {
                log::error!("[Dispatcher] {} task={} error: {}", agent_type, task_id, e);
            }
        }
    }
}
