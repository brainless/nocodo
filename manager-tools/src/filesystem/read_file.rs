use super::path_utils::validate_and_resolve_path;
use crate::types::{ReadFileRequest, ReadFileResponse, ToolErrorResponse, ToolResponse};
use anyhow::Result;
use base64::{prelude::BASE64_STANDARD, Engine};
use std::fs;
use std::path::Path;

pub async fn read_file(
    base_path: &Path,
    max_file_size: u64,
    request: ReadFileRequest,
) -> Result<ToolResponse> {
    let target_path = validate_and_resolve_path(base_path, &request.path)?;

    if !target_path.exists() {
        return Ok(ToolResponse::Error(ToolErrorResponse {
            tool: "read_file".to_string(),
            error: "FileNotFound".to_string(),
            message: format!("File does not exist: {}", request.path),
        }));
    }

    if target_path.is_dir() {
        return Ok(ToolResponse::Error(ToolErrorResponse {
            tool: "read_file".to_string(),
            error: "InvalidPath".to_string(),
            message: format!("Path is a directory, not a file: {}", request.path),
        }));
    }

    let metadata = fs::metadata(&target_path)?;
    let file_size = metadata.len();

    // Check file size limit
    let max_size = request.max_size.unwrap_or(max_file_size);
    if file_size > max_size {
        return Ok(ToolResponse::Error(ToolErrorResponse {
            tool: "read_file".to_string(),
            error: "FileTooLarge".to_string(),
            message: format!("File is too large: {} bytes (max: {})", file_size, max_size),
        }));
    }

    // Read file content
    let content = match fs::read_to_string(&target_path) {
        Ok(content) => content,
        Err(_) => {
            // If it's not UTF-8, read as binary and encode as base64
            let bytes = fs::read(&target_path)?;
            format!("[BINARY_FILE_BASE64] {}", BASE64_STANDARD.encode(&bytes))
        }
    };

    Ok(ToolResponse::ReadFile(ReadFileResponse {
        path: request.path,
        content,
        size: file_size,
        truncated: false,
    }))
}
