use nocodo_agents::AgentConfig;
use std::sync::Arc;
use tokio::sync::{mpsc, Notify};

use super::dispatcher::{AgentDispatcher, DispatchEvent};

/// Shared application state for agent handlers.
pub struct AgentState {
    pub config: AgentConfig,
    pub db_path: String,
    /// Send a DispatchEvent to kick off an agent for a newly created task.
    pub dispatch_tx: mpsc::UnboundedSender<DispatchEvent>,
    /// Notifies long-polling board clients when tasks or epics change.
    pub board_notify: Arc<Notify>,
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
        })
    }
}
