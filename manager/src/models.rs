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
};

// User and SSH key authentication models

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub role: Option<String>,
    pub password_hash: String,
    pub is_active: bool,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_login_at: Option<i64>,
}

impl User {
    #[allow(dead_code)]
    pub fn new(name: String, email: String, password_hash: String) -> Self {
        let now = Utc::now().timestamp();
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

    #[allow(dead_code)]
    pub fn update_timestamp(&mut self) {
        self.updated_at = Utc::now().timestamp();
    }
}

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

// File operation models
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FileType {
    File,
    Directory,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileInfo {
    pub name: String,
    pub path: String,                // relative path
    pub absolute: String,            // absolute path
    pub file_type: FileType,         // enum: File, Directory
    pub ignored: bool,               // whether file is ignored by .gitignore
    pub is_directory: bool,          // computed from file_type
    pub size: Option<u64>,           // file size in bytes, None for directories
    pub modified_at: Option<String>, // ISO 8601 timestamp, None for directories
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileListRequest {
    pub project_id: Option<i64>,
    pub path: Option<String>, // Relative path within project, defaults to root
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
// WorkMessageResponse, WorkMessageListResponse) are now re-exported from manager-models

// LLM Agent Types for Issue 99

/// Tool request from LLM (typed JSON)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolRequest {
    #[serde(rename = "list_files")]
    ListFiles(ListFilesRequest),
    #[serde(rename = "read_file")]
    ReadFile(ReadFileRequest),
    #[serde(rename = "write_file")]
    WriteFile(WriteFileRequest),
    #[serde(rename = "grep")]
    Grep(GrepRequest),
    #[serde(rename = "apply_patch")]
    ApplyPatch(ApplyPatchRequest),
    #[serde(rename = "bash")]
    Bash(BashRequest),
}

/// List files and directories in a given path
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListFilesRequest {
    /// The directory path to list files from
    pub path: String,

    /// Whether to list files recursively
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recursive: Option<bool>,

    /// Whether to include hidden files (starting with .)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_hidden: Option<bool>,

    /// Maximum number of files to return (default: 1000)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_files: Option<u32>,
}



/// Read the contents of a file
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadFileRequest {
    /// The file path to read
    pub path: String,

    /// Maximum number of bytes to read (default: 10000)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_size: Option<u64>,
}



/// Write or modify a file with various operations
///
/// Supports two modes:
/// 1. Full write: Provide `path` and `content` to write/overwrite file
/// 2. Search & replace: Provide `path`, `search`, and `replace` to modify existing content
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WriteFileRequest {
    /// The file path to write to
    pub path: String,

    /// Full content to write (required for full write, omit for search-replace)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    /// Text to search for (for search-and-replace operations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,

    /// Text to replace the search text with
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replace: Option<String>,

    /// Whether to create parent directories if they don't exist
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_dirs: Option<bool>,

    /// Whether to append to the file instead of overwriting
    #[serde(skip_serializing_if = "Option::is_none")]
    pub append: Option<bool>,

    /// Whether to create the file if it doesn't exist
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_if_not_exists: Option<bool>,
}

impl WriteFileRequest {
    /// Validate that either content OR (search + replace) is provided
    pub fn validate(&self) -> anyhow::Result<()> {
        match (&self.content, &self.search, &self.replace) {
            (Some(_), None, None) => Ok(()),  // Full write mode
            (None, Some(_), Some(_)) => Ok(()),  // Search-replace mode
            (Some(_), Some(_), _) | (Some(_), _, Some(_)) => {
                anyhow::bail!("Cannot provide both 'content' and 'search'/'replace'. Use either full write OR search-replace mode.")
            }
            (None, Some(_), None) => {
                anyhow::bail!("Search-replace mode requires both 'search' and 'replace' parameters")
            }
            (None, None, Some(_)) => {
                anyhow::bail!("Search-replace mode requires both 'search' and 'replace' parameters")
            }
            (None, None, None) => {
                anyhow::bail!("Must provide either 'content' (for full write) OR both 'search' and 'replace' (for search-replace)")
            }
        }
    }


}

/// Search for patterns in files using grep
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GrepRequest {
    /// The regex pattern to search for
    pub pattern: String,

    /// The directory path to search in (default: current directory)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,

    /// File pattern to include in search (e.g., '*.rs')
    pub include_pattern: Option<String>,

    /// File pattern to exclude from search
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude_pattern: Option<String>,

    /// Whether to search recursively (default: true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recursive: Option<bool>,

    /// Whether the search is case sensitive (default: false)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub case_sensitive: Option<bool>,

    /// Whether to include line numbers in results (default: true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_line_numbers: Option<bool>,

    /// Maximum number of results to return (default: 100)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_results: Option<u32>,

    /// Maximum number of files to search through (default: 1000)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_files_searched: Option<u32>,
}



/// Apply a patch to create, modify, delete, or move multiple files
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ApplyPatchRequest {
    /// The patch content in unified diff format
    ///
    /// Format:
    /// ```
    /// *** Begin Patch
    /// *** Add File: path/to/new.txt
    /// +line content
    /// *** Update File: path/to/existing.txt
    /// @@ optional context
    /// -old line
    /// +new line
    /// *** Delete File: path/to/remove.txt
    /// *** End Patch
    /// ```
    ///
    /// Supports:
    /// - Add File: Create new files with + prefixed lines
    /// - Update File: Modify files with diff hunks (- for removed, + for added)
    /// - Delete File: Remove files
    /// - Move to: Rename files (after Update File header)
    /// - @@ context headers for targeting specific code blocks
    ///
    /// All file paths must be relative to the project root.
    pub patch: String,
}



/// Tool response to LLM (typed JSON)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolResponse {
    #[serde(rename = "list_files")]
    ListFiles(ListFilesResponse),
    #[serde(rename = "read_file")]
    ReadFile(ReadFileResponse),
    #[serde(rename = "write_file")]
    WriteFile(WriteFileResponse),
    #[serde(rename = "grep")]
    Grep(GrepResponse),
    #[serde(rename = "apply_patch")]
    ApplyPatch(ApplyPatchResponse),
    #[serde(rename = "bash")]
    Bash(BashResponse),
    #[serde(rename = "error")]
    Error(ToolErrorResponse),
}

/// List files tool response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListFilesResponse {
    pub current_path: String,
    pub files: String, // Plain text tree representation
    pub total_files: u32,
    pub truncated: bool,
    pub limit: u32,
}

/// Read file tool response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadFileResponse {
    pub path: String,
    pub content: String,
    pub size: u64,
    pub truncated: bool,
}

