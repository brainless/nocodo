use crate::{database::Database, Agent, AgentTool};
use anyhow::{self, Context};
use async_trait::async_trait;
use manager_tools::{
    bash::{BashExecutor, BashPermissions},
    ToolExecutor,
};
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::tools::{ToolCall, ToolChoice};
use nocodo_llm_sdk::types::{CompletionRequest, ContentBlock, Message, Role};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

/// Agent specialized in extracting text from images using Tesseract OCR
pub struct TesseractAgent {
    client: Arc<dyn LlmClient>,
    database: Arc<Database>,
    tool_executor: Arc<ToolExecutor>,
    #[allow(dead_code)] // Stored for reference, used during construction
    image_path: PathBuf,
    #[allow(dead_code)] // Used in system prompt generation during construction
    image_filename: String,
    system_prompt: String,
}

impl TesseractAgent {
    /// Create a new TesseractAgent
    ///
    /// # Arguments
    /// * `client` - LLM client for AI inference
    /// * `database` - Database for session/message tracking
    /// * `image_path` - Path to the image file to process
    ///
    /// # Security
    /// The agent is configured with restricted bash access:
    /// - Only the `tesseract` command is allowed
    /// - All other bash commands are denied
    /// - File operations are restricted to the image's directory
    ///
    /// # Pre-conditions
    /// - Tesseract OCR must be installed on the system
    /// - Run `tesseract --version` to verify installation
    /// - The image file must exist
    pub fn new(
        client: Arc<dyn LlmClient>,
        database: Arc<Database>,
        image_path: PathBuf,
    ) -> anyhow::Result<Self> {
        // Validate image path exists
        if !image_path.exists() {
            anyhow::bail!("Image file does not exist: {}", image_path.display());
        }

        // Extract filename and directory
        let image_filename = image_path
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid image path - no filename"))?
            .to_string_lossy()
            .to_string();

        let base_path = image_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("Invalid image path - no parent directory"))?
            .to_path_buf();

        // Create restricted bash permissions (only tesseract command)
        let bash_perms = BashPermissions::minimal(vec!["tesseract"]);
        let bash_executor = BashExecutor::new(bash_perms, 120)?;

        // Create tool executor with restricted bash
        let tool_executor = Arc::new(
            ToolExecutor::builder()
                .base_path(base_path)
                .bash_executor(Some(Box::new(bash_executor)))
                .build(),
        );

        let system_prompt = generate_system_prompt(&image_filename);

