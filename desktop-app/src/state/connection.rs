use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

impl Default for ConnectionState {
    fn default() -> Self {
        ConnectionState::Disconnected
    }
}

/// Authentication state for tracking user login status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

impl Default for AuthState {
    fn default() -> Self {
        Self {
            jwt_token: None,
            user_id: None,
            username: None,
            is_first_user: false,
        }
    }
}
