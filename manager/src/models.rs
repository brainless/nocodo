use chrono::Utc;
use serde::{Deserialize, Serialize};

// Re-export shared models from manager-models
pub use manager_models::{
    AddMessageRequest, AiSession, AiSessionListResponse, AiSessionOutput,
    AiSessionOutputListResponse, AiSessionResponse, AiSessionResult, ApiKeyConfig,
    CreateAiSessionRequest, CreateWorkRequest, LlmAgentToolCall, MessageAuthorType,
    MessageContentType, SettingsResponse, SupportedModel, SupportedModelsResponse,
    UpdateApiKeysRequest, Work, WorkListResponse, WorkMessage, WorkMessageListResponse,
    WorkMessageResponse, WorkResponse, WorkWithHistory,
};

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
}

/// List files tool request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListFilesRequest {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recursive: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_hidden: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_files: Option<u32>,
}

impl ListFilesRequest {
    /// Generate example JSON schema for this request type
    pub fn example_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The directory path to list files from"
                },
                "recursive": {
                    "type": "boolean",
                    "description": "Whether to list files recursively",
                    "default": false
                },
                "include_hidden": {
                    "type": "boolean",
                    "description": "Whether to include hidden files",
                    "default": false
                },
                "max_files": {
                    "type": "number",
                    "description": "Maximum number of files to return (default: 1000)",
                    "default": 1000
                }
            },
            "required": ["path"]
        })
    }
}

/// Read file tool request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadFileRequest {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_size: Option<u64>,
}

impl ReadFileRequest {
    /// Generate example JSON schema for this request type
    pub fn example_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The file path to read"
                },
                "max_size": {
                    "type": "number",
                    "description": "Maximum number of bytes to read",
                    "default": 10000
                }
            },
            "required": ["path"]
        })
    }
}

/// Write file tool request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteFileRequest {
    pub path: String,
    pub content: String,
    pub create_dirs: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub append: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_if_not_exists: Option<bool>,
}

impl WriteFileRequest {
    /// Generate example JSON schema for this request type
    pub fn example_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "The file path to write to"
                },
                "content": {
                    "type": "string",
                    "description": "The content to write to the file"
                },
                "create_dirs": {
                    "type": "boolean",
                    "description": "Whether to create parent directories if they don't exist",
                    "default": false
                },
                "append": {
                    "type": "boolean",
                    "description": "Whether to append to the file instead of overwriting",
                    "default": false
                },
                "search": {
                    "type": "string",
                    "description": "Text to search for (for search and replace operations)"
                },
                "replace": {
                    "type": "string",
                    "description": "Text to replace the search text with"
                },
                "create_if_not_exists": {
                    "type": "boolean",
                    "description": "Whether to create the file if it doesn't exist",
                    "default": false
                }
            },
            "required": ["path", "content"]
        })
    }
}

/// Grep search tool request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrepRequest {
    pub pattern: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub include_pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude_pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recursive: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub case_sensitive: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_line_numbers: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_results: Option<u32>,
}

impl GrepRequest {
    /// Generate example JSON schema for this request type
    pub fn example_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "The regex pattern to search for"
                },
                "path": {
                    "type": "string",
                    "description": "The directory path to search in",
                    "default": "."
                },
                "include_pattern": {
                    "type": "string",
                    "description": "File pattern to include in search (e.g., '*.rs')"
                },
                "exclude_pattern": {
                    "type": "string",
                    "description": "File pattern to exclude from search"
                },
                "recursive": {
                    "type": "boolean",
                    "description": "Whether to search recursively",
                    "default": true
                },
                "case_sensitive": {
                    "type": "boolean",
                    "description": "Whether the search is case sensitive",
                    "default": false
                },
                "include_line_numbers": {
                    "type": "boolean",
                    "description": "Whether to include line numbers in results",
                    "default": true
                },
                "max_results": {
                    "type": "number",
                    "description": "Maximum number of results to return",
                    "default": 100
                }
            },
            "required": ["pattern"]
        })
    }
}

/// Apply patch tool request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyPatchRequest {
    pub patch: String,
}

impl ApplyPatchRequest {
    /// Generate example JSON schema for this request type
    pub fn example_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "patch": {
                    "type": "string",
                    "description": "The patch content in the format:\n*** Begin Patch\n*** Add File: path/to/new.txt\n+line content\n*** Update File: path/to/existing.txt\n@@ optional context\n-old line\n+new line\n*** Delete File: path/to/remove.txt\n*** End Patch\n\nSupports:\n- Add File: Create new files with + prefixed lines\n- Update File: Modify files with diff hunks (- for removed, + for added)\n- Delete File: Remove files\n- Move to: Rename files (after Update File header)\n- @@ context headers for targeting specific code blocks\n\nAll file paths must be relative to the project root."
                }
            },
            "required": ["patch"]
        })
    }
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
