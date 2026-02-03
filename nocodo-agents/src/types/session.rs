use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Option<i64>,
    pub agent_name: String,
    pub provider: String,
    pub model: String,
    pub system_prompt: Option<String>,
    pub user_prompt: String,
    pub config: serde_json::Value,
    pub status: SessionStatus,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub result: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionStatus {
    Running,
    Completed,
    Failed,
    WaitingForUserInput,
}

impl SessionStatus {
    pub fn as_str(&self) -> &str {
        match self {
            SessionStatus::Running => "running",
            SessionStatus::Completed => "completed",
            SessionStatus::Failed => "failed",
            SessionStatus::WaitingForUserInput => "waiting_for_user_input",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "running" => SessionStatus::Running,
            "completed" => SessionStatus::Completed,
            "failed" => SessionStatus::Failed,
            "waiting_for_user_input" => SessionStatus::WaitingForUserInput,
            _ => SessionStatus::Failed,
        }
    }
}
