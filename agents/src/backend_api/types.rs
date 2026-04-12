use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;

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

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub project_id: i64,
    pub session_id: Option<i64>,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub session_id: i64,
    pub message_id: i64,
    pub status: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum AgentResponsePayload {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "schema_generated")]
    SchemaGenerated {
        text: String,
        schema: serde_json::Value,
        preview: bool,
    },
    #[serde(rename = "stopped")]
    Stopped { text: String },
    #[serde(rename = "pending")]
    Pending,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub message_id: i64,
    pub response: AgentResponsePayload,
}

#[derive(Debug, Serialize)]
pub struct ChatHistoryMessage {
    pub id: i64,
    pub role: String,
    pub content: String,
    pub created_at: i64,
}

#[derive(Debug, Serialize)]
pub struct ChatHistoryResponse {
    pub session_id: i64,
    pub messages: Vec<ChatHistoryMessage>,
}
