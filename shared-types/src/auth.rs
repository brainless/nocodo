use serde::{Deserialize, Serialize};

// ============ Authentication & User Management ============

/// Login request with username, password, and SSH fingerprint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    pub ssh_fingerprint: String, // SHA256 fingerprint from client
}

/// Login response with JWT token and user info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserInfo,
}

/// User information returned after login
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: i64,
    pub username: String,
    pub email: String,
}

/// Request to create a new user (registration)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: Option<String>,
    pub password: String,
    pub ssh_public_key: String,
    pub ssh_fingerprint: String,
}

/// Response after user creation/registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub user: User,
}

/// User model (full details, including password hash - only for internal use)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub role: Option<String>,
    #[serde(skip)] // Never send password hash to client
    pub password_hash: String,
    pub is_active: bool,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_login_at: Option<i64>,
}

impl User {
    pub fn new(name: String, email: String, password_hash: String) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: 0, // Will be set by database AUTOINCREMENT
            name,
            email,
            role: None,
            password_hash,
            is_active: true,
            created_at: now,
            updated_at: now,
            last_login_at: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct UserWithTeams {
    pub user: User,
    pub teams: Vec<Team>,
}

#[derive(Serialize, Deserialize)]
pub struct UserDetailResponse {
    pub user: User,
    pub teams: Vec<Team>,
}

#[derive(Serialize, Deserialize)]
pub struct UpdateUserRequest {
    pub name: Option<String>,
    pub email: Option<String>,
    pub team_ids: Option<Vec<i64>>, // Update team memberships
}

#[derive(Serialize, Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Team {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub created_by: i64,
}

#[derive(Serialize, Deserialize)]
pub struct TeamListResponse {
    pub teams: Vec<TeamListItem>,
}

// Clean models specifically for user list display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserListItem {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub teams: Vec<TeamItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamItem {
    pub id: i64,
    pub name: String,
}

#[derive(Serialize, Deserialize)]
pub struct UserListResponse {
    pub users: Vec<UserListItem>,
}

// Clean models specifically for team list display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamListItem {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<PermissionItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionItem {
    pub id: i64,
    pub resource_type: String,
    pub resource_id: Option<i64>,
    pub action: String,
}

// Permission model for full permission details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub id: i64,
    pub team_id: i64,
    pub resource_type: String,
    pub resource_id: Option<i64>,
    pub action: String,
    pub granted_by: Option<i64>,
    pub granted_at: i64,
}

// Request model for updating teams
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTeamRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

/// Request to add an authorized SSH key to the server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddAuthorizedSshKeyRequest {
    pub public_key: String,
}

/// Response for adding an authorized SSH key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddAuthorizedSshKeyResponse {
    pub success: bool,
    pub message: String,
}

/// Response containing user's teams (for current user)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentUserTeamsResponse {
    pub teams: Vec<TeamItem>,
}
