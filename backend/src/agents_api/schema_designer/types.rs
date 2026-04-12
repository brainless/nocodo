use serde::{Deserialize, Serialize};

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
