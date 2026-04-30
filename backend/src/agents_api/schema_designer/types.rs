use serde::{Deserialize, Serialize};
use shared_types::SchemaDef;

#[derive(Debug, Serialize)]
pub struct SchemaPreviewResponse {
    pub schema: SchemaDef,
    pub version: i64,
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
        schema: SchemaDef,
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
    pub schema_version: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct SchemaPreviewQuery {
    pub version: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ChatHistoryResponse {
    pub session_id: i64,
    pub messages: Vec<ChatHistoryMessage>,
}

#[derive(Debug, Deserialize)]
pub struct ListSessionsQuery {
    pub project_id: i64,
    pub agent_type: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SessionItem {
    pub id: i64,
    pub project_id: i64,
    pub agent_type: String,
    pub created_at: i64,
}

#[derive(Debug, Serialize)]
pub struct ListSessionsResponse {
    pub sessions: Vec<SessionItem>,
}
