use serde::{Deserialize, Serialize};

/// Used by POST /api/agents/project-manager/init — first message for a brand-new project.
#[derive(Debug, Deserialize)]
pub struct PmInitRequest {
    pub project_id: i64,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct PmChatRequest {
    pub project_id: i64,
    /// None = new task (creates task + session); Some = continue existing task
    pub task_id: Option<i64>,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct PmChatResponse {
    pub task_id: i64,
    pub message_id: i64,
    pub status: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum PmResponsePayload {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "stopped")]
    Stopped { text: String },
    #[serde(rename = "pending")]
    Pending,
}

#[derive(Debug, Serialize)]
pub struct PmMessageResponse {
    pub message_id: i64,
    pub response: PmResponsePayload,
}

#[derive(Debug, Serialize)]
pub struct PmChatHistoryMessage {
    pub id: i64,
    pub role: String,
    pub content: String,
    pub tool_name: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Serialize)]
pub struct PmChatHistoryResponse {
    pub task_id: i64,
    pub messages: Vec<PmChatHistoryMessage>,
}
