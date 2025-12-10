use chrono::Utc;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// Re-export shared models from manager-models
pub use manager_models::{
    AddMessageRequest, AiSession, AiSessionListResponse, AiSessionOutput,
    AiSessionOutputListResponse, AiSessionResponse, AiSessionResult, ApiKeyConfig,
    CreateAiSessionRequest, CreateWorkRequest, LlmAgentToolCall, LlmAgentToolCallListResponse,
    MessageAuthorType, MessageContentType, SettingsResponse, SupportedModel,
    SupportedModelsResponse, UpdateApiKeysRequest, Work, WorkListResponse, WorkMessage,
    WorkMessageListResponse, WorkMessageResponse, WorkResponse, WorkWithHistory,
    // Tool-related types
    ToolRequest, ListFilesRequest, ReadFileRequest, WriteFileRequest,
    GrepRequest, ApplyPatchRequest, BashRequest,
    FileInfo, FileType,
    // LLM Agent types
    LlmAgentSession, LlmAgentMessage, LlmProviderConfig,
    // User types from manager-models
    User, UserResponse, UpdateUserRequest, UpdateTeamRequest,
};

// User and SSH key authentication models (User is re-exported from manager-models)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSshKey {
    pub id: i64,
    pub user_id: i64,
    pub key_type: String, // "ssh-rsa", "ssh-ed25519", "ecdsa-sha2-nistp256", etc.
    pub fingerprint: String, // SHA256:base64hash
    pub public_key_data: String, // Full public key for verification
    pub label: Option<String>, // User-friendly name like "Work Laptop"
    pub is_active: bool,
    pub created_at: i64,
    pub last_used_at: Option<i64>,
}

impl UserSshKey {
    #[allow(dead_code)]
    pub fn new(
        user_id: i64,
        key_type: String,
        fingerprint: String,
        public_key_data: String,
        label: Option<String>,
    ) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: 0, // Will be set by database AUTOINCREMENT
            user_id,
            key_type,
            fingerprint,
            public_key_data,
            label,
            is_active: true,
            created_at: now,
            last_used_at: None,
        }
    }

    #[allow(dead_code)]
    pub fn mark_used(&mut self) {
        self.last_used_at = Some(Utc::now().timestamp());
    }
}

// Login request and response models

// Git-related models

#[derive(Debug, Serialize, Deserialize)]
pub struct GitBranchListResponse {
    pub branches: Vec<manager_models::GitBranch>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
    pub ssh_fingerprint: String, // SHA256 fingerprint from client
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: i64,
    pub username: String,
    pub email: String,
}

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

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectDetailsResponse {
    pub project: Project,
    pub components: Vec<ProjectComponent>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub path: Option<String>,
    pub description: Option<String>,
    pub parent_id: Option<i64>,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct ServerStatus {
    pub status: String,
    pub version: String,
    pub uptime: u64,
}

// AiSession, CreateAiSessionRequest, AiSessionResponse, AiSessionListResponse,
// AiSessionOutput, AiSessionResult, AiSessionOutputListResponse
// are now re-exported from manager-models (see top of file)

#[derive(Debug, Serialize, Deserialize)]
pub struct AddExistingProjectRequest {
    pub name: String,
    pub path: String, // Required - must be existing directory
    pub description: Option<String>,
    pub parent_id: Option<i64>,
}

// File operation models (FileType and FileInfo are re-exported from manager-models)

#[derive(Debug, Serialize, Deserialize)]
pub struct FileListRequest {
    pub project_id: Option<i64>,
    pub path: Option<String>, // Relative path within project, defaults to root
    pub git_branch: Option<String>, // Git branch/worktree to use, defaults to current branch
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileListResponse {
    pub files: Vec<FileInfo>, // List of files and directories
    pub current_path: String, // Current directory being listed
    pub total_files: u32,     // Total number of files found
    pub truncated: bool,      // Whether results were limited to 100
    pub limit: u32,           // Maximum files returned (100)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileCreateRequest {
    pub project_id: i64,
    pub path: String,            // Relative path within project
    pub content: Option<String>, // None for directories
    pub is_directory: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileUpdateRequest {
    pub project_id: i64,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileContentResponse {
    pub path: String,
    pub content: String,
    pub modified_at: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileResponse {
    pub file: FileInfo,
}

// Work history models (MessageContentType, MessageAuthorType, WorkMessage, Work,
// WorkWithHistory, AddMessageRequest, CreateWorkRequest, WorkResponse, WorkListResponse,
// WorkMessageResponse, WorkMessageListResponse) and tool-related types are now re-exported
// from manager-models


/// Bash execution log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct BashExecutionLog {
    pub id: i64,
    pub work_id: i64,
    pub user_id: i64,
    pub command: String,
    pub working_dir: Option<String>,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub timed_out: bool,
    pub execution_time_secs: f64,
    pub created_at: i64,
}

impl BashExecutionLog {
    #[allow(clippy::too_many_arguments, dead_code)]
    pub fn new(
        work_id: i64,
        user_id: i64,
        command: String,
        working_dir: Option<String>,
        stdout: String,
        stderr: String,
        exit_code: i32,
        timed_out: bool,
        execution_time_secs: f64,
    ) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: 0, // Will be set by database AUTOINCREMENT
            work_id,
            user_id,
            command,
            working_dir,
            stdout,
            stderr,
            exit_code,
            timed_out,
            execution_time_secs,
            created_at: now,
        }
    }
}

// User and Team management models (types not in manager-models)

/// Team - groups of users that share permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Team {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub created_by: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

impl Team {
    pub fn new(name: String, description: Option<String>, created_by: i64) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: 0, // Will be set by database AUTOINCREMENT
            name,
            description,
            created_by,
            created_at: now,
            updated_at: now,
        }
    }

    #[allow(dead_code)]
    pub fn update_timestamp(&mut self) {
        self.updated_at = Utc::now().timestamp();
    }
}

