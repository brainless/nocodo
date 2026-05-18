use nocodo_agents::AgentConfig;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, Notify};

use super::dispatcher::{AgentDispatcher, DispatchEvent};

/// Shared application state for agent handlers.
pub struct AgentState {
    pub config: AgentConfig,
    pub db_path: String,
    /// Send a DispatchEvent to kick off an agent for a newly created task.
    pub dispatch_tx: mpsc::UnboundedSender<DispatchEvent>,
    /// Notifies long-polling board clients when tasks or epics change.
    pub board_notify: Arc<Notify>,
    /// Per-session Notifies for user-chat long-polling.
    pub chat_notify: Arc<Mutex<HashMap<i64, Arc<Notify>>>>,
}

impl AgentState {
    pub fn new(db_path: String) -> Result<Self, String> {
        let config =
            AgentConfig::load().map_err(|e| format!("Failed to load agent config: {}", e))?;

        let board_notify = Arc::new(Notify::new());
        let (tx, rx) = mpsc::unbounded_channel::<DispatchEvent>();
        let dispatcher = AgentDispatcher::new(rx, db_path.clone(), board_notify.clone());
        tokio::spawn(dispatcher.run());

        Ok(Self {
            config,
            db_path,
            dispatch_tx: tx,
            board_notify,
            chat_notify: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Get or create a Notify for the given chat session.
    pub async fn get_session_notify(&self, session_id: i64) -> Arc<Notify> {
        let mut map = self.chat_notify.lock().await;
        map.entry(session_id)
            .or_insert_with(|| Arc::new(Notify::new()))
            .clone()
    }
}
