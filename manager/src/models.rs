use chrono::Utc;
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: String,
    pub language: Option<String>,
    pub framework: Option<String>,
    pub status: String,
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number")]
    pub updated_at: i64,
    /// Enhanced technology detection - JSON serialized list of technologies
    pub technologies: Option<String>,
}

impl Project {
    pub fn new(name: String, path: String) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: Uuid::new_v4().to_string(),
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
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProjectComponent {
    pub id: String,
    pub project_id: String,
    pub name: String,
    /// Path relative to project root
    pub path: String,
    pub language: String,
    pub framework: Option<String>,
    #[ts(type = "number")]
    pub created_at: i64,
}

impl ProjectComponent {
    pub fn new(
        project_id: String,
        name: String,
        path: String,
        language: String,
        framework: Option<String>,
    ) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: Uuid::new_v4().to_string(),
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
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProjectTechnology {
    pub language: String,
    pub framework: Option<String>,
    pub file_count: u32,
    pub confidence: f32, // 0.0 - 1.0
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProjectDetectionResult {
    pub primary_language: String,
    pub technologies: Vec<ProjectTechnology>,
    pub build_tools: Vec<String>,
    pub package_managers: Vec<String>,
    pub deployment_configs: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProjectDetailsResponse {
    pub project: Project,
    pub components: Vec<ProjectComponent>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CreateProjectRequest {
    pub name: String,
    pub path: Option<String>,
    pub language: Option<String>,
    pub framework: Option<String>,
    pub template: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProjectResponse {
    pub project: Project,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ProjectListResponse {
    pub projects: Vec<Project>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ServerStatus {
    pub status: String,
    pub version: String,
    pub uptime: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AiSession {
    pub id: String,
    pub work_id: String,
    pub message_id: String,
    pub tool_name: String,
    pub status: String,
    pub project_context: Option<String>,
    #[ts(type = "number")]
    pub started_at: i64,
    #[ts(type = "number | null")]
    pub ended_at: Option<i64>,
}

impl AiSession {
    pub fn new(
        work_id: String,
        message_id: String,
        tool_name: String,
        project_context: Option<String>,
    ) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: Uuid::new_v4().to_string(),
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

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CreateAiSessionRequest {
    pub message_id: String,
    pub tool_name: String,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AiSessionResponse {
    pub session: AiSession,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AiSessionListResponse {
    pub sessions: Vec<AiSession>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AiSessionOutput {
    #[ts(type = "number")]
    pub id: i64,
    pub session_id: String,
    pub content: String,
    #[ts(type = "number")]
    pub created_at: i64,
}

/// Represents an AI session result that stores the response in a WorkMessage
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AiSessionResult {
    pub id: String,
    pub session_id: String,
    pub response_message_id: String,
    pub status: String,
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number | null")]
    pub completed_at: Option<i64>,
}

impl AiSessionResult {
    #[allow(dead_code)]
    pub fn new(session_id: String, response_message_id: String) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: Uuid::new_v4().to_string(),
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

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AiSessionOutputListResponse {
    pub outputs: Vec<AiSessionOutput>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RecordAiOutputRequest {
    pub content: String,
}

/// Send interactive input to a running AI session (Phase 1 streaming)
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AiSessionInputRequest {
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AddExistingProjectRequest {
    pub name: String,
    pub path: String, // Required - must be existing directory
    pub language: Option<String>,
    pub framework: Option<String>,
}

// File operation models
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct FileInfo {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    #[ts(type = "number | null")]
    pub size: Option<u64>,
    #[ts(type = "number | null")]
    pub modified_at: Option<i64>,
    #[ts(type = "number | null")]
    pub created_at: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct FileListRequest {
    pub project_id: Option<String>,
    pub path: Option<String>, // Relative path within project, defaults to root
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct FileListResponse {
    pub files: Vec<FileInfo>,
    pub current_path: String,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct FileCreateRequest {
    pub project_id: String,
    pub path: String,            // Relative path within project
    pub content: Option<String>, // None for directories
    pub is_directory: bool,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct FileUpdateRequest {
    pub project_id: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct FileContentResponse {
    pub path: String,
    pub content: String,
    #[ts(type = "number | null")]
    pub modified_at: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct FileResponse {
    pub file: FileInfo,
}

// Work history models
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
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

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum MessageAuthorType {
    #[serde(rename = "user")]
    User,
    #[serde(rename = "ai")]
    Ai,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WorkMessage {
    pub id: String,
    pub work_id: String,
    pub content: String,
    pub content_type: MessageContentType,
    pub author_type: MessageAuthorType,
    pub author_id: Option<String>,
    #[ts(type = "number")]
    pub sequence_order: i32,
    #[ts(type = "number")]
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Work {
    pub id: String,
    pub title: String,
    pub project_id: Option<String>,
    pub tool_name: Option<String>,
    pub status: String,
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number")]
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WorkWithHistory {
    pub work: Work,
    pub messages: Vec<WorkMessage>,
    #[ts(type = "number")]
    pub total_messages: i32,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AddMessageRequest {
    pub content: String,
    pub content_type: MessageContentType,
    pub author_type: MessageAuthorType,
    pub author_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CreateWorkRequest {
    pub title: String,
    pub project_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WorkResponse {
    pub work: Work,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WorkListResponse {
    pub works: Vec<Work>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WorkMessageResponse {
    pub message: WorkMessage,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WorkMessageListResponse {
    pub messages: Vec<WorkMessage>,
}

// LLM Agent Types for Issue 99

/// Tool request from LLM (typed JSON)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type")]
pub enum ToolRequest {
    #[serde(rename = "list_files")]
    ListFiles(ListFilesRequest),
    #[serde(rename = "read_file")]
    ReadFile(ReadFileRequest),
}

/// List files tool request
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ListFilesRequest {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recursive: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_hidden: Option<bool>,
}

/// Read file tool request
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ReadFileRequest {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_size: Option<u64>,
}

/// Tool response to LLM (typed JSON)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type")]
pub enum ToolResponse {
    #[serde(rename = "list_files")]
    ListFiles(ListFilesResponse),
    #[serde(rename = "read_file")]
    ReadFile(ReadFileResponse),
    #[serde(rename = "error")]
    Error(ToolErrorResponse),
}

/// List files tool response
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ListFilesResponse {
    pub path: String,
    pub files: Vec<FileInfo>,
    pub total_count: u32,
}

/// Read file tool response
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ReadFileResponse {
    pub path: String,
    pub content: String,
    pub size: u64,
    pub truncated: bool,
}

/// Tool error response
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ToolErrorResponse {
    pub tool: String,
    pub error: String,
    pub message: String,
}

/// LLM provider configuration
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LlmProviderConfig {
    pub provider: String,
    pub model: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

/// LLM agent session
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LlmAgentSession {
    pub id: String,
    pub work_id: String,
    pub provider: String,
    pub model: String,
    pub status: String,
    pub system_prompt: Option<String>,
    #[ts(type = "number")]
    pub started_at: i64,
    #[ts(type = "number | null")]
    pub ended_at: Option<i64>,
}

impl LlmAgentSession {
    pub fn new(work_id: String, provider: String, model: String) -> Self {
        let now = Utc::now().timestamp();
        Self {
            id: Uuid::new_v4().to_string(),
            work_id,
            provider,
            model,
            status: "running".to_string(),
            system_prompt: None,
            started_at: now,
            ended_at: None,
        }
    }

    pub fn complete(&mut self) {
        self.status = "completed".to_string();
        self.ended_at = Some(Utc::now().timestamp());
    }

    #[allow(dead_code)]
    pub fn fail(&mut self) {
        self.status = "failed".to_string();
        self.ended_at = Some(Utc::now().timestamp());
    }
}

/// Create LLM agent session request
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CreateLlmAgentSessionRequest {
    pub provider: String,
    pub model: String,
    pub system_prompt: Option<String>,
}

/// LLM agent session response
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LlmAgentSessionResponse {
    pub session: LlmAgentSession,
}

/// LLM agent message
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct LlmAgentMessage {
    #[ts(type = "number")]
    pub id: i64,
    pub session_id: String,
    pub role: String, // "user" | "assistant" | "system"
    pub content: String,
    #[ts(type = "number")]
    pub created_at: i64,
}

/// LLM agent tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmAgentToolCall {
    pub id: i64,
    pub session_id: String,
    pub message_id: Option<i64>,
    pub tool_name: String,
    pub request: serde_json::Value,
    pub response: Option<serde_json::Value>,
    pub status: String, // "pending" | "executing" | "completed" | "failed"
    pub created_at: i64,
    pub completed_at: Option<i64>,
}

impl LlmAgentToolCall {
    pub fn new(session_id: String, tool_name: String, request: serde_json::Value) -> Self {
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
