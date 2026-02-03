pub mod database;
pub mod models;

use crate::{
    storage::AgentStorage,
    types::{
        Message as StorageMessage, MessageRole, Session, SessionStatus,
        ToolCall as StorageToolCall, ToolCallStatus,
    },
    Agent, AgentTool,
};
use anyhow;
use async_trait::async_trait;
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::tools::{ToolCall as LlmToolCall, ToolChoice};
use nocodo_llm_sdk::types::{CompletionRequest, ContentBlock, Message as LlmMessage, Role};
use nocodo_tools::ToolExecutor;
use std::sync::Arc;
use std::time::Instant;

/// Agent that collects and manages settings/variables for workflows
///
/// This agent gathers settings from agents/tools based on their SettingsSchema,
/// collects values from the user using the ask_user tool, and writes them to
/// a TOML settings file.
pub struct SettingsManagementAgent<S: AgentStorage> {
    client: Arc<dyn LlmClient>,
    storage: Arc<S>,
    tool_executor: Arc<ToolExecutor>,
    settings_file_path: std::path::PathBuf,
    agent_schemas: Vec<crate::AgentSettingsSchema>,
}

impl<S: AgentStorage> SettingsManagementAgent<S> {
    pub fn new(
        client: Arc<dyn LlmClient>,
        storage: Arc<S>,
        tool_executor: Arc<ToolExecutor>,
        settings_file_path: std::path::PathBuf,
        agent_schemas: Vec<crate::AgentSettingsSchema>,
    ) -> Self {
        Self {
            client,
            storage,
            tool_executor,
            settings_file_path,
            agent_schemas,
        }
    }

    fn generate_system_prompt(agent_schemas: &[crate::AgentSettingsSchema]) -> String {
        let mut prompt = r#"You are a settings management specialist for workflow automation.
Your role is to collect required settings from the user based on the needs of various
agents and tools, then save them to a TOML settings file.

CONTEXT:
You have access to settings schemas from various agents/tools that define what settings
they need. Your job is to gather these settings from the user and save them to a TOML
file that can be used by the agents.

YOUR CAPABILITIES:
- You can collect settings using the ask_user tool with specific question types:
  * password: For sensitive information like API keys, tokens, passwords
  * file_path: For file or directory paths
  * email: For email addresses
  * url: For web URLs and API endpoints
  * text: For general text settings
  * boolean: For true/false settings
- Settings will be saved to a TOML file with sections for each agent
- You should collect ALL required settings before saving
- You can use default values when provided in the schema

COLLECTION STRATEGY:
1. Review the available agent schemas and identify which agents are relevant to the user's request
2. Gather all required settings for those agents
3. Use ask_user to collect the setting values from the user
   IMPORTANT: When creating questions, use namespaced question IDs in the format "section_name.setting_name"
   For example, if collecting the "db_path" setting for the "sqlite_reader" agent, use ID "sqlite_reader.db_path"
4. Settings will automatically be saved to the TOML file in the correct sections

WHEN TO USE THE ask_user TOOL:
- The user describes a workflow that requires agents with required settings
- You have identified one or more agents that need configuration
- There are settings without default values that the user must provide
- You need to collect sensitive information (API keys, passwords, etc.)

WHEN NOT TO USE THE ask_user TOOL:
- The user's request doesn't involve any agents that need settings
- All required settings have default values and the user hasn't asked to customize them
- The user explicitly states they already have everything configured

"#.to_string();

        if !agent_schemas.is_empty() {
            prompt.push_str("\nAVAILABLE AGENT SCHEMAS:\n");
            for schema in agent_schemas {
                if schema.settings.is_empty() {
                    continue;
                }
                prompt.push_str(&format!(
                    "\n[{}] - {}\n",
                    schema.section_name, schema.agent_name
                ));
                for setting in &schema.settings {
                    let required_label = if setting.required {
                        "required"
                    } else {
                        "optional"
                    };
                    let default_str = setting
                        .default_value
                        .as_ref()
                        .map(|v| format!(", default: {}", v))
                        .unwrap_or_default();
                    prompt.push_str(&format!(
                        "  - {} ({}, {}): {} - {}{}\n",
                        setting.name,
                        format!("{:?}", setting.setting_type).to_lowercase(),
                        required_label,
                        setting.label,
                        setting.description,
                        default_str
                    ));
                }
            }
        }

        prompt.push_str(
            r#"
When the user describes their workflow, identify which agents they need and collect
the required settings. If no settings are needed, explain that no configuration is required.

IMPORTANT: If you identify that the user needs settings, you MUST use the ask_user tool
to collect them. Do not just describe what settings are needed - actively collect them
using the tool."#,
        );

        prompt
    }

