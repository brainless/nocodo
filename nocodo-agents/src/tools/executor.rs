use anyhow::Result;
use serde_json::Value;
use std::path::PathBuf;
use std::time::Instant;

use super::schemas::*;

pub struct ToolExecutor {
    base_path: PathBuf,
}

impl ToolExecutor {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    pub async fn execute(&self, tool_name: &str, arguments: Value) -> Result<Value> {
        let start = Instant::now();

        let result = match tool_name {
            "list_files" => self.execute_list_files(arguments).await,
            "read_file" => self.execute_read_file(arguments).await,
            "grep" => self.execute_grep(arguments).await,
            "write_file" => self.execute_write_file(arguments).await,
            "edit_file" => self.execute_edit_file(arguments).await,
            "bash" => self.execute_bash(arguments).await,
            _ => Err(anyhow::anyhow!("Unknown tool: {}", tool_name)),
        };

        let execution_time = start.elapsed().as_millis() as i64;
        tracing::info!("Tool {} executed in {}ms", tool_name, execution_time);

        result
    }

    async fn execute_list_files(&self, args: Value) -> Result<Value> {
        let params: ListFilesParams = serde_json::from_value(args)?;
        
        let search_path = if let Some(path) = params.path {
            self.base_path.join(path)
        } else {
            self.base_path.clone()
        };

        // Use glob pattern matching
        let pattern = search_path.join(&params.pattern);
        let pattern_str = pattern.to_string_lossy();

        let mut matches = Vec::new();
        
        // Simple glob implementation using walkdir
        for entry in walkdir::WalkDir::new(&self.base_path)
            .follow_links(false)
            .max_depth(10)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file() {
                if let Ok(path_relative) = path.strip_prefix(&self.base_path) {
                    let path_str = path_relative.to_string_lossy();
                    if self.matches_glob(&path_str, &params.pattern) {
                        matches.push(serde_json::json!({
                            "path": path_str,
                            "absolute_path": path.to_string_lossy(),
                        }));
                    }
                }
            }
        }

