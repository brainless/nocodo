use crate::models::{
    FileInfo, GrepMatch, GrepRequest, GrepResponse, ListFilesRequest, ListFilesResponse,
    ReadFileRequest, ReadFileResponse, ToolErrorResponse, ToolRequest, ToolResponse,
    WriteFileRequest, WriteFileResponse,
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

    pub fn base_path(&self) -> &PathBuf {
        &self.base_path
    }

    /// Execute a tool request and return a tool response
    pub async fn execute(&self, request: ToolRequest) -> Result<ToolResponse> {
        match request {
            ToolRequest::ListFiles(req) => self.list_files(req).await,
            ToolRequest::ReadFile(req) => self.read_file(req).await,
            ToolRequest::WriteFile(req) => self.write_file(req).await,
            ToolRequest::Grep(req) => self.grep(req).await,
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

    /// Write file content
    async fn write_file(&self, request: WriteFileRequest) -> Result<ToolResponse> {
        let target_path = self.validate_and_resolve_path(&request.path)?;

        // Create parent directories if requested
        if request.create_dirs.unwrap_or(false) {
            if let Some(parent) = target_path.parent() {
                fs::create_dir_all(parent)?;
            }
        }

        // Check if file exists for metadata
        let existed = target_path.exists();
        let mut bytes_written = 0;

        if request.append.unwrap_or(false) && existed {
            // Append to existing file
            let mut file = fs::OpenOptions::new()
                .append(true)
                .open(&target_path)?;
            use std::io::Write;
            bytes_written = file.write(request.content.as_bytes())?;
        } else {
            // Write new file or overwrite existing
            fs::write(&target_path, &request.content)?;
            bytes_written = request.content.len();
        }

        Ok(ToolResponse::WriteFile(WriteFileResponse {
            path: request.path,
            bytes_written: bytes_written as u64,
            created: !existed,
            modified: existed,
        }))
    }

    /// Search for patterns in files using grep-like functionality
    async fn grep(&self, request: GrepRequest) -> Result<ToolResponse> {
        use regex::RegexBuilder;

        let search_path = if let Some(path) = &request.path {
            self.validate_and_resolve_path(path)?
        } else {
            self.base_path.clone()
        };

        if !search_path.exists() {
            return Ok(ToolResponse::Error(ToolErrorResponse {
                tool: "grep".to_string(),
                error: "PathNotFound".to_string(),
                message: format!("Search path does not exist: {}", request.path.unwrap_or_else(|| ".".to_string())),
            }));
        }

        // Compile regex pattern
        let regex = RegexBuilder::new(&request.pattern)
            .case_insensitive(!request.case_sensitive.unwrap_or(false))
            .build()
            .map_err(|e| ToolError::InvalidPath(format!("Invalid regex pattern: {}", e)))?;

        // Compile include/exclude patterns
        let include_regex = if let Some(pattern) = &request.include_pattern {
            Some(RegexBuilder::new(pattern).build().map_err(|e| ToolError::InvalidPath(format!("Invalid include pattern: {}", e)))?)
        } else {
            None
        };

        let exclude_regex = if let Some(pattern) = &request.exclude_pattern {
            Some(RegexBuilder::new(pattern).build().map_err(|e| ToolError::InvalidPath(format!("Invalid exclude pattern: {}", e)))?)
        } else {
            None
        };

        let mut matches = Vec::new();
        let mut files_searched = 0;
        let max_results = request.max_results.unwrap_or(100);

        // Walk through files
        for entry in WalkDir::new(&search_path) {
            let entry = entry.map_err(|e| ToolError::IoError(e.to_string()))?;

            // Skip directories
            if entry.file_type().is_dir() {
                continue;
            }

            // Check include/exclude patterns
            let file_path = entry.path();
            let relative_path = file_path.strip_prefix(&search_path)
                .unwrap_or(file_path)
                .to_string_lossy();

            // Apply include filter
            if let Some(ref include_re) = include_regex {
                if !include_re.is_match(&relative_path) {
                    continue;
                }
            }

            // Apply exclude filter
            if let Some(ref exclude_re) = exclude_regex {
                if exclude_re.is_match(&relative_path) {
                    continue;
                }
            }

            files_searched += 1;

            // Search file content
            if let Ok(content) = fs::read_to_string(file_path) {
                for (line_num, line) in content.lines().enumerate() {
                    for mat in regex.find_iter(line) {
                        matches.push(GrepMatch {
                            file_path: relative_path.to_string(),
                            line_number: (line_num + 1) as u32,
                            line_content: line.to_string(),
                            match_start: mat.start() as u32,
                            match_end: mat.end() as u32,
                            matched_text: mat.as_str().to_string(),
                        });

                        // Check if we've reached the max results limit
                        if matches.len() >= max_results as usize {
                            break;
                        }
                    }

                    if matches.len() >= max_results as usize {
                        break;
                    }
                }
            }

            if matches.len() >= max_results as usize {
                break;
            }
        }

        let truncated = matches.len() >= max_results as usize;

        let total_matches = matches.len() as u32;
        Ok(ToolResponse::Grep(GrepResponse {
            pattern: request.pattern,
            matches,
            total_matches,
            files_searched,
            truncated,
        }))
    }

    /// Validate and resolve a path relative to the base path
    fn validate_and_resolve_path(&self, path: &str) -> Result<PathBuf> {
        use std::path::Path;

        let input_path = Path::new(path);

        // Handle absolute paths
        if input_path.is_absolute() {
            // If the absolute path equals our base path, allow it
            let canonical_input = match input_path.canonicalize() {
                Ok(path) => path,
                Err(_) => input_path.to_path_buf(), // Fallback if it doesn't exist yet
            };

            let canonical_base = match self.base_path.canonicalize() {
                Ok(path) => path,
                Err(_) => self.base_path.clone(),
            };

            // Security check: ensure the path is within or equals the base directory
            if canonical_input == canonical_base || canonical_input.starts_with(&canonical_base) {
                return Ok(canonical_input);
            } else {
                return Err(ToolError::InvalidPath(format!(
                    "Absolute path '{}' is outside the allowed directory '{}'",
                    path,
                    self.base_path.display()
                ))
                .into());
            }
        }

        // Handle relative paths
        // Clean the path to prevent directory traversal
        let clean_path = path.trim_start_matches("./");
        let clean_path = clean_path.replace("..", "");

        let target_path = if clean_path.is_empty() || clean_path == "." {
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
            ToolResponse::WriteFile(response) => serde_json::to_value(response)?,
            ToolResponse::Grep(response) => serde_json::to_value(response)?,
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
