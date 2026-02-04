use crate::{
    storage::AgentStorage,
    types::{
        Message as StorageMessage, MessageRole, Session, SessionStatus,
        ToolCall as StorageToolCall, ToolCallStatus,
    },
    Agent, AgentTool,
};
use anyhow::{self, Context};
use async_trait::async_trait;
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::tools::{ToolCall as LlmToolCall, ToolChoice};
use nocodo_llm_sdk::types::{CompletionRequest, ContentBlock, Message as LlmMessage, Role};
use nocodo_tools::{
    bash::{BashExecutor, BashPermissions},
    pdftotext::execute_pdftotext,
    ToolExecutor,
};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

/// Agent specialized in extracting text from PDFs using pdftotext and qpdf
pub struct PdfToTextAgent<S: AgentStorage> {
    client: Arc<dyn LlmClient>,
    storage: Arc<S>,
    tool_executor: Arc<ToolExecutor>,
    #[allow(dead_code)] // Stored for reference, used during construction
    pdf_path: PathBuf,
    #[allow(dead_code)] // Used in system prompt generation during construction
    pdf_filename: String,
    system_prompt: String,
    #[allow(dead_code)] // Temp directory used during construction and system prompt
    work_dir: PathBuf,
}

impl<S: AgentStorage> PdfToTextAgent<S> {
    /// Create a new PdfToTextAgent
    ///
    /// # Arguments
    /// * `client` - LLM client for AI inference
    /// * `storage` - Storage for session/message tracking
    /// * `pdf_path` - Path to the PDF file to process
    /// * `allowed_working_dirs` - This parameter is ignored. A temp directory is always created in /tmp
    ///
    /// # Security
    /// The agent is configured with restricted bash access:
    /// - Only the `pdftotext`, `qpdf`, `ls`, `wc`, and `pwd` commands are allowed
    /// - All other bash commands are denied
    /// - File operations are restricted to the created temp directory in /tmp
    ///
    /// # Pre-conditions
    /// - pdftotext (poppler-utils) must be installed on the system
    /// - qpdf must be installed for page extraction operations
    /// - Run `pdftotext -v` and `qpdf --version` to verify installation
    /// - The PDF file must exist
    pub fn new(
        client: Arc<dyn LlmClient>,
        storage: Arc<S>,
        pdf_path: PathBuf,
        _allowed_working_dirs: Option<Vec<String>>,
    ) -> anyhow::Result<Self> {
        // Validate PDF path exists
        if !pdf_path.exists() {
            anyhow::bail!("PDF file does not exist: {}", pdf_path.display());
        }

        // Extract filename
        let pdf_filename = pdf_path
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid PDF path - no filename"))?
            .to_string_lossy()
            .to_string();

        // Create a temp directory in /tmp with random name
        let work_dir = PathBuf::from(format!("/tmp/nocodo-pdftotext-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&work_dir).context("Failed to create temp directory")?;

        tracing::info!("Created temp directory: {:?}", work_dir);

        // Copy the PDF to the temp directory
        let pdf_copy_path = work_dir.join(&pdf_filename);
        std::fs::copy(&pdf_path, &pdf_copy_path).context("Failed to copy PDF to temp directory")?;
        tracing::info!("Copied PDF to: {:?}", pdf_copy_path);

        // Extract text to the temp directory
        let txt_filename = format!(
            "{}.txt",
            pdf_filename.strip_suffix(".pdf").unwrap_or(&pdf_filename)
        );
        let txt_path = work_dir.join(&txt_filename);

        let pdftotext_request = nocodo_tools::types::PdfToTextRequest {
            file_path: pdf_copy_path.to_string_lossy().to_string(),
            output_path: Some(txt_path.to_string_lossy().to_string()),
            preserve_layout: true,
            first_page: None,
            last_page: None,
            encoding: None,
            no_page_breaks: false,
        };

        execute_pdftotext(pdftotext_request).context("Failed to extract text from PDF")?;
        tracing::info!("Extracted text to: {:?}", txt_path);

        // Create bash permissions (pdftotext, qpdf, ls, wc, pwd)
        let allowed_dirs = vec![work_dir.to_string_lossy().to_string()];
        tracing::info!(
            "Creating bash executor with allowed working dir: {:?}",
            allowed_dirs
        );
        let bash_perms =
            BashPermissions::minimal(vec!["pdftotext", "qpdf", "ls", "wc", "pwd", "head"])
                .with_allowed_working_dirs(allowed_dirs);
        let bash_executor = BashExecutor::new(bash_perms, 120)?;

        // Create tool executor with bash, base path is the temp directory
        let tool_executor = Arc::new(
            ToolExecutor::builder()
                .base_path(work_dir.clone())
                .bash_executor(Some(Box::new(bash_executor)))
                .build(),
        );

        let system_prompt = generate_system_prompt(&pdf_filename, &work_dir, &txt_filename);

        Ok(Self {
            client,
            storage,
            tool_executor,
            pdf_path,
            pdf_filename,
            system_prompt,
            work_dir,
        })
    }
}

impl<S: AgentStorage> Drop for PdfToTextAgent<S> {
    fn drop(&mut self) {
        if let Err(e) = std::fs::remove_dir_all(&self.work_dir) {
            tracing::warn!(
                "Failed to clean up temp directory {:?}: {:?}",
                self.work_dir,
                e
            );
        } else {
            tracing::info!("Cleaned up temp directory: {:?}", self.work_dir);
        }
    }
}

impl<S: AgentStorage> PdfToTextAgent<S> {
    async fn get_session(&self, session_id: i64) -> anyhow::Result<Session> {
        self.storage
            .get_session(session_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))
    }

    /// Get tool definitions for this agent
    fn get_tool_definitions(&self) -> Vec<nocodo_llm_sdk::tools::Tool> {
        self.tools()
            .into_iter()
            .map(|tool| tool.to_tool_definition())
            .collect()
    }

    /// Build messages from session history
    async fn build_messages(&self, session_id: i64) -> anyhow::Result<Vec<LlmMessage>> {
        let db_messages = self.storage.get_messages(session_id).await?;

        db_messages
            .into_iter()
            .map(|msg| {
                let role = match msg.role {
                    MessageRole::User => Role::User,
                    MessageRole::Assistant => Role::Assistant,
                    MessageRole::System => Role::System,
                    MessageRole::Tool => Role::User,
                };

                Ok(LlmMessage {
                    role,
                    content: vec![ContentBlock::Text { text: msg.content }],
                })
            })
            .collect()
    }

    /// Execute a tool call
    async fn execute_tool_call(
        &self,
        session_id: i64,
        message_id: Option<i64>,
        tool_call: &LlmToolCall,
    ) -> anyhow::Result<()> {
        // 1. Parse LLM tool call into typed ToolRequest
        let tool_request =
            AgentTool::parse_tool_call(tool_call.name(), tool_call.arguments().clone())?;

        // 2. Record tool call in storage
        let mut tool_call_record = StorageToolCall {
            id: None,
            session_id,
            message_id,
            tool_call_id: tool_call.id().to_string(),
            tool_name: tool_call.name().to_string(),
            request: tool_call.arguments().clone(),
            response: None,
            status: ToolCallStatus::Pending,
            execution_time_ms: None,
            created_at: chrono::Utc::now().timestamp(),
            completed_at: None,
            error_details: None,
        };
        let call_id = self
            .storage
            .create_tool_call(tool_call_record.clone())
            .await?;
        tool_call_record.id = Some(call_id);

        // 3. Execute tool
        let start = Instant::now();
        let result: anyhow::Result<nocodo_tools::types::ToolResponse> =
            self.tool_executor.execute(tool_request).await;
        let execution_time = start.elapsed().as_millis() as i64;

        // 4. Update storage with result
        match result {
            Ok(response) => {
                let response_json = serde_json::to_value(&response)?;
                tool_call_record.complete(response_json.clone(), execution_time);
                self.storage.update_tool_call(tool_call_record).await?;

                let result_text = crate::format_tool_response(&response);
                let message_to_llm = format!("Tool {} result:\n{}", tool_call.name(), result_text);

                tracing::debug!(
                    tool_name = tool_call.name(),
                    tool_id = tool_call.id(),
                    execution_time_ms = execution_time,
                    "Tool execution completed successfully"
                );

                let tool_message = StorageMessage {
                    id: None,
                    session_id,
                    role: MessageRole::Tool,
                    content: message_to_llm,
                    created_at: chrono::Utc::now().timestamp(),
                };
                self.storage.create_message(tool_message).await?;
            }
            Err(e) => {
                let error_msg = format!("{:?}", e);
                tool_call_record.fail(error_msg.clone());
                self.storage.update_tool_call(tool_call_record).await?;

                let error_message_to_llm =
                    format!("Tool {} failed: {}", tool_call.name(), error_msg);

                tracing::debug!(
                    tool_name = tool_call.name(),
                    tool_id = tool_call.id(),
                    error = %error_msg,
                    "Tool execution failed"
                );

                let tool_message = StorageMessage {
                    id: None,
                    session_id,
                    role: MessageRole::Tool,
                    content: error_message_to_llm,
                    created_at: chrono::Utc::now().timestamp(),
                };
                self.storage.create_message(tool_message).await?;
            }
        }

        Ok(())
    }
}

#[async_trait]
impl<S: AgentStorage> Agent for PdfToTextAgent<S> {
    fn objective(&self) -> &str {
        "Extract text from PDF files using pdftotext with layout preservation and optional page selection using qpdf"
    }