    async fn execute_tool_call(
        &self,
        session_id: &str,
        message_id: Option<&String>,
        tool_call: &LlmToolCall,
    ) -> anyhow::Result<()> {
        let tool_request =
            AgentTool::parse_tool_call(tool_call.name(), tool_call.arguments().clone())?;

        let mut tool_call_record = StorageToolCall {
            id: None,
            session_id: session_id.to_string(),
            message_id: message_id.cloned(),
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
        let call_id = self.storage.create_tool_call(tool_call_record.clone()).await?;
        tool_call_record.id = Some(call_id);

        let start = Instant::now();
        let result: anyhow::Result<nocodo_tools::types::ToolResponse> =
            self.tool_executor.execute(tool_request).await;
        let execution_time = start.elapsed().as_millis() as i64;

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
                    session_id: session_id.to_string(),
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
                    session_id: session_id.to_string(),
                    role: MessageRole::Tool,
                    content: error_message_to_llm,
                    created_at: chrono::Utc::now().timestamp(),
                };
                self.storage.create_message(tool_message).await?;
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

    async fn build_messages(&self, session_id: &str) -> anyhow::Result<Vec<LlmMessage>> {
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

    async fn get_session(&self, session_id: &str) -> anyhow::Result<Session> {
        self.storage
            .get_session(session_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))
    }

    /// Write settings to TOML file
    /// This reads existing settings if the file exists, merges new settings into the appropriate
    /// section, and writes back to the file
    fn write_settings_to_toml(
        &self,
        section_name: &str,
        settings: &std::collections::HashMap<String, String>,
    ) -> anyhow::Result<()> {
        use std::io::Write;

        // Read existing TOML if file exists
        let mut toml_value = if self.settings_file_path.exists() {
            let content = std::fs::read_to_string(&self.settings_file_path)?;
            toml::from_str::<toml::Value>(&content)?
        } else {
            toml::Value::Table(toml::map::Map::new())
        };

        // Ensure we have a table structure
        let table = toml_value
            .as_table_mut()
            .ok_or_else(|| anyhow::anyhow!("TOML root must be a table"))?;

        // Get or create the section for this agent
        let section = table
            .entry(section_name.to_string())
            .or_insert_with(|| toml::Value::Table(toml::map::Map::new()))
            .as_table_mut()
            .ok_or_else(|| anyhow::anyhow!("Section must be a table"))?;

        // Add/update settings in the section
        for (key, value) in settings {
            section.insert(key.clone(), toml::Value::String(value.clone()));
        }

        // Write back to file
        let toml_string = toml::to_string_pretty(&toml_value)?;

        // Create parent directory if it doesn't exist
        if let Some(parent) = self.settings_file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut file = std::fs::File::create(&self.settings_file_path)?;
        file.write_all(toml_string.as_bytes())?;

        tracing::info!(
            path = %self.settings_file_path.display(),
            section = section_name,
            settings_count = settings.len(),
            "Wrote settings to TOML file"
        );

        Ok(())
    }
}

#[async_trait]
impl<S: AgentStorage> Agent for SettingsManagementAgent<S> {
    fn objective(&self) -> &str {
        "Collect and manage settings required for workflow automation"
    }

    fn system_prompt(&self) -> String {
        Self::generate_system_prompt(&self.agent_schemas)
    }

    fn tools(&self) -> Vec<AgentTool> {
        vec![AgentTool::AskUser]
    }

