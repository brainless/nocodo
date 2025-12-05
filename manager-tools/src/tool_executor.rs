use manager_models::{
    ApplyPatchFileChange, ApplyPatchRequest, ApplyPatchResponse, BashRequest, BashResponse,
    FileInfo, FileType, GrepMatch, GrepRequest, GrepResponse, ListFilesRequest, ListFilesResponse,
    ReadFileRequest, ReadFileResponse, ToolErrorResponse, ToolRequest, ToolResponse,
    WriteFileRequest, WriteFileResponse,
};
use crate::tool_error::ToolError;
use anyhow::Result;
use base64::{Engine, prelude::BASE64_STANDARD};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Instant, UNIX_EPOCH};
use walkdir::WalkDir;

/// Bash execution result type (re-exported from manager)
#[derive(Debug, Clone)]
pub struct BashExecutionResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub timed_out: bool,
}

/// Bash executor trait to avoid circular dependency
pub trait BashExecutorTrait {
    fn execute_with_cwd(
        &self,
        command: &str,
        working_dir: &PathBuf,
        timeout_secs: Option<u64>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<BashExecutionResult>> + Send + '_>>;
}

/// Tool executor that handles tool requests and responses
pub struct ToolExecutor {
    base_path: PathBuf,
    max_file_size: u64,
    bash_executor: Option<Box<dyn BashExecutorTrait + Send + Sync>>,
}

impl ToolExecutor {
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            base_path,
            max_file_size: 1024 * 1024, // 1MB default
            bash_executor: None,        // Bash executor will be initialized separately
        }
    }

    pub fn with_bash_executor(mut self, bash_executor: Box<dyn BashExecutorTrait + Send + Sync>) -> Self {
        self.bash_executor = Some(bash_executor);
        self
    }

    pub fn bash_executor(&self) -> Option<&(dyn BashExecutorTrait + Send + Sync)> {
        self.bash_executor.as_ref().map(|executor| executor.as_ref())
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
            ToolRequest::ApplyPatch(req) => self.apply_patch(req).await,
            ToolRequest::Bash(req) => self.execute_bash(req).await,
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
                    BASE64_STANDARD.encode(&bytes)
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
        // Validate request parameters
        if let Err(e) = request.validate() {
            return Ok(ToolResponse::Error(ToolErrorResponse {
                tool: "write_file".to_string(),
                error: "ValidationError".to_string(),
                message: e.to_string(),
            }));
        }

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

    /// Convert glob pattern to regex pattern
    /// Examples:
    /// - *.rs -> .*\.rs$
    /// - *.py -> .*\.py$
    /// - test*.txt -> ^test.*\.txt$
    /// - **/*.rs -> .*/.*\.rs$ (for nested paths)
    fn glob_to_regex(glob: &str) -> String {
        let mut regex = String::new();
        let mut chars = glob.chars().peekable();

        // Add start anchor unless pattern starts with ** or *
        if !glob.starts_with("**") && !glob.starts_with('*') {
            regex.push('^');
        }

        while let Some(ch) = chars.next() {
            match ch {
                '*' => {
                    // Check for ** pattern (match any directory depth)
                    if chars.peek() == Some(&'*') {
                        chars.next(); // consume second *
                        regex.push_str(".*");
                    } else {
                        // Single * matches any characters except path separator
                        regex.push_str("[^/]*");
                    }
                }
                '?' => {
                    // ? matches any single character except path separator
                    regex.push_str("[^/]");
                }
                '.' | '+' | '^' | '$' | '(' | ')' | '[' | ']' | '{' | '}' | '|' | '\\' => {
                    // Escape regex special characters
                    regex.push('\\');
                    regex.push(ch);
                }
                _ => {
                    regex.push(ch);
                }
            }
        }

        // Add end anchor if pattern doesn't contain directory separators or wildcards at the end
        if !regex.ends_with(".*") {
            regex.push('$');
        }

        regex
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

        // Compile include/exclude patterns (convert from glob to regex)
        let include_regex =
            if let Some(pattern) = &request.include_pattern {
                let regex_pattern = Self::glob_to_regex(pattern);
                Some(RegexBuilder::new(&regex_pattern).build().map_err(|e| {
                    ToolError::InvalidPath(format!("Invalid include pattern: {}", e))
                })?)
            } else {
                None
            };

        let exclude_regex =
            if let Some(pattern) = &request.exclude_pattern {
                let regex_pattern = Self::glob_to_regex(pattern);
                Some(RegexBuilder::new(&regex_pattern).build().map_err(|e| {
                    ToolError::InvalidPath(format!("Invalid exclude pattern: {}", e))
                })?)
            } else {
                None
            };

        let mut matches = Vec::new();
        let mut files_searched = 0;
        let max_results = request.max_results.unwrap_or(100) as usize;
        let max_files_searched = request.max_files_searched.unwrap_or(1000) as usize;

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

            // Calculate relative path for display
            // When searching a single file, use the file name instead of empty string
            let relative_path = if search_path.is_file() {
                file_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| file_path.to_string_lossy().to_string())
            } else {
                file_path
                    .strip_prefix(&search_path)
                    .unwrap_or(file_path)
                    .to_string_lossy()
                    .to_string()
            };

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
            let file_path_str = relative_path.clone();

            // Skip common build artifacts and directories
            let skip_patterns = [
                "target",
                "node_modules",
                ".git",
                "dist",
                "build",
                "__pycache__",
                ".next",
                ".nuxt",
                ".vuepress",
                ".cache",
                ".parcel-cache",
                ".DS_Store",
                "Thumbs.db",
                "desktop.ini",
            ];

            let should_skip = file_name.starts_with('.')
                || skip_patterns.contains(&file_name.as_ref())
                || file_name.ends_with(".pyc")
                || file_name.ends_with(".pyo")
                || file_name == "Cargo.lock"
                || file_name == "package-lock.json"
                || file_name == "yarn.lock"
                || file_name == "pnpm-lock.yaml"
                || file_path_str.contains("/target/")
                || file_path_str.contains("/node_modules/")
                || file_path_str.contains("/.git/")
                || file_path_str.contains("/dist/")
                || file_path_str.contains("/build/")
                || file_path_str.contains("/__pycache__/");

            if should_skip {
                continue;
            }

            // Skip binary files by checking file extension and attempting to read as UTF-8
            let is_likely_binary = file_name.ends_with(".exe")
                || file_name.ends_with(".dll")
                || file_name.ends_with(".so")
                || file_name.ends_with(".dylib")
                || file_name.ends_with(".bin")
                || file_name.ends_with(".jpg")
                || file_name.ends_with(".jpeg")
                || file_name.ends_with(".png")
                || file_name.ends_with(".gif")
                || file_name.ends_with(".bmp")
                || file_name.ends_with(".tiff")
                || file_name.ends_with(".ico")
                || file_name.ends_with(".pdf")
                || file_name.ends_with(".zip")
                || file_name.ends_with(".tar")
                || file_name.ends_with(".gz")
                || file_name.ends_with(".bz2")
                || file_name.ends_with(".xz")
                || file_name.ends_with(".7z")
                || file_name.ends_with(".rar");

            if is_likely_binary {
                continue;
            }

            files_searched += 1;

            // Check if we've reached the max files searched limit
            if files_searched >= max_files_searched {
                break;
            }

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
                        file_path: relative_path.clone(),
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

        let mut total_matches = matches.len() as u32;
        let mut truncated = matches.len() >= max_results;

        // Check response size and truncate if necessary (limit to ~100KB)
        const MAX_RESPONSE_SIZE: usize = 100 * 1024; // 100KB
        let response_size_estimate = matches
            .iter()
            .map(|m| m.file_path.len() + m.line_content.len() + m.matched_text.len() + 100) // rough estimate
            .sum::<usize>();

        if response_size_estimate > MAX_RESPONSE_SIZE {
            // Truncate matches to fit within size limit
            let mut truncated_matches = Vec::new();
            let mut current_size = 0;

            for match_item in matches {
                let item_size = match_item.file_path.len()
                    + match_item.line_content.len()
                    + match_item.matched_text.len()
                    + 100;
                if current_size + item_size > MAX_RESPONSE_SIZE {
                    truncated = true;
                    break;
                }
                current_size += item_size;
                truncated_matches.push(match_item);
            }

            matches = truncated_matches;
            total_matches = matches.len() as u32;
        }

        Ok(ToolResponse::Grep(GrepResponse {
            pattern: request.pattern,
            matches,
            total_matches,
            files_searched: files_searched as u32,
            truncated,
        }))
    }

    /// Apply a patch to multiple files
    async fn apply_patch(&self, request: ApplyPatchRequest) -> Result<ToolResponse> {
        use codex_apply_patch::{parse_patch, Hunk};

        // Parse the patch
        let parsed = match parse_patch(&request.patch) {
            Ok(parsed) => parsed,
            Err(e) => {
                return Ok(ToolResponse::Error(ToolErrorResponse {
                    tool: "apply_patch".to_string(),
                    error: "ParseError".to_string(),
                    message: format!("Failed to parse patch: {}", e),
                }));
            }
        };

        // Change to base directory before applying patch
        let original_dir = std::env::current_dir()
            .map_err(|e| anyhow::anyhow!("Failed to get current directory: {}", e))?;

        std::env::set_current_dir(&self.base_path)
            .map_err(|e| anyhow::anyhow!("Failed to change to base directory: {}", e))?;

        let mut files_changed = Vec::new();
        let mut total_additions = 0;
        let mut total_deletions = 0;
        let mut errors = Vec::new();

        // Process each hunk
        for hunk in &parsed.hunks {
            match hunk {
                Hunk::AddFile { path, contents } => {
                    // Validate path
                    let path_str = path.to_string_lossy().to_string();
                    if let Err(e) = self.validate_and_resolve_path(&path_str) {
                        errors.push(format!("Invalid path '{}': {}", path_str, e));
                        continue;
                    }

                    // Create parent directories if needed
                    if let Some(parent) = path.parent() {
                        if !parent.as_os_str().is_empty() {
                            if let Err(e) = fs::create_dir_all(parent) {
                                errors.push(format!(
                                    "Failed to create parent directory for '{}': {}",
                                    path_str, e
                                ));
                                continue;
                            }
                        }
                    }

                    // Write the new file
                    if let Err(e) = fs::write(path, contents) {
                        errors.push(format!("Failed to create file '{}': {}", path_str, e));
                        continue;
                    }

                    let line_count = contents.lines().count();
                    total_additions += line_count;

                    files_changed.push(ApplyPatchFileChange {
                        path: path_str,
                        operation: "add".to_string(),
                        new_path: None,
                        unified_diff: None,
                    });
                }
                Hunk::DeleteFile { path } => {
                    let path_str = path.to_string_lossy().to_string();
                    if let Err(e) = self.validate_and_resolve_path(&path_str) {
                        errors.push(format!("Invalid path '{}': {}", path_str, e));
                        continue;
                    }

                    // Read the file first to count deletions
                    if let Ok(content) = fs::read_to_string(path) {
                        total_deletions += content.lines().count();
                    }

                    // Delete the file
                    if let Err(e) = fs::remove_file(path) {
                        errors.push(format!("Failed to delete file '{}': {}", path_str, e));
                        continue;
                    }

                    files_changed.push(ApplyPatchFileChange {
                        path: path_str,
                        operation: "delete".to_string(),
                        new_path: None,
                        unified_diff: None,
                    });
                }
                Hunk::UpdateFile {
                    path,
                    move_path,
                    chunks,
                } => {
                    let path_str = path.to_string_lossy().to_string();
                    if let Err(e) = self.validate_and_resolve_path(&path_str) {
                        errors.push(format!("Invalid path '{}': {}", path_str, e));
                        continue;
                    }

                    // Read original content
                    let original_content = match fs::read_to_string(path) {
                        Ok(content) => content,
                        Err(e) => {
                            errors.push(format!("Failed to read file '{}': {}", path_str, e));
                            continue;
                        }
                    };

                    // Apply chunks using codex-apply-patch's logic
                    let mut original_lines: Vec<String> =
                        original_content.split('\n').map(String::from).collect();
                    if original_lines.last().is_some_and(String::is_empty) {
                        original_lines.pop();
                    }

                    // Apply each chunk to the file
                    let mut modified_content = original_content.clone();

                    for chunk in chunks {
                        total_deletions += chunk.old_lines.len();
                        total_additions += chunk.new_lines.len();

                        // Find and replace the old_lines with new_lines
                        let old_text = chunk.old_lines.join("\n");
                        let new_text = chunk.new_lines.join("\n");

                        // Try to find the exact match first
                        if let Some(pos) = modified_content.find(&old_text) {
                            // Replace the found text
                            modified_content.replace_range(pos..pos + old_text.len(), &new_text);
                        } else {
                            // If exact match fails, try with context
                            if let Some(ref context) = chunk.change_context {
                                // Find the context line first
                                if let Some(context_pos) = modified_content.find(context) {
                                    // Search for old_lines after the context
                                    let search_start = context_pos + context.len();
                                    if let Some(relative_pos) =
                                        modified_content[search_start..].find(&old_text)
                                    {
                                        let absolute_pos = search_start + relative_pos;
                                        modified_content.replace_range(
                                            absolute_pos..absolute_pos + old_text.len(),
                                            &new_text,
                                        );
                                    } else {
                                        errors.push(format!(
                                            "Could not find old lines in '{}' after context '{}'",
                                            path_str, context
                                        ));
                                        continue;
                                    }
                                } else {
                                    errors.push(format!(
                                        "Could not find context '{}' in '{}'",
                                        context, path_str
                                    ));
                                    continue;
                                }
                            } else {
                                errors.push(format!(
                                    "Could not find old lines in '{}' and no context provided",
                                    path_str
                                ));
                                continue;
                            }
                        }
                    }

                    // Write the modified content back to the file
                    if let Err(e) = fs::write(path, modified_content) {
                        errors.push(format!(
                            "Failed to write modified file '{}': {}",
                            path_str, e
                        ));
                        continue;
                    }

                    let operation = if move_path.is_some() {
                        "move"
                    } else {
                        "update"
                    };

                    files_changed.push(ApplyPatchFileChange {
                        path: path_str,
                        operation: operation.to_string(),
                        new_path: move_path.as_ref().map(|p| p.to_string_lossy().to_string()),
                        unified_diff: None,
                    });
                }
            }
        }

        // Restore original directory
        let _ = std::env::set_current_dir(original_dir);

        // Determine success
        let success = errors.is_empty();
        let message = if success {
            format!(
                "Successfully applied patch: {} file(s) changed, {} additions(+), {} deletions(-)",
                files_changed.len(),
                total_additions,
                total_deletions
            )
        } else {
            format!(
                "Patch partially applied with {} error(s): {}",
                errors.len(),
                errors.join("; ")
            )
        };

        Ok(ToolResponse::ApplyPatch(ApplyPatchResponse {
            success,
            files_changed,
            total_additions,
            total_deletions,
            message,
        }))
    }

    /// Execute a bash command
    async fn execute_bash(&self, request: BashRequest) -> Result<ToolResponse> {
        // Check if bash executor is available
        let bash_executor = match &self.bash_executor {
            Some(executor) => executor,
            None => {
                return Ok(ToolResponse::Error(ToolErrorResponse {
                    tool: "bash".to_string(),
                    error: "BashExecutorNotAvailable".to_string(),
                    message: "Bash executor is not configured".to_string(),
                }));
            }
        };

        let start_time = Instant::now();

        // Determine working directory
        let working_dir = if let Some(dir) = &request.working_dir {
            // Validate and resolve the working directory
            match self.validate_and_resolve_path(dir) {
                Ok(path) => path,
                Err(e) => {
                    return Ok(ToolResponse::Error(ToolErrorResponse {
                        tool: "bash".to_string(),
                        error: "InvalidWorkingDirectory".to_string(),
                        message: format!("Invalid working directory '{}': {}", dir, e),
                    }));
                }
            }
        } else {
            self.base_path.clone()
        };

        // Execute the command
        let result = bash_executor
            .execute_with_cwd(&request.command, &working_dir, request.timeout_secs)
            .await;

        let execution_time = start_time.elapsed().as_secs_f64();

        match result {
            Ok(bash_result) => Ok(ToolResponse::Bash(BashResponse {
                command: request.command,
                working_dir: request.working_dir,
                stdout: bash_result.stdout,
                stderr: bash_result.stderr,
                exit_code: bash_result.exit_code,
                timed_out: bash_result.timed_out,
                execution_time_secs: execution_time,
            })),
            Err(e) => Ok(ToolResponse::Error(ToolErrorResponse {
                tool: "bash".to_string(),
                error: "BashExecutionError".to_string(),
                message: format!("Failed to execute bash command: {}", e),
            })),
        }
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
        let is_directory = metadata.is_dir();

        Ok(FileInfo {
            name: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            path: relative_path_str,
            absolute: absolute_path_str,
            file_type: if is_directory {
                FileType::Directory
            } else {
                FileType::File
            },
            ignored,
            is_directory,
            size: if is_directory {
                None
            } else {
                metadata.len().into()
            },
            modified_at: if is_directory {
                None
            } else {
                metadata.modified().ok().map(|t| {
                    t.duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                        .to_string()
                })
            },
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

            file_tree.entry(parent_key).or_default().push(file);
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
            ToolResponse::ApplyPatch(response) => serde_json::to_value(response)?,
            ToolResponse::Bash(response) => serde_json::to_value(response)?,
            ToolResponse::Error(response) => serde_json::to_value(response)?,
        };

        Ok(response_value)
    }
}