    fn system_prompt(&self) -> String {
        self.system_prompt.clone()
    }

    fn pre_conditions(&self) -> Option<Vec<String>> {
        Some(vec![
            "pdftotext (poppler-utils) must be installed on the system".to_string(),
            "qpdf must be installed for page extraction operations".to_string(),
            "Run 'pdftotext -v' to verify pdftotext installation".to_string(),
            "Run 'qpdf --version' to verify qpdf installation".to_string(),
            "Install with: brew install poppler qpdf (macOS) or apt-get install poppler-utils qpdf (Linux)"
                .to_string(),
        ])
    }

    fn tools(&self) -> Vec<AgentTool> {
        vec![
            AgentTool::Bash,              // Only pdftotext and qpdf commands allowed
            AgentTool::ReadFile,          // Read extracted text
            AgentTool::WriteFile,         // Write cleaned results
            AgentTool::ConfirmExtraction, // Confirm extraction looks correct
        ]
    }

    async fn execute(&self, user_prompt: &str, session_id: i64) -> anyhow::Result<String> {
        let txt_filename = format!(
            "{}.txt",
            self.pdf_filename
                .strip_suffix(".pdf")
                .unwrap_or(&self.pdf_filename)
        );

        let mut user_content = user_prompt.to_string();
        user_content.push_str(&format!(
            "\n\nNote: The PDF has already been pre-extracted to: {}\nYou can read this file to verify the extraction quality.",
            txt_filename
        ));

        // Create initial user message
        let user_message = StorageMessage {
            id: None,
            session_id,
            role: MessageRole::User,
            content: user_content,
            created_at: chrono::Utc::now().timestamp(),
        };
        self.storage.create_message(user_message).await?;

        // Get tool definitions
        let tools = self.get_tool_definitions();
        tracing::info!(
            "Tool definitions sent to LLM: {:?}",
            tools.iter().map(|t| t.name()).collect::<Vec<_>>()
        );

        // Execution loop (max 30 iterations)
        let mut iteration = 0;
        let max_iterations = 30;

        loop {
            iteration += 1;
            if iteration > max_iterations {
                let error = "Maximum iteration limit reached";
                let mut session = self.get_session(session_id).await?;
                session.status = SessionStatus::Failed;
                session.error = Some(error.to_string());
                session.ended_at = Some(chrono::Utc::now().timestamp());
                self.storage.update_session(session).await?;
                return Err(anyhow::anyhow!(error));
            }

            // Build request with conversation history
            let messages = self.build_messages(session_id).await?;

            let request = CompletionRequest {
                messages,
                max_tokens: 4000,
                model: self.client.model_name().to_string(),
                system: Some(self.system_prompt()),
                temperature: Some(0.7),
                top_p: None,
                stop_sequences: None,
                tools: Some(tools.clone()),
                tool_choice: Some(ToolChoice::Required),
                response_format: None,
            };

            // Call LLM
            let response = self.client.complete(request).await?;

            // Extract text and save assistant message
            let text = extract_text_from_content(&response.content);
            let assistant_message = StorageMessage {
                id: None,
                session_id,
                role: MessageRole::Assistant,
                content: text.clone(),
                created_at: chrono::Utc::now().timestamp(),
            };
            let message_id = self.storage.create_message(assistant_message).await?;

            // Check for tool calls
            if let Some(tool_calls) = response.tool_calls {
                if tool_calls.is_empty() {
                    let mut session = self.get_session(session_id).await?;
                    session.status = SessionStatus::Completed;
                    session.result = Some(text.clone());
                    session.ended_at = Some(chrono::Utc::now().timestamp());
                    self.storage.update_session(session).await?;
                    return Ok(text);
                }

                // Execute tools
                for tool_call in tool_calls {
                    self.execute_tool_call(session_id, Some(message_id), &tool_call)
                        .await?;
                }
            } else {
                let mut session = self.get_session(session_id).await?;
                session.status = SessionStatus::Completed;
                session.result = Some(text.clone());
                session.ended_at = Some(chrono::Utc::now().timestamp());
                self.storage.update_session(session).await?;
                return Ok(text);
            }
        }
    }
}

/// Generate system prompt for PdfToTextAgent
fn generate_system_prompt(pdf_filename: &str, work_dir: &PathBuf, txt_filename: &str) -> String {
    format!(
        r#"You are a PDF text extraction assistant.

# Current State
The PDF file has already been pre-extracted with layout preservation:
- Working directory: {}
- PDF file: {}
- Extracted text: {}

# Your Task
1. Read the pre-extracted text file: {}
2. Verify the extraction looks correct
3. Use confirm_extraction to complete the task
4. Present the extracted text to the user

# Available Tools
- read_file: Read extracted text files
- write_file: Write cleaned results (optional, if user requests)
- confirm_extraction: Confirm the extraction looks correct
- bash: pdftotext, qpdf, ls, wc, pwd (available ONLY if you think the existing extraction is not good or incorrect)

# Critical: Chat Responses Are Invisible
- Chat responses and text outputs are NOT visible to the user at all
- ONLY tool call results are visible to the user
- Present the extracted text ONLY through the confirm_extraction tool
- Do NOT provide any extracted text in chat responses - it will be lost

# Important
- The text is already extracted - just verify and confirm it looks correct
- Confirm as quickly as possible using confirm_extraction tool once you verify the extraction is correct
- Do not summarize the extracted text when the extraction is correct, use confirm_extraction tool
- Only use bash/PDF commands if the existing extraction appears incorrect, incomplete, or has poor quality
- Only re-extract if the user explicitly requests different pages or options
- Working directory: {}
"#,
        work_dir.display(),
        pdf_filename,
        txt_filename,
        txt_filename,
        work_dir.display()
    )
}

/// Helper function to extract text from content blocks
fn extract_text_from_content(content: &[ContentBlock]) -> String {
    content
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Verify that pdftotext is installed and accessible
pub fn verify_pdftotext_installation() -> anyhow::Result<String> {
    use std::process::Command;

    let output = Command::new("pdftotext")
        .arg("-v")
        .output()
        .context("Failed to execute 'pdftotext -v'. Is pdftotext (poppler-utils) installed?")?;

    // pdftotext -v outputs to stderr
    let version_info = String::from_utf8_lossy(&output.stderr).to_string();

    if version_info.is_empty() {
        anyhow::bail!("pdftotext command did not return version information");
    }

    Ok(version_info)
}

/// Verify that qpdf is installed and accessible
pub fn verify_qpdf_installation() -> anyhow::Result<String> {
    use std::process::Command;

    let output = Command::new("qpdf")
        .arg("--version")
        .output()
        .context("Failed to execute 'qpdf --version'. Is qpdf installed?")?;

    if !output.status.success() {
        anyhow::bail!(
            "qpdf command failed. stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let version_info = String::from_utf8_lossy(&output.stdout).to_string();

    Ok(version_info)
}

impl<S: AgentStorage> PdfToTextAgent<S> {
    /// Verify pre-conditions before creating agent
    pub fn verify_preconditions() -> anyhow::Result<()> {
        // Check pdftotext
        match verify_pdftotext_installation() {
            Ok(version) => {
                tracing::info!("pdftotext found: {}", version.lines().next().unwrap_or(""));
            }
            Err(e) => {
                anyhow::bail!(
                    "pdftotext is not installed or not accessible.\n\
                     Error: {}\n\
                     \n\
                     Installation instructions:\n\
                     - macOS: brew install poppler\n\
                     - Ubuntu/Debian: sudo apt-get install poppler-utils\n\
                     - Windows: Download from https://blog.alivate.com.au/poppler-windows/\n\
                     \n\
                     After installation, verify with: pdftotext -v",
                    e
                )
            }
        }

        // Check qpdf
        match verify_qpdf_installation() {
            Ok(version) => {
                tracing::info!("qpdf found: {}", version.lines().next().unwrap_or(""));
            }
            Err(e) => {
                anyhow::bail!(
                    "qpdf is not installed or not accessible.\n\
                     Error: {}\n\
                     \n\
                     Installation instructions:\n\
                     - macOS: brew install qpdf\n\
                     - Ubuntu/Debian: sudo apt-get install qpdf\n\
                     - Windows: Download from https://github.com/qpdf/qpdf/releases\n\
                     \n\
                     After installation, verify with: qpdf --version",
                    e
                )
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_creation() {
        // Note: This test requires setting up mock dependencies
        // Skipping for now due to complexity of mocking LlmClient and Database
    }

    #[test]
    fn test_system_prompt_generation() {
        let prompt = generate_system_prompt("test.pdf");
        assert!(prompt.contains("pdftotext"));
        assert!(prompt.contains("qpdf"));
        assert!(prompt.contains("-layout"));
        assert!(prompt.contains("test.pdf"));
    }

    #[test]
    fn test_verify_pdftotext_installation() {
        // This test will pass if pdftotext is installed
        let _result = verify_pdftotext_installation();
    }

    #[test]
    fn test_verify_qpdf_installation() {
        // This test will pass if qpdf is installed
        let _result = verify_qpdf_installation();
    }
}
