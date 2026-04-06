use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthContext {
    pub subject: String,
    pub role_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserRecord {
    pub id: String,
    pub email: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserProfileRecord {
    pub user_id: String,
    pub display_name: Option<String>,
    pub department: Option<String>,
    pub country: Option<String>,
}

impl AuthContext {
    pub fn from_user(user: &UserRecord, role_ids: Vec<String>) -> Self {
        Self {
            subject: format!("user:{}", user.id),
            role_ids,
        }
    }
}
