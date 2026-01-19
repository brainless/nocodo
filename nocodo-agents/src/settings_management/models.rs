use serde::{Deserialize, Serialize};

/// Setting record for project settings management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSetting {
    pub id: i64,
    pub session_id: i64,
    pub tool_call_id: Option<i64>,
    pub setting_key: String,
    pub setting_name: String,
    pub description: Option<String>,
    pub setting_type: String,
    pub setting_value: Option<String>,
    pub created_at: i64,
    pub updated_at: Option<i64>,
}