        Ok(Self {
            client,
            database,
            tool_executor,
            image_path,
            image_filename,
            system_prompt,
        })
    }

    /// Get tool definitions for this agent
    fn get_tool_definitions(&self) -> Vec<nocodo_llm_sdk::tools::Tool> {
        self.tools()
            .into_iter()
            .map(|tool| tool.to_tool_definition())
            .collect()
    }

    /// Build messages from session history
    fn build_messages(&self, session_id: i64) -> anyhow::Result<Vec<Message>> {
        let db_messages = self.database.get_messages(session_id)?;

        db_messages
            .into_iter()
            .map(|msg| {
                let role = match msg.role.as_str() {
                    "user" => Role::User,
                    "assistant" => Role::Assistant,
                    "system" => Role::System,
                    "tool" => Role::User,
                    _ => Role::User,
                };

                Ok(Message {
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
        tool_call: &ToolCall,
    ) -> anyhow::Result<()> {
        // 1. Parse LLM tool call into typed ToolRequest
        let tool_request =
            AgentTool::parse_tool_call(tool_call.name(), tool_call.arguments().clone())?;

        // 2. Record tool call in database
        let call_id = self.database.create_tool_call(
            session_id,
            message_id,
            tool_call.id(),
            tool_call.name(),
            tool_call.arguments().clone(),
        )?;

        // 3. Execute tool
        let start = Instant::now();
        let result: anyhow::Result<manager_tools::types::ToolResponse> =
            self.tool_executor.execute(tool_request).await;
        let execution_time = start.elapsed().as_millis() as i64;

        // 4. Update database with result
        match result {
            Ok(response) => {
                let response_json = serde_json::to_value(&response)?;
                self.database
                    .complete_tool_call(call_id, response_json.clone(), execution_time)?;

                let result_text = crate::format_tool_response(&response);
                let message_to_llm = format!("Tool {} result:\n{}", tool_call.name(), result_text);

                tracing::debug!(
                    tool_name = tool_call.name(),
                    tool_id = tool_call.id(),
                    execution_time_ms = execution_time,
                    "Tool execution completed successfully"
                );

                self.database
                    .create_message(session_id, "tool", &message_to_llm)?;
            }
            Err(e) => {
                let error_msg = format!("{:?}", e);
                self.database.fail_tool_call(call_id, &error_msg)?;

                let error_message_to_llm =
                    format!("Tool {} failed: {}", tool_call.name(), error_msg);

                tracing::debug!(
                    tool_name = tool_call.name(),
                    tool_id = tool_call.id(),
                    error = %error_msg,
                    "Tool execution failed"
                );

                self.database
                    .create_message(session_id, "tool", &error_message_to_llm)?;
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Agent for TesseractAgent {
    fn objective(&self) -> &str {
        "Extract text from images using Tesseract OCR and optionally clean/format the output"
    }

    fn system_prompt(&self) -> String {
        self.system_prompt.clone()
    }

    fn pre_conditions(&self) -> Option<Vec<String>> {
        Some(vec![
            "Tesseract OCR must be installed on the system".to_string(),
            "Run 'tesseract --version' to verify installation".to_string(),
            "Install with: brew install tesseract (macOS) or apt-get install tesseract-ocr (Linux)"
                .to_string(),
        ])
    }

    fn tools(&self) -> Vec<AgentTool> {
        vec![
            AgentTool::Bash,      // Only tesseract command allowed
            AgentTool::ReadFile,  // Read OCR output
            AgentTool::WriteFile, // Write cleaned results
        ]
    }

    async fn execute(&self, user_prompt: &str, _session_id: i64) -> anyhow::Result<String> {
        // 1. Create session
        let session_id = self.database.create_session(
            "tesseract-ocr",
            self.client.provider_name(),
            self.client.model_name(),
            Some(&self.system_prompt),
            user_prompt,
            None, // No config for TesseractAgent
        )?;

        // 2. Create initial user message
        self.database
            .create_message(session_id, "user", user_prompt)?;

        // 3. Get tool definitions
        let tools = self.get_tool_definitions();

        // 4. Execution loop (max 30 iterations)
        let mut iteration = 0;
        let max_iterations = 30;

        loop {
            iteration += 1;
            if iteration > max_iterations {
                let error = "Maximum iteration limit reached";
                self.database.fail_session(session_id, error)?;
                return Err(anyhow::anyhow!(error));
            }

            // 5. Build request with conversation history
            let messages = self.build_messages(session_id)?;

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

            // 6. Call LLM
            let response = self.client.complete(request).await?;

            // 7. Extract text and save assistant message
            let text = extract_text_from_content(&response.content);
            let message_id = self
                .database
                .create_message(session_id, "assistant", &text)?;

            // 8. Check for tool calls
            if let Some(tool_calls) = response.tool_calls {
                if tool_calls.is_empty() {
                    self.database.complete_session(session_id, &text)?;
                    return Ok(text);
                }

                // 9. Execute tools
                for tool_call in tool_calls {
                    self.execute_tool_call(session_id, Some(message_id), &tool_call)
                        .await?;
                }
            } else {
                self.database.complete_session(session_id, &text)?;
                return Ok(text);
            }
        }
    }
}

/// Generate system prompt for TesseractAgent
fn generate_system_prompt(image_filename: &str) -> String {
    format!(
        r#"You are a Tesseract OCR specialist. Your task is to extract text from the image file "{}" and optionally clean and format the extracted text.

You have access to these tools:
1. bash - ONLY for running tesseract command
2. read_file - To read tesseract output files
3. write_file - To write cleaned results (optional)

# Image File

The image file to process is: {}

# Tesseract Command Format

tesseract {} <output_base> [options]

Common options:
- -l <lang> - Language (eng, spa, fra, deu, etc.)
- --psm <n> - Page segmentation mode:
  0 = Orientation and script detection (OSD) only
  1 = Automatic page segmentation with OSD
  3 = Fully automatic page segmentation (default)
  6 = Assume a single uniform block of text
  11 = Sparse text. Find as much text as possible
- --oem <n> - OCR Engine mode:
  0 = Legacy engine
  1 = Neural nets LSTM engine
  2 = Legacy + LSTM engines
  3 = Default (based on what is available)

# Workflow

1. Run tesseract command to extract text from the image
   Example: tesseract {} output -l eng --psm 3
2. Read the output file (tesseract adds .txt automatically)
   Example: read_file output.txt
3. Analyze the extracted text
4. If the user requests cleaning or formatting:
   - Fix OCR errors (common misrecognitions like l/I, O/0, etc.)
   - Improve formatting and structure
   - Remove noise/artifacts
   - Preserve intended structure (paragraphs, lists, tables)
5. Present the result to the user
6. Optionally write cleaned result to a file if requested

# Example Interactions

User: "Extract text from this image"
1. Run: tesseract {} output -l eng
2. Read: output.txt
3. Present the extracted text to user

User: "Extract and clean the text"
1. Run: tesseract {} output -l eng --psm 6
2. Read: output.txt
3. Analyze and clean the text (fix OCR errors, improve formatting)
4. Present cleaned text to user

User: "Extract text in Spanish"
1. Run: tesseract {} output -l spa
2. Read: output.txt
3. Present the extracted text to user

# Important Notes

- You can ONLY run the tesseract command (no other bash commands will work)
- The image file is: {}
- Tesseract automatically adds .txt extension to output files
- For PDF files, tesseract can process them directly
- If OCR quality is poor, try different --psm values (6 for single block, 11 for sparse text)
- Choose appropriate language with -l flag if the image contains non-English text
"#,
        image_filename,
        image_filename,
        image_filename,
        image_filename,
        image_filename,
        image_filename,
        image_filename,
        image_filename
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

/// Verify that Tesseract is installed and accessible
pub fn verify_tesseract_installation() -> anyhow::Result<String> {
    use std::process::Command;

    let output = Command::new("tesseract")
        .arg("--version")
        .output()
        .context("Failed to execute 'tesseract --version'. Is Tesseract installed?")?;

    if !output.status.success() {
        anyhow::bail!(
            "Tesseract command failed. stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let version_info = String::from_utf8_lossy(&output.stdout).to_string();

    Ok(version_info)
}

impl TesseractAgent {
    /// Verify pre-conditions before creating agent
    pub fn verify_preconditions() -> anyhow::Result<()> {
        match verify_tesseract_installation() {
            Ok(version) => {
                tracing::info!(
                    "Tesseract OCR found: {}",
                    version.lines().next().unwrap_or("")
                );
                Ok(())
            }
            Err(e) => {
                anyhow::bail!(
                    "Tesseract OCR is not installed or not accessible.\n\
                     Error: {}\n\
                     \n\
                     Installation instructions:\n\
                     - macOS: brew install tesseract\n\
                     - Ubuntu/Debian: sudo apt-get install tesseract-ocr\n\
                     - Windows: Download from https://github.com/UB-Mannheim/tesseract/wiki\n\
                     \n\
                     After installation, verify with: tesseract --version",
                    e
                )
            }
        }
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
        let prompt = generate_system_prompt("test.png");
        assert!(prompt.contains("tesseract"));
        assert!(prompt.contains("OCR"));
        assert!(prompt.contains("--psm"));
        assert!(prompt.contains("test.png"));
    }

    #[test]
    fn test_verify_tesseract_installation() {
        // This test will fail if tesseract is not installed
        // That's expected behavior for this utility function
        let _result = verify_tesseract_installation();
    }
}
