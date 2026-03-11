use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Stable principal representation passed to authorization.
/// Keep `subject` stable across schema changes (for example, `user:<uuid>`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthContext {
    pub subject: String,
    pub role_ids: Vec<String>,
    pub attributes: HashMap<String, String>,
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
    pub fn from_user(
        user: &UserRecord,
        profile: Option<&UserProfileRecord>,
        role_ids: Vec<String>,
    ) -> Self {
        let mut attributes = HashMap::new();
        attributes.insert("status".to_string(), user.status.clone());

        if let Some(profile) = profile {
            if let Some(v) = &profile.display_name {
                attributes.insert("display_name".to_string(), v.clone());
            }
            if let Some(v) = &profile.department {
                attributes.insert("department".to_string(), v.clone());
            }
            if let Some(v) = &profile.country {
                attributes.insert("country".to_string(), v.clone());
            }
        }

        Self {
            subject: format!("user:{}", user.id),
            role_ids,
            attributes,
        }
    }
}
