use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

/// In-memory storage for agent responses that are pending or completed.
pub struct ResponseStorage {
    responses: Mutex<HashMap<i64, StoredResponse>>,
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
            responses: Mutex::new(HashMap::new()),
        }
    }

    pub fn store_pending(&self, message_id: i64) {
        let mut responses = self.responses.lock().unwrap();
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

    pub fn store_text(&self, message_id: i64, text: String) {
        let mut responses = self.responses.lock().unwrap();
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

    pub fn store_schema(&self, message_id: i64, text: String, schema_json: String) {
        let mut responses = self.responses.lock().unwrap();
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

    pub fn store_stopped(&self, message_id: i64, text: String) {
        let mut responses = self.responses.lock().unwrap();
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

    pub fn get(&self, message_id: i64) -> Option<StoredResponse> {
        let responses = self.responses.lock().unwrap();
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
