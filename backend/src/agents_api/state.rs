use nocodo_agents::AgentConfig;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use super::dispatcher::{AgentDispatcher, DispatchEvent};

/// In-memory storage for agent responses that are pending or completed.
pub struct ResponseStorage {
    responses: RwLock<HashMap<i64, StoredResponse>>,
}

#[derive(Clone, Debug)]
pub struct StoredResponse {
    pub response_type: String,
    pub text: String,
    pub schema_json: Option<String>,
    pub _completed: bool,
}

impl ResponseStorage {
    pub fn new() -> Self {
        Self {
            responses: RwLock::new(HashMap::new()),
        }
    }

    pub async fn store_pending(&self, message_id: i64) {
        let mut responses = self.responses.write().await;
        responses.insert(
            message_id,
            StoredResponse {
                response_type: "pending".to_string(),
                text: String::new(),
                schema_json: None,
                _completed: false,
            },
        );
    }

    pub async fn store_text(&self, message_id: i64, text: String) {
        let mut responses = self.responses.write().await;
        responses.insert(
            message_id,
            StoredResponse {
                response_type: "text".to_string(),
                text,
                schema_json: None,
                _completed: true,
            },
        );
    }

    pub async fn store_schema(&self, message_id: i64, text: String, schema_json: String) {
        let mut responses = self.responses.write().await;
        responses.insert(
            message_id,
            StoredResponse {
                response_type: "schema_generated".to_string(),
                text,
                schema_json: Some(schema_json),
                _completed: true,
            },
        );
    }

    pub async fn store_stopped(&self, message_id: i64, text: String) {
        let mut responses = self.responses.write().await;
        responses.insert(
            message_id,
            StoredResponse {
                response_type: "stopped".to_string(),
                text,
                schema_json: None,
                _completed: true,
            },
        );
    }

    pub async fn store_question(&self, message_id: i64, text: String) {
        let mut responses = self.responses.write().await;
        responses.insert(
            message_id,
            StoredResponse {
                response_type: "question".to_string(),
                text,
                schema_json: None,
                _completed: true,
            },
        );
    }

    pub async fn get(&self, message_id: i64) -> Option<StoredResponse> {
        let responses = self.responses.read().await;
        responses.get(&message_id).cloned()
    }
}

impl Default for ResponseStorage {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared application state for agent handlers.
pub struct AgentState {
    pub config: AgentConfig,
    pub db_path: String,
    pub response_storage: Arc<ResponseStorage>,
    /// Send a DispatchEvent to kick off an agent for a newly created task.
    pub dispatch_tx: mpsc::UnboundedSender<DispatchEvent>,
}

impl AgentState {
    pub fn new(db_path: String) -> Result<Self, String> {
        let config =
            AgentConfig::load().map_err(|e| format!("Failed to load agent config: {}", e))?;

        let (tx, rx) = mpsc::unbounded_channel::<DispatchEvent>();
        let dispatcher = AgentDispatcher::new(rx, db_path.clone());
        tokio::spawn(dispatcher.run());

        Ok(Self {
            config,
            db_path,
            response_storage: Arc::new(ResponseStorage::new()),
            dispatch_tx: tx,
        })
    }

    pub fn with_config(config: AgentConfig, db_path: String) -> Self {
        let (tx, rx) = mpsc::unbounded_channel::<DispatchEvent>();
        let dispatcher = AgentDispatcher::new(rx, db_path.clone());
        tokio::spawn(dispatcher.run());

        Self {
            config,
            db_path,
            response_storage: Arc::new(ResponseStorage::new()),
            dispatch_tx: tx,
        }
    }
}
