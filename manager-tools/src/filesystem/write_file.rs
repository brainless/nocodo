use super::path_utils::validate_and_resolve_path;
use anyhow::Result;
use manager_models::{ToolErrorResponse, ToolResponse, WriteFileRequest, WriteFileResponse};
use std::fs;
use std::path::Path;

pub async fn write_file(base_path: &Path, request: WriteFileRequest) -> Result<ToolResponse> {
    // Validate request parameters
    if let Err(e) = request.validate() {
        return Ok(ToolResponse::Error(ToolErrorResponse {
            tool: "write_file".to_string(),
            error: "ValidationError".to_string(),
            message: e.to_string(),
        }));
    }

    let target_path = validate_and_resolve_path(base_path, &request.path)?;

    // Check if file exists for metadata
    let file_exists = target_path.exists();

    // Create parent directories if requested
    if request.create_dirs.unwrap_or(false) || request.create_if_not_exists.unwrap_or(false) {
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }
    }

    // Handle create_if_not_exists flag
    if !file_exists
        && !request.create_if_not_exists.unwrap_or(false)
        && !request.create_dirs.unwrap_or(false)
    {
        return Ok(ToolResponse::Error(ToolErrorResponse {
            tool: "write_file".to_string(),
            error: "FileNotFound".to_string(),
            message: format!(
                "File does not exist: {} (use create_if_not_exists=true to create it)",
                request.path
            ),
        }));
    }

    // Handle search and replace functionality
    let content_to_write =
        if let (Some(search), Some(replace)) = (&request.search, &request.replace) {
            // Read existing content for search/replace
            let existing_content = match fs::read_to_string(&target_path) {
                Ok(content) => content,
                Err(_) => {
                    return Ok(ToolResponse::Error(ToolErrorResponse {
                        tool: "write_file".to_string(),
                        error: "ReadError".to_string(),
                        message: format!("Cannot read file for search/replace: {}", request.path),
                    }));
                }
            };

            // Perform search and replace
            if existing_content.contains(search) {
                existing_content.replace(search, replace)
            } else {
                return Ok(ToolResponse::Error(ToolErrorResponse {
                    tool: "write_file".to_string(),
                    error: "SearchNotFound".to_string(),
                    message: format!(
                        "Search pattern '{}' not found in file: {}",
                        search, request.path
                    ),
                }));
            }
        } else {
            // Full write mode - content must be present (validated earlier)
            request
                .content
                .clone()
                .expect("content should be present for full write mode")
        };

    let bytes_written = if request.append.unwrap_or(false) && file_exists {
        // Append to existing file
        let mut file = fs::OpenOptions::new().append(true).open(&target_path)?;
        use std::io::Write;
        file.write(content_to_write.as_bytes())?
    } else {
        // Write new file or overwrite existing
        fs::write(&target_path, &content_to_write)?;
        content_to_write.len()
    };

    Ok(ToolResponse::WriteFile(WriteFileResponse {
        path: request.path,
        success: true,
        bytes_written: bytes_written as u64,
        created: !file_exists,
        modified: file_exists,
    }))
}
