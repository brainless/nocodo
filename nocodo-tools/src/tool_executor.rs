use crate::types::{ToolRequest, ToolResponse};
use anyhow::Result;
use serde_json::Value;
use std::path::PathBuf;

// Re-export from individual modules
use crate::bash;
pub use crate::bash::{BashExecutionResult, BashExecutorTrait};
use crate::filesystem::{apply_patch, list_files, read_file, write_file};
use crate::grep;
#[cfg(feature = "sqlite")]
use crate::hackernews;
use crate::imap;
use crate::pdftotext;
#[cfg(feature = "sqlite")]
use crate::sqlite_reader;
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

    /// Start building a ToolExecutor with custom configuration
    pub fn builder() -> ToolExecutorBuilder {
        ToolExecutorBuilder::default()
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
            #[cfg(feature = "sqlite")]
            ToolRequest::Sqlite3Reader(req) => sqlite_reader::execute_sqlite3_reader(req)
                .await
                .map_err(|e| anyhow::anyhow!(e)),
            #[cfg(feature = "sqlite")]
            ToolRequest::HackerNewsRequest(req) => hackernews::execute_hackernews_request(req)
                .await
                .map_err(|e| anyhow::anyhow!(e)),
            ToolRequest::ImapReader(req) => imap::execute_imap_reader(req)
                .await
                .map_err(|e| anyhow::anyhow!(e)),
            ToolRequest::PdfToText(req) => pdftotext::execute_pdftotext(req)
                .map(ToolResponse::PdfToText)
                .map_err(|e| anyhow::anyhow!(e)),
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
            #[cfg(feature = "sqlite")]
            ToolResponse::Sqlite3Reader(response) => serde_json::to_value(response)?,
            #[cfg(feature = "sqlite")]
            ToolResponse::HackerNewsResponse(response) => serde_json::to_value(response)?,
            ToolResponse::ImapReader(response) => serde_json::to_value(response)?,
            ToolResponse::PdfToText(response) => serde_json::to_value(response)?,
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

/// Builder for creating a ToolExecutor with custom configuration
#[derive(Default)]
pub struct ToolExecutorBuilder {
    base_path: Option<PathBuf>,
    max_file_size: Option<u64>,
    bash_executor: Option<Option<Box<dyn BashExecutorTrait + Send + Sync>>>,
}

impl ToolExecutorBuilder {
    /// Set the base path for file operations
    pub fn base_path(mut self, path: PathBuf) -> Self {
        self.base_path = Some(path);
        self
    }

    /// Set the maximum file size for file operations
    pub fn max_file_size(mut self, size: u64) -> Self {
        self.max_file_size = Some(size);
        self
    }

    /// Set a custom bash executor with specific permissions
    ///
    /// # Examples
    ///
    /// ## With custom bash executor
    /// ```rust
    /// use nocodo_tools::{ToolExecutor, bash::{BashExecutor, BashPermissions}};
    /// use std::path::PathBuf;
    ///
    /// let perms = BashPermissions::only_allow(vec!["tesseract*"]);
    /// let bash = BashExecutor::new(perms, 120)?;
    ///
    /// let executor = ToolExecutor::builder()
    ///     .base_path(PathBuf::from("."))
    ///     .bash_executor(Some(Box::new(bash)))
    ///     .build();
    /// ```
    ///
    /// ## Without bash executor (disable bash tool)
    /// ```rust
    /// let executor = ToolExecutor::builder()
    ///     .base_path(PathBuf::from("."))
    ///     .bash_executor(None)
    ///     .build();
    /// ```
    pub fn bash_executor(
        mut self,
        executor: Option<Box<dyn BashExecutorTrait + Send + Sync>>,
    ) -> Self {
        self.bash_executor = Some(executor);
        self
    }

    /// Build the ToolExecutor
    pub fn build(self) -> ToolExecutor {
        let base_path = self.base_path.unwrap_or_else(|| PathBuf::from("."));
        let max_file_size = self.max_file_size.unwrap_or(1024 * 1024); // 1MB default
        let bash_executor = self.bash_executor.unwrap_or(None);

        ToolExecutor {
            base_path,
            max_file_size,
            bash_executor,
        }
    }
}
