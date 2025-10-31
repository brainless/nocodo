use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum ConnectionState {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

/// Authentication state for tracking user login status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AuthState {
    /// JWT token received from manager
    pub jwt_token: Option<String>,
    /// User ID
    pub user_id: Option<i64>,
    /// Username
    pub username: Option<String>,
    /// Whether this is the first user (for UI messaging)
    pub is_first_user: bool,
}
