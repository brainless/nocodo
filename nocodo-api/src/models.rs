use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct AgentExecutionRequest {
    pub user_prompt: String,
    pub db_path: String,
}

#[derive(Debug, Serialize)]
pub struct AgentExecutionResponse {
    pub session_id: i64,
    pub agent_name: String,
    pub status: String,
    pub result: String,
}

#[derive(Debug, Serialize)]
pub struct SessionMessage {
    pub role: String,
    pub content: String,
    pub created_at: i64,
}

#[derive(Debug, Serialize)]
pub struct SessionToolCall {
    pub tool_name: String,
    pub request: serde_json::Value,
    pub response: Option<serde_json::Value>,
    pub status: String,
    pub execution_time_ms: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub id: i64,
    pub agent_name: String,
    pub provider: String,
    pub model: String,
    pub system_prompt: Option<String>,
    pub user_prompt: String,
    pub status: String,
    pub result: Option<String>,
    pub messages: Vec<SessionMessage>,
    pub tool_calls: Vec<SessionToolCall>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}
