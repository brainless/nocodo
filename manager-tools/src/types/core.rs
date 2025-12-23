use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
    AskUser(super::user_interaction::AskUserRequest),
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
    AskUser(super::user_interaction::AskUserResponse),
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
