use manager_models::{WriteFileRequest, WriteFileResponse, ToolErrorResponse, ToolResponse};
use crate::tool_error::ToolError;
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

pub async fn write_file(
    base_path: &PathBuf,
    request: WriteFileRequest,
) -> Result<ToolResponse> {
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
    let content_to_write = if let (Some(search), Some(replace)) =
        (&request.search, &request.replace)
    {
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
        request.content.clone().expect("content should be present for full write mode")
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

/// Validate and resolve a path relative to the base path
fn validate_and_resolve_path(base_path: &PathBuf, path: &str) -> Result<PathBuf> {
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
            Err(_) => base_path.clone(),
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
        base_path.clone()
    } else {
        base_path.join(&normalized_input)
    };

    // Canonicalize the path to resolve any remaining relative components
    let canonical_path = match target_path.canonicalize() {
        Ok(path) => path,
        Err(_) => {
            // If the file doesn't exist, try to canonicalize the parent directory
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
        Err(_) => base_path.clone(), // Fallback to non-canonical base path
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