        Ok(serde_json::json!({
            "files": matches,
            "count": matches.len()
        }))
    }

    async fn execute_read_file(&self, args: Value) -> Result<Value> {
        let params: ReadFileParams = serde_json::from_value(args)?;
        let file_path = self.base_path.join(&params.file_path);

        if !file_path.exists() {
            return Err(anyhow::anyhow!("File not found: {}", params.file_path));
        }

        let content = std::fs::read_to_string(&file_path)?;
        let lines: Vec<&str> = content.lines().collect();

        let (start_line, end_line) = match (params.offset, params.limit) {
            (Some(offset), Some(limit)) => (offset, offset + limit),
            (Some(offset), None) => (offset, lines.len()),
            (None, Some(limit)) => (0, limit),
            (None, None) => (0, lines.len()),
        };

        let end_line = end_line.min(lines.len());
        let selected_lines: Vec<&str> = lines[start_line..end_line].to_vec();
        let content = selected_lines.join("\n");

        Ok(serde_json::json!({
            "content": content,
            "line_count": selected_lines.len(),
            "total_lines": lines.len(),
            "file_path": params.file_path
        }))
    }

    async fn execute_grep(&self, args: Value) -> Result<Value> {
        let params: GrepParams = serde_json::from_value(args)?;
        
        let mut matches = Vec::new();
        
        // Simple grep implementation using walkdir
        for entry in walkdir::WalkDir::new(&self.base_path)
            .follow_links(false)
            .max_depth(10)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file() {
                if let Some(path_relative) = path.strip_prefix(&self.base_path).ok() {
                    let path_str = path_relative.to_string_lossy();
                    
                    // Check file extension if specified
                    if let Some(file_type) = &params.file_type {
                        if let Some(extension) = path.extension() {
                            if extension.to_string_lossy() != *file_type {
                                continue;
                            }
                        } else {
                            continue;
                        }
                    }

                    // Check glob pattern if specified
                    if let Some(glob) = &params.glob {
                        if !self.matches_glob(&path_str, glob) {
                            continue;
                        }
                    }

                    // Read file and search for pattern
                    if let Ok(content) = std::fs::read_to_string(path) {
                        let regex = regex::Regex::new(&params.pattern)?;
                        for (line_num, line) in content.lines().enumerate() {
                            let search_line = if params.case_insensitive.unwrap_or(false) {
                                line.to_lowercase()
                            } else {
                                line.to_string()
                            };
                            
                            let search_pattern = if params.case_insensitive.unwrap_or(false) {
                                params.pattern.to_lowercase()
                            } else {
                                params.pattern.clone()
                            };

                            if search_line.contains(&search_pattern) || regex.is_match(line) {
                                matches.push(serde_json::json!({
                                    "file": path_str,
                                    "line": line_num + 1,
                                    "content": line,
                                }));
                            }
                        }
                    }
                }
            }
        }

        Ok(serde_json::json!({
            "matches": matches,
            "count": matches.len(),
            "pattern": params.pattern
        }))
    }

    async fn execute_write_file(&self, args: Value) -> Result<Value> {
        let params: WriteFileParams = serde_json::from_value(args)?;
        let file_path = self.base_path.join(&params.file_path);

        // Ensure parent directory exists
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let content_len = params.content.len();
        std::fs::write(&file_path, &params.content)?;

        Ok(serde_json::json!({
            "success": true,
            "file_path": params.file_path,
            "bytes_written": content_len
        }))
    }

    async fn execute_edit_file(&self, args: Value) -> Result<Value> {
        let params: EditFileParams = serde_json::from_value(args)?;
        let file_path = self.base_path.join(&params.file_path);

        if !file_path.exists() {
            return Err(anyhow::anyhow!("File not found: {}", params.file_path));
        }

        let mut content = std::fs::read_to_string(&file_path)?;
        let replace_all = params.replace_all.unwrap_or(false);

        let replacements_made = if replace_all {
            content = content.replace(&params.old_string, &params.new_string);
            content.matches(&params.old_string).count()
        } else {
            if let Some(pos) = content.find(&params.old_string) {
                content.replace_range(pos..pos + params.old_string.len(), &params.new_string);
                1
            } else {
                0
            }
        };

        if replacements_made > 0 {
            std::fs::write(&file_path, content)?;
        }

        Ok(serde_json::json!({
            "success": true,
            "file_path": params.file_path,
            "replacements_made": replacements_made,
            "replace_all": replace_all
        }))
    }

    async fn execute_bash(&self, args: Value) -> Result<Value> {
        let params: BashParams = serde_json::from_value(args)?;
        
        let workdir = if let Some(dir) = params.workdir {
            self.base_path.join(dir)
        } else {
            self.base_path.clone()
        };

        let _timeout = params.timeout.unwrap_or(120000); // Default 2 minutes

        let output = tokio::process::Command::new("bash")
            .arg("-c")
            .arg(&params.command)
            .current_dir(&workdir)
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        Ok(serde_json::json!({
            "success": output.status.success(),
            "exit_code": output.status.code(),
            "stdout": stdout.to_string(),
            "stderr": stderr.to_string(),
            "command": params.command,
            "workdir": workdir.to_string_lossy()
        }))
    }

    // Simple glob matching function
    fn matches_glob(&self, path: &str, pattern: &str) -> bool {
        // Convert glob pattern to regex
        let mut regex_pattern = pattern
            .replace('.', "\\.")
            .replace("**", ".*")
            .replace('*', "[^/]*")
            .replace('?', "[^/]");

        // Ensure full match
        if !regex_pattern.starts_with(".*") {
            regex_pattern = format!("^{}", regex_pattern);
        }
        if !regex_pattern.ends_with(".*") {
            regex_pattern = format!("{}$", regex_pattern);
        }

        if let Ok(regex) = regex::Regex::new(&regex_pattern) {
            regex.is_match(path)
        } else {
            // Fallback to simple string matching if regex fails
            path.contains(pattern.replace("**", "").replace("*", "").replace("?", "").as_str())
        }
    }
}