/// Write file tool response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteFileResponse {
    pub path: String,
    pub success: bool,
    pub bytes_written: u64,
    pub created: bool,
    pub modified: bool,
}

/// Grep match result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrepMatch {
    pub file_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_number: Option<u32>,
    pub line_content: String,
    pub match_start: u32,
    pub match_end: u32,
    pub matched_text: String,
}

/// Grep search tool response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrepResponse {
    pub pattern: String,
    pub matches: Vec<GrepMatch>,
    pub total_matches: u32,
    pub files_searched: u32,
    pub truncated: bool,
}

/// Apply patch file change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyPatchFileChange {
    pub path: String,
    pub operation: String, // "add", "update", "delete", "move"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_path: Option<String>, // For move operations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unified_diff: Option<String>, // For update operations
}

/// Apply patch tool response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyPatchResponse {
    pub success: bool,
    pub files_changed: Vec<ApplyPatchFileChange>,
    pub total_additions: usize,
    pub total_deletions: usize,
    pub message: String,
}

/// Tool error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolErrorResponse {
    pub tool: String,
    pub error: String,
    pub message: String,
}

/// LLM provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProviderConfig {
    pub provider: String,
    pub model: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

/// LLM agent session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmAgentSession {
    pub id: i64,
    pub work_id: i64,
    pub provider: String,
    pub model: String,
    pub status: String,
    pub system_prompt: Option<String>,
    pub started_at: i64,
    pub ended_at: Option<i64>,
}

impl LlmAgentSession {
    pub fn new(work_id: i64, provider: String, model: String) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: 0, // Will be set by database AUTOINCREMENT
            work_id,
            provider,
            model,
            status: "running".to_string(),
            system_prompt: None,
            started_at: now,
            ended_at: None,
        }
    }

    #[allow(dead_code)]
    pub fn fail(&mut self) {
        self.status = "failed".to_string();
        self.ended_at = Some(Utc::now().timestamp());
    }
}

/// Create LLM agent session request
#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct CreateLlmAgentSessionRequest {
    pub provider: String,
    pub model: String,
    pub system_prompt: Option<String>,
}

