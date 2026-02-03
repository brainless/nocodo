use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: Option<i64>,
    pub session_id: i64,
    pub message_id: Option<i64>,
    pub tool_call_id: String,
    pub tool_name: String,
    pub request: serde_json::Value,
    pub response: Option<serde_json::Value>,
    pub status: ToolCallStatus,
    pub execution_time_ms: Option<i64>,
    pub created_at: i64,
    pub completed_at: Option<i64>,
    pub error_details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ToolCallStatus {
    Pending,
    Executing,
    Completed,
    Failed,
}

impl ToolCallStatus {
    pub fn as_str(&self) -> &str {
        match self {
            ToolCallStatus::Pending => "pending",
            ToolCallStatus::Executing => "executing",
            ToolCallStatus::Completed => "completed",
            ToolCallStatus::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "pending" => ToolCallStatus::Pending,
            "executing" => ToolCallStatus::Executing,
            "completed" => ToolCallStatus::Completed,
            "failed" => ToolCallStatus::Failed,
            _ => ToolCallStatus::Failed,
        }
    }
}

impl ToolCall {
    pub fn complete(&mut self, response: serde_json::Value, execution_time_ms: i64) {
        self.response = Some(response);
        self.status = ToolCallStatus::Completed;
        self.completed_at = Some(chrono::Utc::now().timestamp());
        self.execution_time_ms = Some(execution_time_ms);
    }

    pub fn fail(&mut self, error: String) {
        self.status = ToolCallStatus::Failed;
        self.error_details = Some(error);
        self.completed_at = Some(chrono::Utc::now().timestamp());
    }
}
