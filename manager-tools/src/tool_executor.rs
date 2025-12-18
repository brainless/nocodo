use anyhow::Result;
use manager_models::{ToolRequest, ToolResponse};
use serde_json::Value;
use std::path::PathBuf;

// Re-export from individual modules
use crate::bash;
pub use crate::bash::{BashExecutionResult, BashExecutorTrait};
use crate::filesystem::{apply_patch, list_files, read_file, write_file};
use crate::grep;
use crate::user_interaction;

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

    pub fn with_bash_executor(
        mut self,
        bash_executor: Box<dyn BashExecutorTrait + Send + Sync>,
    ) -> Self {
        self.bash_executor = Some(bash_executor);
        self
    }

    pub fn bash_executor(&self) -> Option<&(dyn BashExecutorTrait + Send + Sync)> {
        self.bash_executor
            .as_ref()
            .map(|executor| executor.as_ref())
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
            ToolRequest::ListFiles(req) => list_files::list_files(&self.base_path, req).await,
            ToolRequest::ReadFile(req) => {
                read_file::read_file(&self.base_path, self.max_file_size, req).await
            }
            ToolRequest::WriteFile(req) => write_file::write_file(&self.base_path, req).await,
            ToolRequest::Grep(req) => grep::grep_search(&self.base_path, req).await,
            ToolRequest::ApplyPatch(req) => apply_patch::apply_patch(&self.base_path, req).await,
            ToolRequest::Bash(req) => {
                bash::execute_bash(
                    self.base_path.as_path(),
                    self.bash_executor.as_ref().map(|e| e.as_ref()),
                    req,
                )
                .await
            }
            ToolRequest::AskUser(req) => user_interaction::ask_user(req).await,
        }
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
            ToolResponse::AskUser(response) => serde_json::to_value(response)?,
            ToolResponse::Error(response) => serde_json::to_value(response)?,
        };

        Ok(response_value)
    }

    /// Convert glob pattern to regex pattern
    pub fn glob_to_regex(glob: &str) -> String {
        let mut regex = String::new();
        let chars: Vec<char> = glob.chars().collect();
        let mut i = 0;

        // Add start anchor if pattern doesn't start with *
        if !glob.starts_with('*') {
            regex.push('^');
        }

        while i < chars.len() {
            match chars[i] {
                '*' => {
                    if i + 1 < chars.len() && chars[i + 1] == '*' {
                        // ** pattern - match any number of directories
                        regex.push_str(".*");
                        i += 2;
                    } else {
                        // * pattern - match any characters except /
                        regex.push_str("[^/]*");
                        i += 1;
                    }
                }
                '?' => {
                    // ? pattern - match any single character except /
                    regex.push_str("[^/]");
                    i += 1;
                }
                '.' | '+' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '|' | '\\' => {
                    // Escape regex special characters
                    regex.push('\\');
                    regex.push(chars[i]);
                    i += 1;
                }
                c => {
                    regex.push(c);
                    i += 1;
                }
            }
        }

        // Add end anchor if pattern doesn't end with *
        if !glob.ends_with('*') {
            regex.push('$');
        }

        regex
    }
}