    async fn execute(&self, user_prompt: &str, session_id: i64) -> anyhow::Result<String> {
        let session_id_str = session_id.to_string();
        let user_message = StorageMessage {
            id: None,
            session_id: session_id_str.clone(),
            role: MessageRole::User,
            content: user_prompt.to_string(),
            created_at: chrono::Utc::now().timestamp(),
        };
        self.storage.create_message(user_message).await?;

        let tools = self.get_tool_definitions();

        let mut iteration = 0;
        let max_iterations = 10;

        loop {
            iteration += 1;
            if iteration > max_iterations {
                let error = "Maximum iteration limit reached";
                let mut session = self.get_session(&session_id_str).await?;
                session.status = SessionStatus::Failed;
                session.error = Some(error.to_string());
                session.ended_at = Some(chrono::Utc::now().timestamp());
                self.storage.update_session(session).await?;
                return Err(anyhow::anyhow!(error));
            }

            let messages = self.build_messages(&session_id_str).await?;

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
                "[Using tools]".to_string()
            } else {
                text.clone()
            };

            let assistant_message = StorageMessage {
                id: None,
                session_id: session_id_str.clone(),
                role: MessageRole::Assistant,
                content: text_to_save,
                created_at: chrono::Utc::now().timestamp(),
            };
            let message_id = self.storage.create_message(assistant_message).await?;

            if let Some(tool_calls) = response.tool_calls {
                if tool_calls.is_empty() {
                    let mut session = self.get_session(&session_id_str).await?;
                    session.status = SessionStatus::Completed;
                    session.result = Some(text.clone());
                    session.ended_at = Some(chrono::Utc::now().timestamp());
                    self.storage.update_session(session).await?;
                    return Ok(text);
                }

                for tool_call in tool_calls {
                    // Special handling for ask_user tool - collect settings and write to TOML
                    if tool_call.name() == "ask_user" {
                        tracing::info!(session_id = %session_id_str, "Agent requesting user settings");

                        // Create tool call record in agent_tool_calls table
                        let start = Instant::now();
                        let mut tool_call_record = StorageToolCall {
                            id: None,
                            session_id: session_id_str.clone(),
                            message_id: Some(message_id.clone()),
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
                        let tool_call_id = self.storage.create_tool_call(tool_call_record.clone()).await?;
                        tool_call_record.id = Some(tool_call_id);

                        // Execute the ask_user tool to get responses from user
                        let tool_request = crate::AgentTool::parse_tool_call(
                            tool_call.name(),
                            tool_call.arguments().clone(),
                        )?;
                        let tool_response = self.tool_executor.execute(tool_request).await?;
                        let execution_time = start.elapsed().as_millis() as i64;

                        // Extract responses from tool_response
                        if let nocodo_tools::types::ToolResponse::AskUser(ask_user_response) =
                            &tool_response
                        {
                            // Group responses by section name (parsed from question IDs like "section.key")
                            let mut sections: std::collections::HashMap<
                                String,
                                std::collections::HashMap<String, String>,
                            > = std::collections::HashMap::new();

                            for response in &ask_user_response.responses {
                                // Parse question_id to extract section and setting name
                                if let Some((section_name, setting_name)) =
                                    response.question_id.split_once('.')
                                {
                                    sections
                                        .entry(section_name.to_string())
                                        .or_insert_with(std::collections::HashMap::new)
                                        .insert(setting_name.to_string(), response.answer.clone());
                                } else {
                                    // If no section prefix, log a warning and skip
                                    tracing::warn!(
                                        question_id = response.question_id,
                                        "Question ID missing section prefix, skipping"
                                    );
                                }
                            }

                            // Write settings to TOML file, one section at a time
                            for (section_name, settings) in sections {
                                self.write_settings_to_toml(&section_name, &settings)?;
                            }

                            // Mark tool call as completed
                            let response_json = serde_json::to_value(&tool_response)?;
                            tool_call_record.complete(response_json, execution_time);
                            self.storage.update_tool_call(tool_call_record).await?;

                            // Create success message
                            let message_to_llm = format!(
                                "Tool {} result:\nSuccessfully collected and saved {} settings to {}",
                                tool_call.name(),
                                ask_user_response.responses.len(),
                                self.settings_file_path.display()
                            );
                            let tool_message = StorageMessage {
                                id: None,
                                session_id: session_id_str.clone(),
                                role: MessageRole::Tool,
                                content: message_to_llm,
                                created_at: chrono::Utc::now().timestamp(),
                            };
                            self.storage.create_message(tool_message).await?;
                        } else {
                            return Err(anyhow::anyhow!(
                                "Expected AskUser response from ask_user tool"
                            ));
                        }
                    } else {
                        // Execute other tools normally
                        self.execute_tool_call(&session_id_str, Some(&message_id), &tool_call)
                            .await?;
                    }
                }
            } else {
                let mut session = self.get_session(&session_id_str).await?;
                session.status = SessionStatus::Completed;
                session.result = Some(text.clone());
                session.ended_at = Some(chrono::Utc::now().timestamp());
                self.storage.update_session(session).await?;
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

/// Create a SettingsManagementAgent with in-memory storage
pub fn create_settings_management_agent(
    client: Arc<dyn LlmClient>,
    settings_file_path: std::path::PathBuf,
    agent_schemas: Vec<crate::AgentSettingsSchema>,
) -> anyhow::Result<SettingsManagementAgent<crate::storage::InMemoryStorage>> {
    let storage = Arc::new(crate::storage::InMemoryStorage::new());
    let tool_executor = Arc::new(nocodo_tools::ToolExecutor::new(std::path::PathBuf::from(
        ".",
    )));
    let agent = SettingsManagementAgent::new(
        client,
        storage,
        tool_executor,
        settings_file_path,
        agent_schemas,
    );
    Ok(agent)
}
