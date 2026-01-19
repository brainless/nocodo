pub mod database;
pub mod models;

use crate::{database::Database, Agent, AgentTool};
use anyhow;
use async_trait::async_trait;
use manager_tools::ToolExecutor;
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::tools::{ToolCall, ToolChoice};
use nocodo_llm_sdk::types::{CompletionRequest, ContentBlock, Message, Role};
use std::sync::Arc;
use std::time::Instant;

/// Agent that collects and manages settings/variables for workflows
///
/// This agent analyzes user requests and determines what settings or variables
/// are needed for a workflow (like API keys, file paths, URLs, etc.). It uses
/// the ask_user tool to collect these settings and stores them in the database.
/// The agent returns settings that have been collected or prompts for missing ones.
pub struct SettingsManagementAgent {
    client: Arc<dyn LlmClient>,
    database: Arc<Database>,
    tool_executor: Arc<ToolExecutor>,
}

impl SettingsManagementAgent {
    pub fn new(
        client: Arc<dyn LlmClient>,
        database: Arc<Database>,
        tool_executor: Arc<ToolExecutor>,
    ) -> Self {
        Self {
            client,
            database,
            tool_executor,
        }
    }

    fn generate_system_prompt() -> String {
        r#"You are a settings management specialist for workflow automation.
Your role is to analyze user requests and identify what settings, API keys, credentials,
file paths, URLs, or other variables are needed to execute the requested workflow.

CONTEXT:
You are part of a system that helps users set up automated workflows. Users need to
provide various settings and credentials for their automations to work properly.

YOUR CAPABILITIES:
- You can collect settings using the ask_user tool with specific question types:
  * password: For sensitive information like API keys, tokens, passwords
  * file_path: For file or directory paths
  * email: For email addresses
  * url: For web URLs and API endpoints
  * text: For general text settings
- You should identify ALL required settings before proceeding
- You can provide default values when appropriate
- You should explain why each setting is needed

WHEN TO COLLECT SETTINGS:
- The workflow requires API access (API keys, endpoints)
- The workflow needs file system access (paths, directories)
- The workflow requires user credentials or authentication
- The workflow needs email addresses or URLs
- Any external service integration is mentioned

WHEN NOT TO ASK QUESTIONS:
- No external dependencies or settings are needed
- The user has already provided all necessary settings
- The request is purely informational

COLLECTION STRATEGY:
1. Analyze the user's workflow description
2. Identify all required settings and their types
3. Ask for all settings at once using a single ask_user call
4. Store the collected settings for the workflow execution

If the user's request requires no settings, respond directly explaining that
no additional configuration is needed."#
            .to_string()
    }

    async fn execute_tool_call(
        &self,
        session_id: i64,
        message_id: Option<i64>,
        tool_call: &ToolCall,
    ) -> anyhow::Result<()> {
        let tool_request =
            AgentTool::parse_tool_call(tool_call.name(), tool_call.arguments().clone())?;

        let call_id = self.database.create_tool_call(
            session_id,
            message_id,
            tool_call.id(),
            tool_call.name(),
            tool_call.arguments().clone(),
        )?;

        let start = Instant::now();
        let result: anyhow::Result<manager_tools::types::ToolResponse> =
            self.tool_executor.execute(tool_request).await;
        let execution_time = start.elapsed().as_millis() as i64;

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

    fn get_tool_definitions(&self) -> Vec<nocodo_llm_sdk::tools::Tool> {
        self.tools()
            .into_iter()
            .map(|tool| tool.to_tool_definition())
            .collect()
    }

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
}

#[async_trait]
impl Agent for SettingsManagementAgent {
    fn objective(&self) -> &str {
        "Collect and manage settings required for workflow automation"
    }

    fn system_prompt(&self) -> String {
        Self::generate_system_prompt()
    }

    fn tools(&self) -> Vec<AgentTool> {
        vec![AgentTool::AskUser]
    }

