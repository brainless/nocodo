use crate::models::{
    FileInfo, FileType, GrepMatch, GrepRequest, GrepResponse, ListFilesRequest, ListFilesResponse,
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
            ToolRequest::Grep(req) => self.grep_search(req).await,
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
        let max_files = request.max_files.unwrap_or(100) as usize;

        // Collect all files with breadth-first traversal
        let mut all_files = Vec::new();
        let mut queue = vec![target_path.clone()];
        let mut visited = std::collections::HashSet::new();

        while !queue.is_empty() && all_files.len() < max_files {
            let current_dir = queue.remove(0);

            if visited.contains(&current_dir) {
                continue;
            }
            visited.insert(current_dir.clone());

            let entries = match fs::read_dir(&current_dir) {
                Ok(entries) => entries,
                Err(_) => continue, // Skip directories we can't read
            };

            let mut subdirs = Vec::new();

            for entry in entries {
                if all_files.len() >= max_files {
                    break;
                }

                let entry = match entry {
                    Ok(entry) => entry,
                    Err(_) => continue,
                };

                let path = entry.path();

                // Skip hidden files/directories if not requested
                if !include_hidden {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    if file_name.starts_with('.') {
                        continue;
                    }
                }

                let file_info = match self.create_file_info(&path, &target_path) {
                    Ok(info) => info,
                    Err(_) => continue,
                };

                if matches!(file_info.file_type, FileType::Directory) {
                    subdirs.push(path);
                }

                all_files.push(file_info);
            }

            // Add subdirectories to queue for breadth-first traversal
            if recursive {
                queue.extend(subdirs);
            }
        }

        // Sort files: directories first, then by name (case-insensitive)
        all_files.sort_by(|a, b| {
            match (&a.file_type, &b.file_type) {
                (FileType::Directory, FileType::File) => std::cmp::Ordering::Less,
                (FileType::File, FileType::Directory) => std::cmp::Ordering::Greater,
                _ => {
                    // Both are same type, sort by name case-insensitively
                    a.name
                        .to_lowercase()
                        .cmp(&b.name.to_lowercase())
                        .then_with(|| a.name.cmp(&b.name)) // Stable sort for same lowercase names
                }
            }
        });

        // Generate tree representation
        let tree_output = self.format_as_tree(&all_files, &target_path);

        let total_files = all_files.len() as u32;
        let truncated = all_files.len() >= max_files;

        Ok(ToolResponse::ListFiles(ListFilesResponse {
            current_path: request.path,
            files: tree_output,
            total_files,
            truncated,
            limit: max_files as u32,
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
            request.content
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

    /// Search files using grep
    async fn grep_search(&self, request: GrepRequest) -> Result<ToolResponse> {
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
                message: format!(
                    "Search path does not exist: {}",
                    request.path.unwrap_or_else(|| ".".to_string())
                ),
            }));
        }

        // Compile regex pattern
        let regex = RegexBuilder::new(&request.pattern)
            .case_insensitive(!request.case_sensitive.unwrap_or(false))
            .build()
            .map_err(|e| ToolError::InvalidPath(format!("Invalid regex pattern: {}", e)))?;

        // Compile include/exclude patterns
        let include_regex =
            if let Some(pattern) = &request.include_pattern {
                Some(RegexBuilder::new(pattern).build().map_err(|e| {
                    ToolError::InvalidPath(format!("Invalid include pattern: {}", e))
                })?)
            } else {
                None
            };

        let exclude_regex =
            if let Some(pattern) = &request.exclude_pattern {
                Some(RegexBuilder::new(pattern).build().map_err(|e| {
                    ToolError::InvalidPath(format!("Invalid exclude pattern: {}", e))
                })?)
            } else {
                None
            };

        let mut matches = Vec::new();
        let mut files_searched = 0;
        let max_results = request.max_results.unwrap_or(100) as usize;

        // Use walkdir for recursive search if requested
        let recursive = request.recursive.unwrap_or(true);
        let walker = if recursive {
            WalkDir::new(&search_path)
        } else {
            WalkDir::new(&search_path).max_depth(1)
        };

        for entry in walker {
            let entry = entry.map_err(|e| ToolError::IoError(e.to_string()))?;

            // Skip directories
            if entry.file_type().is_dir() {
                continue;
            }

            // Check include/exclude patterns
            let file_path = entry.path();
            let relative_path = file_path
                .strip_prefix(&search_path)
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

            // Skip files that don't match common patterns (like .gitignore)
            let file_name = entry.file_name().to_string_lossy();
            if file_name.starts_with('.') || file_name == "target" || file_name == "node_modules" {
                continue;
            }

            files_searched += 1;

            // Search file content
            let content = match fs::read_to_string(file_path) {
                Ok(content) => content,
                Err(_) => continue, // Skip files we can't read
            };

            // Search for pattern in content
            for (line_num, line) in content.lines().enumerate() {
                if matches.len() >= max_results {
                    break;
                }

                // Find all matches in this line
                for mat in regex.find_iter(line) {
                    if matches.len() >= max_results {
                        break;
                    }

                    let matched_text = mat.as_str().to_string();

                    let grep_match = GrepMatch {
                        file_path: relative_path.to_string(),
                        line_number: if request.include_line_numbers.unwrap_or(true) {
                            Some((line_num + 1) as u32)
                        } else {
                            None
                        },
                        line_content: line.to_string(),
                        match_start: mat.start() as u32,
                        match_end: mat.end() as u32,
                        matched_text,
                    };

                    matches.push(grep_match);
                }
            }

            // Stop if we've reached the max results limit
            if matches.len() >= max_results {
                break;
            }
        }

        let total_matches = matches.len() as u32;
        let truncated = matches.len() >= max_results;
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

        // Normalize the input path to handle . and .. components
        let normalized_input = self.normalize_path(input_path)?;

        // Handle absolute paths
        if normalized_input.is_absolute() {
            // If the absolute path equals our base path, allow it
            let canonical_input = match normalized_input.canonicalize() {
                Ok(path) => path,
                Err(_) => normalized_input.to_path_buf(), // Fallback if it doesn't exist yet
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
        let target_path = if normalized_input == Path::new(".") {
            self.base_path.clone()
        } else {
            self.base_path.join(&normalized_input)
        };

        // Canonicalize the path to resolve any remaining relative components
        let canonical_path = match target_path.canonicalize() {
            Ok(path) => path,
            Err(_) => target_path, // Fallback to non-canonical path if it doesn't exist
        };

        // Security check: ensure the path is within the base directory
        if !canonical_path.starts_with(&self.base_path) {
            return Err(ToolError::InvalidPath(format!(
                "Path '{}' resolves to location outside the allowed directory",
                path
            ))
            .into());
        }

        Ok(canonical_path)
    }

    /// Normalize a path by resolving . and .. components while preventing directory traversal
    fn normalize_path(&self, path: &Path) -> Result<PathBuf> {
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

    /// Create FileInfo from a path
    fn create_file_info(&self, path: &Path, base_path: &Path) -> Result<FileInfo> {
        let metadata = fs::metadata(path)?;

        let relative_path = path.strip_prefix(base_path).map_err(|_| {
            ToolError::InvalidPath(format!("Cannot compute relative path for {:?}", path))
        })?;

        let relative_path_str = relative_path.to_string_lossy().to_string();
        let absolute_path_str = path.to_string_lossy().to_string();

        // Check if file is ignored by .gitignore
        let ignored = self.is_ignored_by_gitignore(path)?;

        Ok(FileInfo {
            name: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            path: relative_path_str,
            absolute: absolute_path_str,
            file_type: if metadata.is_dir() {
                FileType::Directory
            } else {
                FileType::File
            },
            ignored,
        })
    }

    /// Format files as a tree structure
    fn format_as_tree(&self, files: &[FileInfo], base_path: &Path) -> String {
        let mut output = String::new();

        // Add root directory name
        let root_name = base_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        output.push_str(&root_name);
        output.push('\n');

        // Group files by their directory depth and parent
        let mut file_tree: std::collections::BTreeMap<String, Vec<&FileInfo>> =
            std::collections::BTreeMap::new();

        for file in files.iter() {
            let path_parts: Vec<&str> = file.path.split('/').collect();
            let depth = path_parts.len().saturating_sub(1);

            // Create a key for the parent directory at this depth
            let parent_key = if depth == 0 {
                "".to_string()
            } else {
                path_parts[..depth].join("/")
            };

            file_tree
                .entry(parent_key)
                .or_default()
                .push(file);
        }

        // Recursive function to build tree
        fn build_tree_level(
            output: &mut String,
            tree: &std::collections::BTreeMap<String, Vec<&FileInfo>>,
            current_path: &str,
            prefix: &str,
        ) {
            let files = match tree.get(current_path) {
                Some(files) => files,
                None => return,
            };

            for file in files.iter() {
                output.push_str(&format!("{}  {}", prefix, file.name));

                if file.ignored {
                    output.push_str(" (ignored)");
                }

                output.push('\n');

                // If it's a directory, recurse
                if matches!(file.file_type, FileType::Directory) {
                    let child_path = if current_path.is_empty() {
                        file.name.clone()
                    } else {
                        format!("{}/{}", current_path, file.name)
                    };
                    build_tree_level(output, tree, &child_path, &format!("{}  ", prefix));
                }
            }
        }

        build_tree_level(&mut output, &file_tree, "", "");
        output
    }

    /// Check if a file is ignored by .gitignore
    fn is_ignored_by_gitignore(&self, file_path: &Path) -> Result<bool> {
        // For now, implement basic ignore patterns
        // TODO: Implement full .gitignore parsing
        let file_name = file_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // Common ignore patterns
        let ignore_patterns = [
            "node_modules",
            ".git",
            "dist",
            "build",
            ".next",
            "__pycache__",
            "*.pyc",
            ".DS_Store",
            "target", // Rust build directory
            "Cargo.lock",
        ];

        // Check if file name matches any ignore pattern
        for pattern in &ignore_patterns {
            if file_name == *pattern || file_name.starts_with(&format!("{}.", pattern)) {
                return Ok(true);
            }
        }

        // Check if any component in the path matches ignore patterns
        for component in file_path.components() {
            let comp_str = component.as_os_str().to_string_lossy();
            for pattern in &ignore_patterns {
                if comp_str == *pattern {
                    return Ok(true);
                }
            }
        }

        Ok(false)
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
            max_files: None,
        };

        let response = executor
            .execute(ToolRequest::ListFiles(request))
            .await
            .unwrap();

        match response {
            ToolResponse::ListFiles(list_response) => {
                assert_eq!(list_response.total_files, 3);
                assert!(!list_response.truncated);
                assert_eq!(list_response.limit, 100);
                // Check that tree string contains expected files
                assert!(list_response.files.contains("test.txt"));
                assert!(list_response.files.contains("subdir"));
                assert!(list_response.files.contains("nested.txt"));
            }
            _ => panic!("Expected ListFiles response"),
        }
    }

    #[tokio::test]
    async fn test_tool_executor_list_files_hidden_files_recursive() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::new(temp_dir.path().to_path_buf());

        // Create test files including hidden ones
        fs::write(temp_dir.path().join("normal.txt"), "Normal file").unwrap();
        fs::write(temp_dir.path().join(".hidden.txt"), "Hidden file").unwrap();
        fs::create_dir_all(temp_dir.path().join(".hidden_dir")).unwrap();
        fs::write(
            temp_dir.path().join(".hidden_dir/file.txt"),
            "File in hidden dir",
        )
        .unwrap();
        fs::create_dir_all(temp_dir.path().join("normal_dir")).unwrap();
        fs::write(
            temp_dir.path().join("normal_dir/.hidden_in_normal.txt"),
            "Hidden in normal dir",
        )
        .unwrap();

        // Test without including hidden files
        let request = ListFilesRequest {
            path: ".".to_string(),
            recursive: Some(true),
            include_hidden: Some(false),
            max_files: None,
        };

        let response = executor
            .execute(ToolRequest::ListFiles(request))
            .await
            .unwrap();

        match response {
            ToolResponse::ListFiles(list_response) => {
                // Should only include normal.txt and normal_dir
                assert_eq!(list_response.total_files, 2);
                assert!(list_response.files.contains("normal.txt"));
                assert!(list_response.files.contains("normal_dir"));
                // Should not include any hidden files
                assert!(!list_response.files.contains(".hidden.txt"));
            }
            _ => panic!("Expected ListFiles response"),
        }

        // Test with including hidden files
        let request_hidden = ListFilesRequest {
            path: ".".to_string(),
            recursive: Some(true),
            include_hidden: Some(true),
            max_files: None,
        };

        let response_hidden = executor
            .execute(ToolRequest::ListFiles(request_hidden))
            .await
            .unwrap();

        match response_hidden {
            ToolResponse::ListFiles(list_response) => {
                // Should include all files
                assert_eq!(list_response.total_files, 6);
                assert!(list_response.files.contains("normal.txt"));
                assert!(list_response.files.contains(".hidden.txt"));
                assert!(list_response.files.contains(".hidden_dir"));
                assert!(list_response.files.contains("file.txt"));
                assert!(list_response.files.contains("normal_dir"));
                assert!(list_response.files.contains(".hidden_in_normal.txt"));
            }
            _ => panic!("Expected ListFiles response"),
        }
    }

    #[tokio::test]
    async fn test_tool_executor_list_files_sorting() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::new(temp_dir.path().to_path_buf());

        // Create files with mixed case and types
        fs::create_dir_all(temp_dir.path().join("Z_dir")).unwrap();
        fs::create_dir_all(temp_dir.path().join("a_dir")).unwrap();
        fs::write(temp_dir.path().join("Z_file.txt"), "Z content").unwrap();
        fs::write(temp_dir.path().join("a_file.txt"), "a content").unwrap();
        fs::write(temp_dir.path().join("M_file.txt"), "M content").unwrap();

        let request = ListFilesRequest {
            path: ".".to_string(),
            recursive: Some(false),
            include_hidden: Some(false),
            max_files: None,
        };

        let response = executor
            .execute(ToolRequest::ListFiles(request))
            .await
            .unwrap();

        match response {
            ToolResponse::ListFiles(list_response) => {
                assert_eq!(list_response.total_files, 5);

                // Check that tree string contains expected files
                assert!(list_response.files.contains("a_dir"));
                assert!(list_response.files.contains("Z_dir"));
                assert!(list_response.files.contains("a_file.txt"));
                assert!(list_response.files.contains("M_file.txt"));
                assert!(list_response.files.contains("Z_file.txt"));
            }
            _ => panic!("Expected ListFiles response"),
        }
    }

    #[tokio::test]
    async fn test_tool_executor_list_files_max_files_limit() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::new(temp_dir.path().to_path_buf());

        // Create more files than the limit
        for i in 0..15 {
            fs::write(
                temp_dir.path().join(format!("file_{:02}.txt", i)),
                format!("Content {}", i),
            )
            .unwrap();
        }

        // Test with max_files limit of 10
        let request = ListFilesRequest {
            path: ".".to_string(),
            recursive: Some(false),
            include_hidden: Some(false),
            max_files: Some(10),
        };

        let response = executor
            .execute(ToolRequest::ListFiles(request))
            .await
            .unwrap();

        match response {
            ToolResponse::ListFiles(list_response) => {
                assert_eq!(list_response.total_files, 10);
                assert!(list_response.truncated);
                assert_eq!(list_response.limit, 10);
            }
            _ => panic!("Expected ListFiles response"),
        }

        // Test with max_files higher than available files
        let request_high_limit = ListFilesRequest {
            path: ".".to_string(),
            recursive: Some(false),
            include_hidden: Some(false),
            max_files: Some(20),
        };

        let response_high = executor
            .execute(ToolRequest::ListFiles(request_high_limit))
            .await
            .unwrap();

        match response_high {
            ToolResponse::ListFiles(list_response) => {
                assert_eq!(list_response.total_files, 15);
                assert!(!list_response.truncated);
                assert_eq!(list_response.limit, 20);
            }
            _ => panic!("Expected ListFiles response"),
        }
    }

    #[tokio::test]
    async fn test_path_normalization() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::new(temp_dir.path().to_path_buf());

        // Create test structure
        fs::create_dir_all(temp_dir.path().join("subdir")).unwrap();
        fs::write(temp_dir.path().join("subdir/test.txt"), "test content").unwrap();

        // Test path normalization with . and .. components
        let test_cases = vec![
            ("./subdir/../subdir/test.txt", "subdir/test.txt"),
            ("subdir/./test.txt", "subdir/test.txt"),
            ("subdir//test.txt", "subdir/test.txt"), // Multiple slashes
        ];

        for (input_path, expected_relative) in test_cases {
            let resolved = executor.validate_and_resolve_path(input_path).unwrap();
            let expected = temp_dir.path().join(expected_relative);
            assert_eq!(
                resolved, expected,
                "Failed to normalize path: {}",
                input_path
            );
        }

        // Test directory traversal prevention
        let traversal_result = executor.validate_and_resolve_path("../outside");
        assert!(
            traversal_result.is_err(),
            "Should prevent directory traversal"
        );

        let traversal_result2 = executor.validate_and_resolve_path("../../../etc/passwd");
        assert!(
            traversal_result2.is_err(),
            "Should prevent directory traversal with multiple .."
        );
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

    #[tokio::test]
    async fn test_tool_executor_write_file() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::new(temp_dir.path().to_path_buf());

        let request = WriteFileRequest {
            path: "test.txt".to_string(),
            content: "Hello World".to_string(),
            create_dirs: None,
            append: None,
            search: None,
            replace: None,
            create_if_not_exists: Some(true),
        };

        let response = executor
            .execute(ToolRequest::WriteFile(request))
            .await
            .unwrap();

        match response {
            ToolResponse::WriteFile(write_response) => {
                assert_eq!(write_response.path, "test.txt");
                assert!(write_response.success);
                assert_eq!(write_response.bytes_written, 11);
                assert!(write_response.created);
            }
            _ => panic!("Expected WriteFile response"),
        }

        // Verify file was created
        let content = fs::read_to_string(temp_dir.path().join("test.txt")).unwrap();
        assert_eq!(content, "Hello World");
    }

    #[tokio::test]
    async fn test_tool_executor_write_file_search_replace() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::new(temp_dir.path().to_path_buf());

        // Create initial file
        fs::write(temp_dir.path().join("test.txt"), "Hello old world").unwrap();

        let request = WriteFileRequest {
            path: "test.txt".to_string(),
            content: "".to_string(), // Not used in search/replace
            create_dirs: None,
            append: None,
            search: Some("old".to_string()),
            replace: Some("new".to_string()),
            create_if_not_exists: None,
        };

        let response = executor
            .execute(ToolRequest::WriteFile(request))
            .await
            .unwrap();

        match response {
            ToolResponse::WriteFile(write_response) => {
                assert_eq!(write_response.path, "test.txt");
                assert!(write_response.success);
                assert_eq!(write_response.bytes_written, 15); // "Hello new world".len()
                assert!(!write_response.created); // File was modified, not created
            }
            _ => panic!("Expected WriteFile response"),
        }

        // Verify content was replaced
        let content = fs::read_to_string(temp_dir.path().join("test.txt")).unwrap();
        assert_eq!(content, "Hello new world");
    }

    #[tokio::test]
    async fn test_tool_executor_grep_search() {
        let temp_dir = TempDir::new().unwrap();
        let executor = ToolExecutor::new(temp_dir.path().to_path_buf());

        // Create test files
        fs::write(
            temp_dir.path().join("test1.txt"),
            "fn main() {\n    println!(\"Hello\");\n}",
        )
        .unwrap();
        fs::write(
            temp_dir.path().join("test2.txt"),
            "fn helper() {\n    println!(\"World\");\n}",
        )
        .unwrap();

        let request = GrepRequest {
            pattern: "fn \\w+\\(\\)".to_string(),
            path: None,
            include_pattern: None,
            exclude_pattern: None,
            recursive: Some(false),
            case_sensitive: Some(false),
            include_line_numbers: Some(true),
            max_results: Some(10),
        };

        let response = executor.execute(ToolRequest::Grep(request)).await.unwrap();

        match response {
            ToolResponse::Grep(grep_response) => {
                assert_eq!(grep_response.pattern, "fn \\w+\\(\\)");
                assert!(grep_response.total_matches >= 2); // Should find both functions
                assert!(grep_response.files_searched >= 2);
                assert!(!grep_response.truncated);

                // Check that we found matches
                let main_match = grep_response
                    .matches
                    .iter()
                    .find(|m| m.matched_text.contains("main"));
                let helper_match = grep_response
                    .matches
                    .iter()
                    .find(|m| m.matched_text.contains("helper"));

                assert!(main_match.is_some());
                assert!(helper_match.is_some());
            }
            _ => panic!("Expected Grep response"),
        }
    }
}
