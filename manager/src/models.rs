use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub language: Option<String>,
    pub framework: Option<String>,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
    /// Enhanced technology detection - JSON serialized list of technologies
    pub technologies: Option<String>,
}

impl Project {
    pub fn new(name: String, path: String) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: 0, // Will be set by database AUTOINCREMENT
            name,
            path,
            language: None,
            framework: None,
            status: "created".to_string(),
            created_at: now,
            updated_at: now,
            technologies: None,
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

/// Enhanced technology information for a project
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectTechnology {
    pub language: String,
    pub framework: Option<String>,
    pub file_count: u32,
    pub confidence: f32, // 0.0 - 1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectDetectionResult {
    pub primary_language: String,
    pub technologies: Vec<ProjectTechnology>,
    pub build_tools: Vec<String>,
    pub package_managers: Vec<String>,
    pub deployment_configs: Vec<String>,
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
#[allow(dead_code)]
pub struct RecordAiOutputRequest {
    pub content: String,
}

/// Send interactive input to a running AI session (Phase 1 streaming)
#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct AiSessionInputRequest {
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddExistingProjectRequest {
    pub name: String,
    pub path: String, // Required - must be existing directory
    pub language: Option<String>,
    pub framework: Option<String>,
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
    pub total_files: u32, // Total number of files found
    pub truncated: bool,      // Whether results were limited to 100
    pub limit: u32, // Maximum files returned (100)
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
    pub tool_name: Option<String>,
    pub model: Option<String>, // Model ID for the work
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
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

/// LLM agent session response
#[derive(Debug, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct LlmAgentSessionResponse {
    pub session: LlmAgentSession,
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

/// LLM agent tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmAgentToolCall {
    pub id: i64,
    pub session_id: i64,
    pub message_id: Option<i64>,
    pub tool_name: String,
    pub request: serde_json::Value,
    pub response: Option<serde_json::Value>,
    pub status: String, // "pending" | "executing" | "completed" | "failed"
    pub created_at: i64,
    pub completed_at: Option<i64>,
    pub execution_time_ms: Option<i64>,
    pub progress_updates: Option<String>, // JSON array of progress updates
    pub error_details: Option<String>,
}

impl LlmAgentToolCall {
    pub fn new(session_id: i64, tool_name: String, request: serde_json::Value) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: now, // Simple ID based on timestamp
            session_id,
            message_id: None,
            tool_name,
            request,
            response: None,
            status: "pending".to_string(),
            created_at: now,
            completed_at: None,
            execution_time_ms: None,
            progress_updates: None,
            error_details: None,
        }
    }

    pub fn complete(&mut self, response: serde_json::Value) {
        self.response = Some(response);
        self.status = "completed".to_string();
        self.completed_at = Some(Utc::now().timestamp());
    }

    pub fn fail(&mut self, error: String) {
        self.response = Some(serde_json::json!({
            "error": error
        }));
        self.status = "failed".to_string();
        self.completed_at = Some(Utc::now().timestamp());
    }
}