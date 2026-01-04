use chrono::Utc;
use serde::{Deserialize, Serialize};

// Shared models for nocodo manager and desktop-app

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub description: Option<String>,
    pub parent_id: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Project {
    pub fn new(name: String, path: String) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: 0, // Will be set by database AUTOINCREMENT
            name,
            path,
            description: None,
            parent_id: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[allow(dead_code)]
    pub fn update_timestamp(&mut self) {
        self.updated_at = Utc::now().timestamp();
    }
}

/// Component app within a project (e.g., backend API, web frontend, mobile app)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectComponent {
    pub id: i64,
    pub project_id: i64,
    pub name: String,
    /// Path relative to project root
    pub path: String,
    pub language: String,
    pub framework: Option<String>,
    pub created_at: i64,
}

impl ProjectComponent {
    pub fn new(
        project_id: i64,
        name: String,
        path: String,
        language: String,
        framework: Option<String>,
    ) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: 0, // Will be set by database AUTOINCREMENT
            project_id,
            name,
            path,
            language,
            framework,
            created_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectDetectionResult {
    pub primary_language: String,
    pub build_tools: Vec<String>,
    pub package_managers: Vec<String>,
    pub deployment_configs: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub path: Option<String>,
    pub language: Option<String>,
    pub framework: Option<String>,
    pub template: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectResponse {
    pub project: Project,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectListResponse {
    pub projects: Vec<Project>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectDetailsResponse {
    pub project: Project,
    pub components: Vec<ProjectComponent>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerStatus {
    pub status: String,
    pub version: String,
    pub uptime: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSession {
    pub id: i64,
    pub work_id: i64,
    pub message_id: i64,
    pub tool_name: String,
    pub status: String,
    pub project_context: Option<String>,
    pub started_at: i64,
    pub ended_at: Option<i64>,
}

impl AiSession {
    pub fn new(
        work_id: i64,
        message_id: i64,
        tool_name: String,
        project_context: Option<String>,
    ) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: 0, // Will be set by database AUTOINCREMENT
            work_id,
            message_id,
            tool_name,
            status: "started".to_string(),
            project_context,
            started_at: now,
            ended_at: None,
        }
    }

    pub fn complete(&mut self) {
        self.status = "completed".to_string();
        self.ended_at = Some(Utc::now().timestamp());
    }

    pub fn fail(&mut self) {
        self.status = "failed".to_string();
        self.ended_at = Some(Utc::now().timestamp());
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateAiSessionRequest {
    pub message_id: String,
    pub tool_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AiSessionResponse {
    pub session: AiSession,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AiSessionListResponse {
    pub sessions: Vec<AiSession>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSessionOutput {
    pub id: i64,
    pub session_id: i64,
    pub content: String,
    pub created_at: i64,
    /// Role of the message author: "assistant" for LLM responses, "tool" for tool responses
    pub role: Option<String>,
    /// Model name/ID used for this message (only for assistant messages)
    pub model: Option<String>,
}

/// Represents an AI session result that stores the response in a WorkMessage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSessionResult {
    pub id: i64,
    pub session_id: i64,
    pub response_message_id: i64,
    pub status: String,
    pub created_at: i64,
    pub completed_at: Option<i64>,
}

impl AiSessionResult {
    #[allow(dead_code)]
    pub fn new(session_id: i64, response_message_id: i64) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: 0, // Will be set by database AUTOINCREMENT
            session_id,
            response_message_id,
            status: "processing".to_string(),
            created_at: now,
            completed_at: None,
        }
    }

    #[allow(dead_code)]
    pub fn complete(&mut self) {
        self.status = "completed".to_string();
        self.completed_at = Some(Utc::now().timestamp());
    }

    #[allow(dead_code)]
    pub fn fail(&mut self) {
        self.status = "failed".to_string();
        self.completed_at = Some(Utc::now().timestamp());
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AiSessionOutputListResponse {
    pub outputs: Vec<AiSessionOutput>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddExistingProjectRequest {
    pub name: String,
    pub path: String, // Required - must be existing directory
    pub language: Option<String>,
    pub framework: Option<String>,
}

// Work history models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageContentType {
    #[serde(rename = "text")]
    Text,
    #[serde(rename = "markdown")]
    Markdown,
    #[serde(rename = "json")]
    Json,
    #[serde(rename = "code")]
    Code { language: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageAuthorType {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "ai")]
    Ai,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkMessage {
    pub id: i64,
    pub work_id: i64,
    pub content: String,
    pub content_type: MessageContentType,
    pub author_type: MessageAuthorType,
    pub author_id: Option<String>,
    pub sequence_order: i32,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Work {
    pub id: i64,
    pub title: String,
    pub project_id: Option<i64>,
    pub model: Option<String>, // Model ID for the work
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub git_branch: Option<String>, // Git branch for worktree support
    pub working_directory: Option<String>, // Absolute path to working directory (set at creation)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkWithHistory {
    pub work: Work,
    pub messages: Vec<WorkMessage>,
    pub total_messages: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddMessageRequest {
    pub content: String,
    pub content_type: MessageContentType,
    pub author_type: MessageAuthorType,
    pub author_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateWorkRequest {
    pub title: String,
    pub project_id: Option<i64>,
    pub model: Option<String>, // Model ID for the work (e.g., "gpt-4", "claude-3-opus-20240229")
    #[serde(default = "default_auto_start")]
    pub auto_start: bool, // Whether to automatically start AI agent session (default: true, auto-start removed)
    pub tool_name: Option<String>, // Tool to use for auto-started session (LLM agent removed)
    pub git_branch: Option<String>, // Git branch for worktree support
}

fn default_auto_start() -> bool {
    true
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkResponse {
    pub work: Work,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkListResponse {
    pub works: Vec<Work>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkMessageResponse {
    pub message: WorkMessage,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkMessageListResponse {
    pub messages: Vec<WorkMessage>,
}

/// API key configuration for the settings page
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyConfig {
    pub name: String,
    pub key: Option<String>, // Will be masked for security
    pub is_configured: bool,
}

/// Settings response containing API keys and configuration info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsResponse {
    pub config_file_path: String,
    pub api_keys: Vec<ApiKeyConfig>,
    pub projects_default_path: Option<String>,
}

/// Agent information for the agents list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
}

/// Configuration for SQLite analysis agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqliteAgentConfig {
    pub db_path: String,
}

/// Configuration for codebase analysis agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodebaseAnalysisAgentConfig {
    pub path: String,
    pub max_depth: Option<usize>,
}

/// Variant-specific agent configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AgentConfig {
    #[serde(rename = "sqlite")]
    Sqlite(SqliteAgentConfig),
    #[serde(rename = "codebase-analysis")]
    CodebaseAnalysis(CodebaseAnalysisAgentConfig),
}

/// Generic agent execution request with type-safe config
#[derive(Debug, Deserialize)]
pub struct AgentExecutionRequest {
    pub user_prompt: String,
    pub config: AgentConfig,
}

/// Response containing list of available agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentsResponse {
    pub agents: Vec<AgentInfo>,
}

/// Supported model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportedModel {
    pub provider: String,
    pub model_id: String,
    pub name: String,
    pub context_length: u32,
    pub supports_streaming: bool,
    pub supports_tool_calling: bool,
    pub supports_vision: bool,
    pub supports_reasoning: bool,
    pub input_cost_per_token: Option<f64>,
    pub output_cost_per_token: Option<f64>,
    pub default_temperature: Option<f32>,
    pub default_max_tokens: Option<u32>,
}

/// Response containing list of supported models
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupportedModelsResponse {
    pub models: Vec<SupportedModel>,
}

/// Request for updating API keys
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateApiKeysRequest {
    pub xai_api_key: Option<String>,
    pub openai_api_key: Option<String>,
    pub anthropic_api_key: Option<String>,
    pub zai_api_key: Option<String>,
    pub zai_coding_plan: Option<bool>,
}

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

/// Git branch information for worktree support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBranch {
    pub name: String,
    pub is_worktree: bool,
    pub path: Option<String>, // Path for worktree branches
}

/// Response containing list of git branches with worktree information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBranchListResponse {
    pub branches: Vec<GitBranch>,
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

// ============ Project Commands ============

/// A project command that can be executed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectCommand {
    pub id: String,
    pub project_id: i64,
    pub name: String,
    pub description: Option<String>,
    pub command: String,
    pub shell: Option<String>,
    pub working_directory: Option<String>,
    pub environment: Option<std::collections::HashMap<String, String>>,
    pub timeout_seconds: Option<u64>,
    pub os_filter: Option<Vec<String>>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// A suggested command discovered from the project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestedCommand {
    pub name: String,
    pub description: Option<String>,
    pub command: String,
    pub shell: Option<String>,
    pub working_directory: Option<String>,
    pub environment: Option<std::collections::HashMap<String, String>>,
    pub timeout_seconds: Option<u64>,
    pub os_filter: Option<Vec<String>>,
}

impl SuggestedCommand {
    pub fn to_project_command(&self, project_id: i64) -> ProjectCommand {
        let now = chrono::Utc::now().timestamp();
        ProjectCommand {
            id: uuid::Uuid::new_v4().to_string(),
            project_id,
            name: self.name.clone(),
            description: self.description.clone(),
            command: self.command.clone(),
            shell: self.shell.clone(),
            working_directory: self.working_directory.clone(),
            environment: self.environment.clone(),
            timeout_seconds: self.timeout_seconds,
            os_filter: self.os_filter.clone(),
            created_at: now,
            updated_at: now,
        }
    }
}

/// Response from command discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverCommandsResponse {
    pub commands: Vec<SuggestedCommand>,
    pub project_types: Vec<String>,
    pub reasoning: Option<String>,
}

/// Execution record for a project command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectCommandExecution {
    pub id: i64,
    pub command_id: String,
    pub git_branch: Option<String>,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    pub executed_at: i64,
    pub success: bool,
}

/// Response containing list of command executions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectCommandExecutionListResponse {
    pub executions: Vec<ProjectCommandExecution>,
}
