use crate::tool_error::ToolError;
use anyhow::Result;
use base64::{prelude::BASE64_STANDARD, Engine};
use manager_models::{ReadFileRequest, ReadFileResponse, ToolErrorResponse, ToolResponse};
use std::fs;
use std::path::{Path, PathBuf};

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

/// Validate and resolve a path relative to the base path
fn validate_and_resolve_path(base_path: &Path, path: &str) -> Result<PathBuf> {
    let input_path = Path::new(path);

    // Normalize the input path to handle . and .. components
    let normalized_input = normalize_path(input_path)?;

    // Handle absolute paths
    if normalized_input.is_absolute() {
        // If the absolute path equals our base path, allow it
        let canonical_input = match normalized_input.canonicalize() {
            Ok(path) => path,
            Err(_) => normalized_input.to_path_buf(), // Fallback if it doesn't exist yet
        };

        let canonical_base = match base_path.canonicalize() {
            Ok(path) => path,
            Err(_) => base_path.to_path_buf(),
        };

        // Security check: ensure the path is within or equals the base directory
        if canonical_input == canonical_base || canonical_input.starts_with(&canonical_base) {
            return Ok(canonical_input);
        } else {
            return Err(ToolError::InvalidPath(format!(
                "Absolute path '{}' is outside the allowed directory '{}'",
                path,
                base_path.display()
            ))
            .into());
        }
    }

    // Handle relative paths
    let target_path = if normalized_input == Path::new(".") {
        base_path.to_path_buf()
    } else {
        base_path.join(&normalized_input)
    };

    // Canonicalize the path to resolve any remaining relative components
    let canonical_path = match target_path.canonicalize() {
        Ok(path) => path,
        Err(_) => {
            // If file doesn't exist, try to canonicalize parent directory
            // and reconstruct the path to handle symlink issues on macOS
            if let Some(parent) = target_path.parent() {
                match parent.canonicalize() {
                    Ok(canonical_parent) => {
                        if let Some(filename) = target_path.file_name() {
                            canonical_parent.join(filename)
                        } else {
                            target_path
                        }
                    }
                    Err(_) => target_path,
                }
            } else {
                target_path
            }
        }
    };

    // Also canonicalize the base path for comparison (handles symlinks on macOS)
    let canonical_base = match base_path.canonicalize() {
        Ok(path) => path,
        Err(_) => base_path.to_path_buf(), // Fallback to non-canonical base path
    };

    // Security check: ensure the path is within the base directory
    if !canonical_path.starts_with(&canonical_base) {
        return Err(ToolError::InvalidPath(format!(
            "Path '{}' resolves to location outside the allowed directory",
            path
        ))
        .into());
    }

    Ok(canonical_path)
}

/// Normalize a path by resolving . and .. components while preventing directory traversal
fn normalize_path(path: &Path) -> Result<PathBuf> {
    let mut components = Vec::new();

    for component in path.components() {
        match component {
            std::path::Component::Prefix(_) | std::path::Component::RootDir => {
                // For absolute paths, keep the prefix/root
                components.push(component);
            }
            std::path::Component::CurDir => {
                // Skip current directory components
                continue;
            }
            std::path::Component::ParentDir => {
                // Prevent directory traversal attacks
                if components.is_empty()
                    || matches!(components.last(), Some(std::path::Component::ParentDir))
                {
                    return Err(ToolError::InvalidPath(format!(
                        "Invalid path '{}': contains directory traversal",
                        path.display()
                    ))
                    .into());
                }
                // Remove the last component (go up one level)
                components.pop();
            }
            std::path::Component::Normal(_name) => {
                components.push(component);
            }
        }
    }

    // Reconstruct the path from components
    let mut result = PathBuf::new();
    for component in components {
        result.push(component);
    }

    Ok(result)
}
