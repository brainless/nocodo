use crate::models::{
    FileInfo, ListFilesRequest, ListFilesResponse, ReadFileRequest, ReadFileResponse,
    ToolErrorResponse, ToolRequest, ToolResponse,
};
use anyhow::Result;
use base64::Engine;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[allow(clippy::needless_borrow)]
/// Tool execution error
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("File not found: {0}")]
    #[allow(dead_code)]
    FileNotFound(String),
    #[error("Permission denied: {0}")]
    #[allow(dead_code)]
    PermissionDenied(String),
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    #[error("File too large: {0} bytes (max: {1})")]
    #[allow(dead_code)]
    FileTooLarge(u64, u64),
    #[error("IO error: {0}")]
    IoError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

impl From<std::io::Error> for ToolError {
    fn from(err: std::io::Error) -> Self {
        ToolError::IoError(err.to_string())
    }
}

impl From<serde_json::Error> for ToolError {
    fn from(err: serde_json::Error) -> Self {
        ToolError::SerializationError(err.to_string())
    }
}

/// Tool executor that handles tool requests and responses
pub struct ToolExecutor {
    base_path: PathBuf,
    max_file_size: u64,
}

impl ToolExecutor {
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            base_path,
            max_file_size: 1024 * 1024, // 1MB default
        }
    }

    #[allow(dead_code)]
    pub fn with_max_file_size(mut self, max_size: u64) -> Self {
        self.max_file_size = max_size;
        self
    }

    /// Execute a tool request and return a tool response
    pub async fn execute(&self, request: ToolRequest) -> Result<ToolResponse> {
        match request {
            ToolRequest::ListFiles(req) => self.list_files(req).await,
            ToolRequest::ReadFile(req) => self.read_file(req).await,
        }
    }

    /// List files in a directory
    #[allow(clippy::needless_borrow)]
    async fn list_files(&self, request: ListFilesRequest) -> Result<ToolResponse> {
        let target_path = self.validate_and_resolve_path(&request.path)?;

        if !target_path.exists() {
            return Ok(ToolResponse::Error(ToolErrorResponse {
                tool: "list_files".to_string(),
                error: "FileNotFound".to_string(),
                message: format!("Path does not exist: {}", request.path),
            }));
        }

        if !target_path.is_dir() {
            return Ok(ToolResponse::Error(ToolErrorResponse {
                tool: "list_files".to_string(),
                error: "InvalidPath".to_string(),
                message: format!("Path is not a directory: {}", request.path),
            }));
        }

        let recursive = request.recursive.unwrap_or(false);
        let include_hidden = request.include_hidden.unwrap_or(false);

        let mut files = Vec::new();

        if recursive {
            // Use WalkDir for recursive listing
            let walker = WalkDir::new(&target_path);

            for entry in walker {
                let entry = entry.map_err(|e| ToolError::IoError(e.to_string()))?;

                // Skip hidden files if not requested
                if !include_hidden {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    if file_name.starts_with('.') {
                        continue;
                    }
                }

                // Skip the root directory itself
                if entry.path() == target_path {
                    continue;
                }

                let file_info = self.create_file_info(&entry.path(), &target_path)?;
                files.push(file_info);
            }
        } else {
            // Non-recursive listing
            let entries = match fs::read_dir(&target_path) {
                Ok(entries) => entries,
                Err(e) => {
                    return Ok(ToolResponse::Error(ToolErrorResponse {
                        tool: "list_files".to_string(),
                        error: "PermissionDenied".to_string(),
                        message: format!("Cannot read directory {}: {}", request.path, e),
                    }));
                }
            };

            for entry in entries {
                let entry = entry.map_err(|e| ToolError::IoError(e.to_string()))?;

                // Skip hidden files if not requested
                if !include_hidden {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    if file_name.starts_with('.') {
                        continue;
                    }
                }

                let file_info = self.create_file_info(&entry.path(), &target_path)?;
                files.push(file_info);
            }
        }

        // Sort files: directories first, then by name
        files.sort_by(|a, b| {
            if a.is_directory && !b.is_directory {
                std::cmp::Ordering::Less
            } else if !a.is_directory && b.is_directory {
                std::cmp::Ordering::Greater
            } else {
                a.name.to_lowercase().cmp(&b.name.to_lowercase())
            }
        });

        let total_count = files.len() as u32;
        Ok(ToolResponse::ListFiles(ListFilesResponse {
            path: request.path,
            files,
            total_count,
        }))
    }

    /// Read file content
    async fn read_file(&self, request: ReadFileRequest) -> Result<ToolResponse> {
        let target_path = self.validate_and_resolve_path(&request.path)?;

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
        let max_size = request.max_size.unwrap_or(self.max_file_size);
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
                format!(
                    "[BINARY_FILE_BASE64] {}",
                    base64::prelude::BASE64_STANDARD.encode(&bytes)
                )
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
    fn validate_and_resolve_path(&self, path: &str) -> Result<PathBuf> {
        // Clean the path to prevent directory traversal
        let clean_path = path.trim_start_matches('.').trim_start_matches('/');
        let clean_path = clean_path.replace("..", "");

        let target_path = if clean_path.is_empty() {
            self.base_path.clone()
        } else {
            self.base_path.join(clean_path)
        };

        // Canonicalize the path to resolve any remaining relative components
        let canonical_path = match target_path.canonicalize() {
            Ok(path) => path,
            Err(_) => target_path, // Fallback to non-canonical path if it doesn't exist
        };

        // Security check: ensure the path is within the base directory
        if !canonical_path.starts_with(&self.base_path) {
            return Err(ToolError::InvalidPath(format!(
                "Path '{}' is outside the allowed directory",
                path
            ))
            .into());
        }

        Ok(canonical_path)
    }

    /// Create FileInfo from a path
    fn create_file_info(&self, path: &Path, base_path: &Path) -> Result<FileInfo> {
        let metadata = fs::metadata(path)?;

        let relative_path = path.strip_prefix(base_path).map_err(|_| {
            ToolError::InvalidPath(format!("Cannot compute relative path for {:?}", path))
        })?;

        let relative_path_str = relative_path.to_string_lossy().to_string();

        let modified_at = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64);

        let created_at = metadata
            .created()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64);

        Ok(FileInfo {
            name: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            path: relative_path_str,
            is_directory: metadata.is_dir(),
            size: if metadata.is_dir() {
                None
            } else {
                Some(metadata.len())
            },
            modified_at,
            created_at,
        })
    }

    /// Execute tool from JSON value (for LLM integration)
    #[allow(dead_code)]
    pub async fn execute_from_json(&self, json_request: Value) -> Result<Value> {
        let tool_request: ToolRequest = serde_json::from_value(json_request)?;
        let tool_response = self.execute(tool_request).await?;

        let response_value = match tool_response {
            ToolResponse::ListFiles(response) => serde_json::to_value(response)?,
            ToolResponse::ReadFile(response) => serde_json::to_value(response)?,
            ToolResponse::Error(response) => serde_json::to_value(response)?,
        };

        Ok(response_value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_tool_executor_list_files() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::new(temp_dir.path().to_path_buf());

        // Create test files
        fs::write(temp_dir.path().join("test.txt"), "Hello World").unwrap();
        fs::create_dir_all(temp_dir.path().join("subdir")).unwrap();
        fs::write(temp_dir.path().join("subdir/nested.txt"), "Nested").unwrap();

        let request = ListFilesRequest {
            path: ".".to_string(),
            recursive: Some(true),
            include_hidden: Some(false),
        };

        let response = executor
            .execute(ToolRequest::ListFiles(request))
            .await
            .unwrap();

        match response {
            ToolResponse::ListFiles(list_response) => {
                assert_eq!(list_response.files.len(), 3);
                assert!(list_response.files.iter().any(|f| f.name == "test.txt"));
                assert!(list_response.files.iter().any(|f| f.name == "subdir"));
                assert!(list_response.files.iter().any(|f| f.name == "nested.txt"));
            }
            _ => panic!("Expected ListFiles response"),
        }
    }

    #[tokio::test]
    async fn test_tool_executor_read_file() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::new(temp_dir.path().to_path_buf());

        fs::write(temp_dir.path().join("test.txt"), "Hello World").unwrap();

        let request = ReadFileRequest {
            path: "test.txt".to_string(),
            max_size: None,
        };

        let response = executor
            .execute(ToolRequest::ReadFile(request))
            .await
            .unwrap();

        match response {
            ToolResponse::ReadFile(read_response) => {
                assert_eq!(read_response.content, "Hello World");
                assert_eq!(read_response.size, 11);
            }
            _ => panic!("Expected ReadFile response"),
        }
    }
}
