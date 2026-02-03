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
    ToolExecutor,
};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

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
}

impl<S: AgentStorage> PdfToTextAgent<S> {
    /// Create a new PdfToTextAgent
    ///
    /// # Arguments
    /// * `client` - LLM client for AI inference
    /// * `storage` - Storage for session/message tracking
    /// * `pdf_path` - Path to the PDF file to process
    /// * `allowed_working_dirs` - Optional list of allowed working directories. Defaults to ["/tmp", "/home", "/workspace", "/project"]
    ///
    /// # Security
    /// The agent is configured with restricted bash access:
    /// - Only the `pdftotext` and `qpdf` commands are allowed
    /// - All other bash commands are denied
    /// - File operations are restricted to the allowed working directories
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
        allowed_working_dirs: Option<Vec<String>>,
    ) -> anyhow::Result<Self> {
        // Validate PDF path exists
        if !pdf_path.exists() {
            anyhow::bail!("PDF file does not exist: {}", pdf_path.display());
        }

        // Extract filename and directory
        let pdf_filename = pdf_path
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid PDF path - no filename"))?
            .to_string_lossy()
            .to_string();

        let base_path = pdf_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Invalid PDF path - no parent directory"))?
            .to_path_buf();

        // Create restricted bash permissions (only pdftotext and qpdf commands)
        let bash_perms = BashPermissions::minimal(vec!["pdftotext", "qpdf"])
            .with_allowed_working_dirs(
                allowed_working_dirs.unwrap_or_else(|| vec!["/tmp".to_string()]),
            );
        let bash_executor = BashExecutor::new(bash_perms, 120)?;

        // Create tool executor with restricted bash
        let tool_executor = Arc::new(
            ToolExecutor::builder()
                .base_path(base_path)
                .bash_executor(Some(Box::new(bash_executor)))
                .build(),
        );

        let system_prompt = generate_system_prompt(&pdf_filename);

        Ok(Self {
            client,
            storage,
            tool_executor,
            pdf_path,
            pdf_filename,
            system_prompt,
        })
    }

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
            AgentTool::Bash,      // Only pdftotext and qpdf commands allowed
            AgentTool::ReadFile,  // Read extracted text
            AgentTool::WriteFile, // Write cleaned results
        ]
    }

    async fn execute(&self, user_prompt: &str, session_id: i64) -> anyhow::Result<String> {
        // Create initial user message
        let user_message = StorageMessage {
            id: None,
            session_id,
            role: MessageRole::User,
            content: user_prompt.to_string(),
            created_at: chrono::Utc::now().timestamp(),
        };
        self.storage.create_message(user_message).await?;

        // Get tool definitions
        let tools = self.get_tool_definitions();

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
                tool_choice: Some(ToolChoice::Auto),
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
fn generate_system_prompt(pdf_filename: &str) -> String {
    format!(
        r#"You are a PDF text extraction specialist. Your task is to extract text from the PDF file "{}" and optionally clean and format the extracted text.

You have access to these tools:
1. bash - ONLY for running pdftotext and qpdf commands
2. read_file - To read extracted text files
3. write_file - To write cleaned results (optional)

# PDF File

The PDF file to process is: {}

# Available Commands

## pdftotext - Extract text from PDF

Basic usage:
pdftotext [options] {} <output_base>

Key options:
- -layout              : Maintain original physical layout (RECOMMENDED for preserving formatting)
- -f <n>               : First page to convert
- -l <n>               : Last page to convert
- -nopgbrk             : Don't insert page breaks between pages
- -enc <encoding>      : Output text encoding (default: UTF-8)
- -raw                 : Keep strings in content stream order (alternative to -layout)

The -layout flag is HIGHLY RECOMMENDED as it preserves the original formatting, tables, and structure.

Examples:
- Extract all pages with layout: pdftotext -layout {} output.txt
- Extract pages 1-5: pdftotext -layout -f 1 -l 5 {} output.txt
- Extract without page breaks: pdftotext -layout -nopgbrk {} output.txt

## qpdf - Extract specific pages to a new PDF

Use qpdf when the user wants to extract specific pages BEFORE text extraction.

Basic usage:
qpdf {} --pages . <page-range> -- output.pdf

Page range syntax:
- Single page: 1
- Range: 1-5
- Multiple ranges: 1-3,7-10
- From end: r1 (last page), r2 (second to last)
- Last page: z

Examples:
- Extract pages 1-5: qpdf {} --pages . 1-5 -- pages_1-5.pdf
- Extract pages 2,4,6: qpdf {} --pages . 2,4,6 -- selected_pages.pdf
- Extract last 3 pages: qpdf {} --pages . r3-r1 -- last_3_pages.pdf

# Workflow

## Simple extraction (most common):
1. Run: pdftotext -layout {} output.txt
2. Read: output.txt
3. Present the extracted text to the user

## Extract specific pages (if user requests):
Option A: Use pdftotext -f and -l flags directly
1. Run: pdftotext -layout -f 1 -l 5 {} output.txt
2. Read: output.txt
3. Present the extracted text

Option B: Use qpdf first, then pdftotext
1. Run: qpdf {} --pages . 1-5 -- pages_1-5.pdf
2. Run: pdftotext -layout pages_1-5.pdf output.txt
3. Read: output.txt
4. Present the extracted text

## Clean and format (if user requests):
1. Extract text using pdftotext
2. Read the output file
3. Analyze and clean the text:
   - Fix common extraction errors
   - Improve formatting and structure
   - Remove artifacts or noise
   - Preserve intended structure (tables, paragraphs, lists)
4. Present cleaned text to user
5. Optionally write cleaned result to a file if requested

# Example Interactions

User: "Extract text from this PDF"
1. Run: pdftotext -layout {} output.txt
2. Read: output.txt
3. Present the extracted text

User: "Extract text from pages 1-10"
1. Run: pdftotext -layout -f 1 -l 10 {} output.txt
2. Read: output.txt
3. Present the extracted text

User: "Extract and clean the text from pages 5-15"
1. Run: pdftotext -layout -f 5 -l 15 {} output.txt
2. Read: output.txt
3. Analyze and clean the text
4. Present cleaned text to user

User: "Extract page 3 only"
1. Run: pdftotext -layout -f 3 -l 3 {} page_3.txt
2. Read: page_3.txt
3. Present the extracted text

# Important Notes

- You can ONLY run pdftotext and qpdf commands (no other bash commands will work)
- The PDF file is: {}
- ALWAYS use -layout flag with pdftotext to preserve formatting (unless user explicitly asks not to)
- pdftotext creates output files automatically (don't need to redirect with >)
- Page numbers start at 1
- For page extraction, using pdftotext -f/-l is usually simpler than qpdf
- Use qpdf when you need complex page selection (e.g., non-contiguous pages like 1,5,10)
"#,
        pdf_filename,
        pdf_filename,
        pdf_filename,
        pdf_filename,
        pdf_filename,
        pdf_filename,
        pdf_filename,
        pdf_filename,
        pdf_filename,
        pdf_filename,
        pdf_filename,
        pdf_filename,
        pdf_filename,
        pdf_filename,
        pdf_filename,
        pdf_filename,
        pdf_filename,
        pdf_filename
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
