use serde::{Deserialize, Serialize};

/// Question and answer record for project requirements gathering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectRequirementQnA {
    pub id: i64,
    pub session_id: i64,
    pub tool_call_id: Option<i64>,
    pub question_id: String,
    pub question: String,
    pub description: Option<String>,
    pub response_type: String,
    pub answer: Option<String>,
    pub created_at: i64,
    pub answered_at: Option<i64>,
}
