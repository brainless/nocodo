use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shared_types::{AskUserRequest, AskUserResponse};

/// Tool request enum containing all possible tool operations
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type")]
pub enum ToolRequest {
    #[serde(rename = "list_files")]
    ListFiles(super::filesystem::ListFilesRequest),
    #[serde(rename = "read_file")]
    ReadFile(super::filesystem::ReadFileRequest),
    #[serde(rename = "write_file")]
    WriteFile(super::filesystem::WriteFileRequest),
    #[serde(rename = "grep")]
    Grep(super::grep::GrepRequest),
    #[serde(rename = "apply_patch")]
    ApplyPatch(super::filesystem::ApplyPatchRequest),
    #[serde(rename = "bash")]
    Bash(super::bash::BashRequest),
    #[serde(rename = "ask_user")]
    AskUser(AskUserRequest),
    #[serde(rename = "sqlite3_reader")]
    Sqlite3Reader(super::sqlite_reader::Sqlite3ReaderRequest),
    #[serde(rename = "hackernews_request")]
    HackerNewsRequest(super::hackernews::HackerNewsRequest),
}

/// Tool response enum containing all possible tool results
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolResponse {
    #[serde(rename = "list_files")]
    ListFiles(super::filesystem::ListFilesResponse),
    #[serde(rename = "read_file")]
    ReadFile(super::filesystem::ReadFileResponse),
    #[serde(rename = "write_file")]
    WriteFile(super::filesystem::WriteFileResponse),
    #[serde(rename = "grep")]
    Grep(super::grep::GrepResponse),
    #[serde(rename = "apply_patch")]
    ApplyPatch(super::filesystem::ApplyPatchResponse),
    #[serde(rename = "bash")]
    Bash(super::bash::BashResponse),
    #[serde(rename = "ask_user")]
    AskUser(AskUserResponse),
    #[serde(rename = "sqlite3_reader")]
    Sqlite3Reader(super::sqlite_reader::Sqlite3ReaderResponse),
    #[serde(rename = "hackernews_response")]
    HackerNewsResponse(super::hackernews::HackerNewsResponse),
    #[serde(rename = "error")]
    Error(ToolErrorResponse),
}

/// Error response for tool execution failures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolErrorResponse {
    pub tool: String,
    pub error: String,
    pub message: String,
}