    async fn execute(&self, user_prompt: &str, session_id: i64) -> anyhow::Result<String> {
        self.database
            .create_message(session_id, "user", user_prompt)?;

        let tools = self.get_tool_definitions();

        let mut iteration = 0;
        let max_iterations = 10;

        loop {
            iteration += 1;
            if iteration > max_iterations {
                let error = "Maximum iteration limit reached";
                self.database.fail_session(session_id, error)?;
                return Err(anyhow::anyhow!(error));
            }

            let messages = self.build_messages(session_id)?;

            let request = CompletionRequest {
                messages,
                max_tokens: 2000,
                model: self.client.model_name().to_string(),
                system: Some(self.system_prompt()),
                temperature: Some(0.3),
                top_p: None,
                stop_sequences: None,
                tools: Some(tools.clone()),
                tool_choice: Some(ToolChoice::Auto),
                response_format: None,
            };

            let response = self.client.complete(request).await?;

            let text = extract_text_from_content(&response.content);

            let text_to_save = if text.is_empty() && response.tool_calls.is_some() {
                "[Using tools]"
            } else {
                &text
            };

            let message_id = self
                .database
                .create_message(session_id, "assistant", text_to_save)?;

            if let Some(tool_calls) = response.tool_calls {
                if tool_calls.is_empty() {
                    self.database.complete_session(session_id, &text)?;
                    return Ok(text);
                }

                for tool_call in tool_calls {
                    // Special handling for ask_user tool - don't execute, just store settings
                    if tool_call.name() == "ask_user" {
                        tracing::info!(session_id = session_id, "Agent requesting user settings");

                        // Parse the ask_user request
                        let ask_user_request: shared_types::user_interaction::AskUserRequest =
                            serde_json::from_value(tool_call.arguments().clone())?;

                        // Create tool call record in agent_tool_calls table
                        let start = Instant::now();
                        let tool_call_id = self.database.create_tool_call(
                            session_id,
                            Some(message_id),
                            tool_call.id(),
                            tool_call.name(),
                            tool_call.arguments().clone(),
                        )?;
                        let execution_time = start.elapsed().as_millis() as i64;

                        // Store settings in database with reference to tool call
                        self.database.store_settings(
                            session_id,
                            Some(tool_call_id),
                            &ask_user_request.questions,
                        )?;

                        // Mark the tool call as completed with the settings as response
                        let response = serde_json::json!({
                            "status": "settings_stored",
                            "setting_count": ask_user_request.questions.len()
                        });
                        self.database
                            .complete_tool_call(tool_call_id, response, execution_time)?;

                        // Create a tool result message for the conversation
                        let message_to_llm = format!(
                            "Tool {} result:\nStored {} settings. Waiting for user input.",
                            tool_call.name(),
                            ask_user_request.questions.len()
                        );
                        self.database
                            .create_message(session_id, "tool", &message_to_llm)?;

                        // Pause session to wait for user input
                        self.database.pause_session_for_user_input(session_id)?;

                        return Ok(format!(
                            "Waiting for user to provide {} settings",
                            ask_user_request.questions.len()
                        ));
                    } else {
                        // Execute other tools normally
                        self.execute_tool_call(session_id, Some(message_id), &tool_call)
                            .await?;
                    }
                }
            } else {
                self.database.complete_session(session_id, &text)?;
                return Ok(text);
            }
        }
    }
}

/// Extract text content from LLM response content blocks
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

/// Create a SettingsManagementAgent with an in-memory database
pub fn create_settings_management_agent(
    client: Arc<dyn LlmClient>,
) -> anyhow::Result<(SettingsManagementAgent, Arc<Database>)> {
    let database = Arc::new(Database::new(&std::path::PathBuf::from(":memory:"))?);
    let tool_executor = Arc::new(manager_tools::ToolExecutor::new(std::path::PathBuf::from(
        ".",
    )));
    let agent = SettingsManagementAgent::new(client, database.clone(), tool_executor);
    Ok((agent, database))
}