/// LLM agent message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmAgentMessage {
    pub id: i64,
    pub session_id: i64,
    pub role: String, // "user" | "assistant" | "system"
    pub content: String,
    pub created_at: i64,
}

// Permission system models (Phase 1: DB & Models)

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

// Additional request/response models

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct UserListResponse {
    pub users: Vec<User>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserResponse {
    pub user: User,
}

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
pub struct UpdateUserRequest {
    pub name: Option<String>,
    pub email: Option<String>,
    pub team_ids: Option<Vec<i64>>, // Update team memberships
}

// Team management models

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTeamRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTeamRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddTeamMemberRequest {
    pub user_id: i64,
}

// Permission management models

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePermissionRequest {
    pub team_id: i64,
    pub resource_type: String,
    pub resource_id: Option<i64>,
    pub action: String,
}

// Bash tool configuration models

/// Configuration for the Bash tool
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct BashToolConfig {
    pub enabled: bool,
    pub default_timeout_secs: u64,
    pub max_timeout_secs: u64,
    pub permissions: BashPermissionConfig,
    pub sandbox: BashSandboxConfig,
    pub logging: BashLoggingConfig,
}

impl Default for BashToolConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_timeout_secs: 30,
            max_timeout_secs: 300, // 5 minutes max
            permissions: BashPermissionConfig::default(),
            sandbox: BashSandboxConfig::default(),
            logging: BashLoggingConfig::default(),
        }
    }
}

/// Permission configuration for the Bash tool
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct BashPermissionConfig {
    pub default_action: String, // "allow" or "deny"
    pub allowed_working_dirs: Vec<String>,
    pub deny_changing_to_sensitive_dirs: bool,
    pub custom_rules: Vec<BashPermissionRuleConfig>,
}

impl Default for BashPermissionConfig {
    fn default() -> Self {
        Self {
            default_action: "deny".to_string(),
            allowed_working_dirs: vec![
                "/tmp".to_string(),
                "/home".to_string(),
                "/workspace".to_string(),
                "/project".to_string(),
            ],
            deny_changing_to_sensitive_dirs: true,
            custom_rules: vec![
                BashPermissionRuleConfig {
                    pattern: "echo*".to_string(),
                    action: "allow".to_string(),
                    description: Some("Allow echo commands".to_string()),
                },
                BashPermissionRuleConfig {
                    pattern: "ls*".to_string(),
                    action: "allow".to_string(),
                    description: Some("Allow listing files".to_string()),
                },
                BashPermissionRuleConfig {
                    pattern: "cat*".to_string(),
                    action: "allow".to_string(),
                    description: Some("Allow reading files".to_string()),
                },
                BashPermissionRuleConfig {
                    pattern: "pwd".to_string(),
                    action: "allow".to_string(),
                    description: Some("Allow showing current directory".to_string()),
                },
                BashPermissionRuleConfig {
                    pattern: "which*".to_string(),
                    action: "allow".to_string(),
                    description: Some("Allow finding commands".to_string()),
                },
                BashPermissionRuleConfig {
                    pattern: "git status".to_string(),
                    action: "allow".to_string(),
                    description: Some("Allow git status".to_string()),
                },
                BashPermissionRuleConfig {
                    pattern: "git log*".to_string(),
                    action: "allow".to_string(),
                    description: Some("Allow git log".to_string()),
                },
                BashPermissionRuleConfig {
                    pattern: "git diff*".to_string(),
                    action: "allow".to_string(),
                    description: Some("Allow git diff".to_string()),
                },
                BashPermissionRuleConfig {
                    pattern: "git show*".to_string(),
                    action: "allow".to_string(),
                    description: Some("Allow git show".to_string()),
                },
                BashPermissionRuleConfig {
                    pattern: "cargo check".to_string(),
                    action: "allow".to_string(),
                    description: Some("Allow cargo check".to_string()),
                },
                BashPermissionRuleConfig {
                    pattern: "cargo test".to_string(),
                    action: "allow".to_string(),
                    description: Some("Allow cargo test".to_string()),
                },
                BashPermissionRuleConfig {
                    pattern: "cargo build".to_string(),
                    action: "allow".to_string(),
                    description: Some("Allow cargo build".to_string()),
                },
                BashPermissionRuleConfig {
                    pattern: "npm test".to_string(),
                    action: "allow".to_string(),
                    description: Some("Allow npm test".to_string()),
                },
                BashPermissionRuleConfig {
                    pattern: "npm run build".to_string(),
                    action: "allow".to_string(),
                    description: Some("Allow npm build".to_string()),
                },
                BashPermissionRuleConfig {
                    pattern: "find*".to_string(),
                    action: "allow".to_string(),
                    description: Some("Allow finding files".to_string()),
                },
                BashPermissionRuleConfig {
                    pattern: "grep*".to_string(),
                    action: "allow".to_string(),
                    description: Some("Allow grep search".to_string()),
                },
                BashPermissionRuleConfig {
                    pattern: "head*".to_string(),
                    action: "allow".to_string(),
                    description: Some("Allow head command".to_string()),
                },
                BashPermissionRuleConfig {
                    pattern: "tail*".to_string(),
                    action: "allow".to_string(),
                    description: Some("Allow tail command".to_string()),
                },
                BashPermissionRuleConfig {
                    pattern: "wc*".to_string(),
                    action: "allow".to_string(),
                    description: Some("Allow word count".to_string()),
                },
                BashPermissionRuleConfig {
                    pattern: "sort*".to_string(),
                    action: "allow".to_string(),
                    description: Some("Allow sort".to_string()),
                },
                BashPermissionRuleConfig {
                    pattern: "uniq*".to_string(),
                    action: "allow".to_string(),
                    description: Some("Allow uniq".to_string()),
                },
                BashPermissionRuleConfig {
                    pattern: "rm -rf /*".to_string(),
                    action: "deny".to_string(),
                    description: Some("Prevent catastrophic deletion".to_string()),
                },
                BashPermissionRuleConfig {
                    pattern: "rm -rf /".to_string(),
                    action: "deny".to_string(),
                    description: Some("Prevent root deletion".to_string()),
                },
                BashPermissionRuleConfig {
                    pattern: "chmod 777 /*".to_string(),
                    action: "deny".to_string(),
                    description: Some("Prevent global permission changes".to_string()),
                },
                BashPermissionRuleConfig {
                    pattern: "chmod 777 /".to_string(),
                    action: "deny".to_string(),
                    description: Some("Prevent root permission changes".to_string()),
                },
                BashPermissionRuleConfig {
                    pattern: "sudo *".to_string(),
                    action: "deny".to_string(),
                    description: Some("Prevent sudo usage".to_string()),
                },
                BashPermissionRuleConfig {
                    pattern: "su *".to_string(),
                    action: "deny".to_string(),
                    description: Some("Prevent su usage".to_string()),
                },
                BashPermissionRuleConfig {
                    pattern: "passwd*".to_string(),
                    action: "deny".to_string(),
                    description: Some("Prevent password changes".to_string()),
                },
            ],
        }
    }
}