/// Team member - links users to teams
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMember {
    pub id: i64,
    pub team_id: i64,
    pub user_id: i64,
    pub added_by: Option<i64>,
    pub added_at: i64,
}

impl TeamMember {
    #[allow(dead_code)]
    pub fn new(team_id: i64, user_id: i64, added_by: Option<i64>) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: 0, // Will be set by database AUTOINCREMENT
            team_id,
            user_id,
            added_by,
            added_at: now,
        }
    }
}

/// Permission - access rules assigned to teams
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub id: i64,
    pub team_id: i64,
    pub resource_type: String, // "project", "work", "settings", "user", "team"
    pub resource_id: Option<i64>, // NULL = entity-level permission (all resources of this type)
    pub action: String,        // "read", "write", "delete", "admin"
    pub granted_by: Option<i64>,
    pub granted_at: i64,
}

impl Permission {
    pub fn new(
        team_id: i64,
        resource_type: String,
        resource_id: Option<i64>,
        action: String,
        granted_by: Option<i64>,
    ) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: 0, // Will be set by database AUTOINCREMENT
            team_id,
            resource_type,
            resource_id,
            action,
            granted_by,
            granted_at: now,
        }
    }
}

/// Resource ownership - tracks who created/owns resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceOwnership {
    pub id: i64,
    pub resource_type: String, // "project", "work", "settings", "user", "team"
    pub resource_id: i64,
    pub owner_id: i64,
    pub created_at: i64,
}

impl ResourceOwnership {
    pub fn new(resource_type: String, resource_id: i64, owner_id: i64) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: 0, // Will be set by database AUTOINCREMENT
            resource_type,
            resource_id,
            owner_id,
            created_at: now,
        }
    }
}

// User and Team management request models

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: Option<String>,
    pub password: String,
    #[serde(default)]
    pub ssh_public_key: Option<String>,
    #[serde(default)]
    pub ssh_fingerprint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTeamRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddTeamMemberRequest {
    pub user_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePermissionRequest {
    pub team_id: i64,
    pub resource_type: String,
    pub resource_id: Option<i64>,
    pub action: String,
}

// Project Commands Models

/// Project command that can be executed for development tasks
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[allow(dead_code)]
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

#[allow(dead_code)]
impl ProjectCommand {
    pub fn new(
        id: String,
        project_id: i64,
        name: String,
        command: String,
    ) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id,
            project_id,
            name,
            description: None,
            command,
            shell: None,
            working_directory: None,
            environment: None,
            timeout_seconds: Some(120),
            os_filter: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[allow(dead_code)]
    pub fn update_timestamp(&mut self) {
        self.updated_at = Utc::now().timestamp();
    }
}

/// Request to create a new project command
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct CreateProjectCommandRequest {
    pub name: String,
    pub description: Option<String>,
    pub command: String,
    pub shell: Option<String>,
    pub working_directory: Option<String>,
    pub environment: Option<std::collections::HashMap<String, String>>,
    pub timeout_seconds: Option<u64>,
    pub os_filter: Option<Vec<String>>,
}

/// Request to update a project command
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct UpdateProjectCommandRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub command: Option<String>,
    pub shell: Option<String>,
    pub working_directory: Option<String>,
    pub environment: Option<std::collections::HashMap<String, String>>,
    pub timeout_seconds: Option<u64>,
    pub os_filter: Option<Vec<String>>,
}

/// Request to execute a command
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[allow(dead_code)]
pub struct ExecuteProjectCommandRequest {
    pub git_branch: Option<String>,
    pub environment: Option<std::collections::HashMap<String, String>>,
    pub timeout_seconds: Option<u64>,
}

/// Execution result for a project command
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[allow(dead_code)]
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

#[allow(dead_code)]
impl ProjectCommandExecution {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        command_id: String,
        git_branch: Option<String>,
        exit_code: Option<i32>,
        stdout: String,
        stderr: String,
        duration_ms: u64,
        success: bool,
    ) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: 0, // Will be set by database AUTOINCREMENT
            command_id,
            git_branch,
            exit_code,
            stdout,
            stderr,
            duration_ms,
            executed_at: now,
            success,
        }
    }
}

/// Response for project command list
#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ProjectCommandListResponse {
    pub commands: Vec<ProjectCommand>,
}

/// Response for single project command
#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ProjectCommandResponse {
    pub command: ProjectCommand,
}

/// Response for project command execution
#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ProjectCommandExecutionResponse {
    pub execution: ProjectCommandExecution,
}

/// Response for project command execution list
#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ProjectCommandExecutionListResponse {
    pub executions: Vec<ProjectCommandExecution>,
}

/// Query parameters for filtering project commands
#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ProjectCommandFilterQuery {
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// Query parameters for command discovery
#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct DiscoveryOptionsQuery {
    /// Whether to use LLM for enhanced discovery (default: true)
    pub use_llm: Option<bool>,
    /// LLM provider to use (default: from config)
    pub llm_provider: Option<String>,
    /// LLM model to use (default: from config)
    pub llm_model: Option<String>,
}
