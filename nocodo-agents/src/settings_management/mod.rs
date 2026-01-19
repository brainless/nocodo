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
/// This agent gathers settings from agents/tools based on their SettingsSchema,
/// collects values from the user using the ask_user tool, and writes them to
/// a TOML settings file.
pub struct SettingsManagementAgent {
    client: Arc<dyn LlmClient>,
    database: Arc<Database>,
    tool_executor: Arc<ToolExecutor>,
    settings_file_path: std::path::PathBuf,
    agent_schemas: Vec<crate::AgentSettingsSchema>,
}

impl SettingsManagementAgent {
    pub fn new(
        client: Arc<dyn LlmClient>,
        database: Arc<Database>,
        tool_executor: Arc<ToolExecutor>,
        settings_file_path: std::path::PathBuf,
        agent_schemas: Vec<crate::AgentSettingsSchema>,
    ) -> Self {
        Self {
            client,
            database,
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
   For example, if collecting the "db_path" setting for the "sqlite_analysis" agent, use ID "sqlite_analysis.db_path"
4. Settings will automatically be saved to the TOML file in the correct sections

"#.to_string();

        if !agent_schemas.is_empty() {
            prompt.push_str("\nAVAILABLE AGENT SCHEMAS:\n");
            for schema in agent_schemas {
                if schema.settings.is_empty() {
                    continue;
                }
                prompt.push_str(&format!("\n[{}] - {}\n", schema.section_name, schema.agent_name));
                for setting in &schema.settings {
                    let required_label = if setting.required { "required" } else { "optional" };
                    let default_str = setting.default_value.as_ref()
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

        prompt.push_str(r#"
When the user describes their workflow, identify which agents they need and collect
the required settings. If no settings are needed, explain that no configuration is required."#);

        prompt
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
        let table = toml_value.as_table_mut()
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
impl Agent for SettingsManagementAgent {
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
                    // Special handling for ask_user tool - collect settings and write to TOML
                    if tool_call.name() == "ask_user" {
                        tracing::info!(session_id = session_id, "Agent requesting user settings");

                        // Create tool call record in agent_tool_calls table
                        let start = Instant::now();
                        let tool_call_id = self.database.create_tool_call(
                            session_id,
                            Some(message_id),
                            tool_call.id(),
                            tool_call.name(),
                            tool_call.arguments().clone(),
                        )?;

                        // Execute the ask_user tool to get responses from user
                        let tool_request = crate::AgentTool::parse_tool_call(
                            tool_call.name(),
                            tool_call.arguments().clone(),
                        )?;
                        let tool_response = self.tool_executor.execute(tool_request).await?;
                        let execution_time = start.elapsed().as_millis() as i64;

                        // Extract responses from tool_response
                        if let manager_tools::types::ToolResponse::AskUser(ask_user_response) = &tool_response {
                            // Group responses by section name (parsed from question IDs like "section.key")
                            let mut sections: std::collections::HashMap<String, std::collections::HashMap<String, String>> =
                                std::collections::HashMap::new();

                            for response in &ask_user_response.responses {
                                // Parse question_id to extract section and setting name
                                if let Some((section_name, setting_name)) = response.question_id.split_once('.') {
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
                            self.database.complete_tool_call(tool_call_id, response_json, execution_time)?;

                            // Create success message
                            let message_to_llm = format!(
                                "Tool {} result:\nSuccessfully collected and saved {} settings to {}",
                                tool_call.name(),
                                ask_user_response.responses.len(),
                                self.settings_file_path.display()
                            );
                            self.database.create_message(session_id, "tool", &message_to_llm)?;
                        } else {
                            return Err(anyhow::anyhow!("Expected AskUser response from ask_user tool"));
                        }
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
    settings_file_path: std::path::PathBuf,
    agent_schemas: Vec<crate::AgentSettingsSchema>,
) -> anyhow::Result<(SettingsManagementAgent, Arc<Database>)> {
    let database = Arc::new(Database::new(&std::path::PathBuf::from(":memory:"))?);
    let tool_executor = Arc::new(manager_tools::ToolExecutor::new(std::path::PathBuf::from(
        ".",
    )));
    let agent = SettingsManagementAgent::new(
        client,
        database.clone(),
        tool_executor,
        settings_file_path,
        agent_schemas,
    );
    Ok((agent, database))
}
