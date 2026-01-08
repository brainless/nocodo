use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AgentExecutionResponse {
    pub session_id: i64,
    pub agent_name: String,
    pub status: String,
    pub result: String,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SessionMessage {
    pub role: String,
    pub content: String,
    pub created_at: i64,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SessionToolCall {
    pub tool_name: String,
    #[ts(type = "any")]
    pub request: serde_json::Value,
    #[ts(type = "any")]
    pub response: Option<serde_json::Value>,
    pub status: String,
    pub execution_time_ms: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SessionResponse {
    pub id: i64,
    pub agent_name: String,
    pub provider: String,
    pub model: String,
    pub system_prompt: Option<String>,
    pub user_prompt: String,
    #[ts(type = "any")]
    pub config: Option<serde_json::Value>,
    pub status: String,
    pub result: Option<String>,
    pub messages: Vec<SessionMessage>,
    pub tool_calls: Vec<SessionToolCall>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SessionListItem {
    pub id: i64,
    pub agent_name: String,
    pub user_prompt: String,
    pub started_at: i64,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SessionListResponse {
    pub sessions: Vec<SessionListItem>,
}