/// Individual permission rule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct BashPermissionRuleConfig {
    pub pattern: String,
    pub action: String, // "allow" or "deny"
    pub description: Option<String>,
}

/// Sandbox configuration for the Bash tool
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct BashSandboxConfig {
    pub enabled: bool,
    pub use_landlock: bool, // Phase 2: Linux sandboxing
    pub use_seccomp: bool,
    pub no_new_privileges: bool,
    pub isolate_filesystem: bool,
    pub restricted_network: bool,
}

impl Default for BashSandboxConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            use_landlock: false, // Will be enabled in Phase 2
            use_seccomp: true,
            no_new_privileges: true,
            isolate_filesystem: false, // Will be enabled in Phase 2
            restricted_network: false,
        }
    }
}

/// Logging configuration for the Bash tool
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct BashLoggingConfig {
    pub log_commands: bool,
    pub log_stdout: bool,
    pub log_stderr: bool,
    pub log_working_directory: bool,
    pub max_log_size_bytes: Option<u64>,
}

impl Default for BashLoggingConfig {
    fn default() -> Self {
        Self {
            log_commands: true,
            log_stdout: true,
            log_stderr: true,
            log_working_directory: true,
            max_log_size_bytes: Some(1024 * 1024), // 1MB max log entry
        }
    }
}

/// Execute bash commands with timeout and permission checking
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BashRequest {
    /// The bash command to execute
    pub command: String,

    /// Working directory for command execution (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<String>,

    /// Timeout in seconds (optional, uses default if not specified)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,

    /// Human-readable description of what the command does (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}



/// Bash tool response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BashResponse {
    pub command: String,
    pub working_dir: Option<String>,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub timed_out: bool,
    pub execution_time_secs: f64,
}

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
