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
    #[cfg(feature = "sqlite")]
    #[serde(rename = "sqlite3_reader")]
    Sqlite3Reader(super::sqlite_reader::Sqlite3ReaderRequest),
    #[cfg(feature = "sqlite")]
    #[serde(rename = "hackernews_request")]
    HackerNewsRequest(super::hackernews::HackerNewsRequest),
    #[serde(rename = "imap_reader")]
    ImapReader(super::imap::ImapReaderRequest),
    #[serde(rename = "pdftotext")]
    PdfToText(super::pdftotext::PdfToTextRequest),
    #[serde(rename = "confirm_extraction")]
    ConfirmExtraction(super::pdftotext::ConfirmExtractionRequest),
    #[cfg(feature = "postgres")]
    #[serde(rename = "postgres_reader")]
    PostgresReader(super::postgres_reader::PostgresReaderRequest),
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
    #[cfg(feature = "sqlite")]
    #[serde(rename = "sqlite3_reader")]
    Sqlite3Reader(super::sqlite_reader::Sqlite3ReaderResponse),
    #[cfg(feature = "sqlite")]
    #[serde(rename = "hackernews_response")]
    HackerNewsResponse(super::hackernews::HackerNewsResponse),
    #[serde(rename = "imap_reader")]
    ImapReader(super::imap::ImapReaderResponse),
    #[serde(rename = "pdftotext")]
    PdfToText(super::pdftotext::PdfToTextResponse),
    #[serde(rename = "confirm_extraction")]
    ConfirmExtraction(super::pdftotext::ConfirmExtractionResponse),
    #[cfg(feature = "postgres")]
    #[serde(rename = "postgres_reader")]
    PostgresReader(super::postgres_reader::PostgresReaderResponse),
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
