use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    pub id: i64,
    pub agent_name: String,
    pub provider: String,
    pub model: String,
    pub system_prompt: Option<String>,
    pub user_prompt: String,
    pub status: String,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub result: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub id: i64,
    pub session_id: i64,
    pub role: String,
    pub content: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolCall {
    pub id: i64,
    pub session_id: i64,
    pub message_id: Option<i64>,
    pub tool_call_id: String,
    pub tool_name: String,
    pub request: Value,
    pub response: Option<Value>,
    pub status: String,
    pub created_at: i64,
    pub completed_at: Option<i64>,
    pub execution_time_ms: Option<i64>,
    pub error_details: Option<String>,
}

impl AgentToolCall {
    pub fn complete(&mut self, response: Value, execution_time_ms: i64) {
        self.response = Some(response);
        self.status = "completed".to_string();
        self.completed_at = Some(chrono::Utc::now().timestamp());
        self.execution_time_ms = Some(execution_time_ms);
    }

    pub fn fail(&mut self, error: String) {
        self.status = "failed".to_string();
        self.error_details = Some(error);
        self.completed_at = Some(chrono::Utc::now().timestamp());
    }
}
