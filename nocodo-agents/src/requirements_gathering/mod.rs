pub mod database;
pub mod models;

#[cfg(test)]
mod migrations_test;

use crate::{database::Database, Agent, AgentTool};
use anyhow;
use async_trait::async_trait;
use manager_tools::ToolExecutor;
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::tools::{ToolCall, ToolChoice};
use nocodo_llm_sdk::types::{CompletionRequest, ContentBlock, Message, Role};
use std::sync::Arc;
use std::time::Instant;

#[cfg(test)]
mod tests;

/// Agent that analyzes user requests and determines if clarification is needed.
///
/// This agent takes the user's original prompt and asks the LLM to determine
/// if any clarifying questions are needed before proceeding. It returns an
/// `AskUserRequest` with the clarifying questions, or an empty questions list
/// if no clarification is needed.
pub struct UserClarificationAgent {
    client: Arc<dyn LlmClient>,
    database: Arc<Database>,
    tool_executor: Arc<ToolExecutor>,
}

impl UserClarificationAgent {
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
        r#"You are a requirements gathering specialist for business process automation.
Your role is to analyze user requests and determine if clarification is needed before implementation.

CONTEXT:
You are part of a system that helps users define their business processes and automate workflows.
Users will share access to their data sources (databases, APIs, etc.) as needed.

YOUR CAPABILITIES:
- You can ask clarifying questions using the ask_user tool
- You should focus on high-level process understanding, not technical implementation details
- You can ask about data source types/names (not authentication details)
- You can request specific examples (e.g., sample emails, messages to process)
- You should understand the goal and desired outcome of the automation

WHEN TO ASK QUESTIONS:
- The user's goal is unclear or ambiguous
- Critical information about data sources is missing
- The scope of the automation needs definition
- Specific examples would help clarify requirements

WHEN NOT TO ASK QUESTIONS:
- The user has provided a clear, actionable request
- The request is not about business process automation
- You have sufficient information to proceed

If the user's request is clear and describes an automatable software process, respond directly
without using the ask_user tool. Explain that you understand the requirements.

If the user did not share a process that can be automated with software, respond politely
that you need more information about what they want to automate."#.to_string()
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
impl Agent for UserClarificationAgent {
    fn objective(&self) -> &str {
        "Analyze user requests and determine if clarification is needed"
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
                    // Special handling for ask_user tool - don't execute, just store questions
                    if tool_call.name() == "ask_user" {
                        tracing::info!(
                            session_id = session_id,
                            "Agent requesting user clarification"
                        );

                        // Log the raw arguments for debugging
                        let args_pretty = serde_json::to_string_pretty(tool_call.arguments())
                            .unwrap_or_else(|_| format!("{:?}", tool_call.arguments()));
                        tracing::info!("Raw ask_user tool call arguments:\n{}", args_pretty);

                        // Parse the ask_user request
                        let ask_user_request: shared_types::user_interaction::AskUserRequest =
                            serde_json::from_value(tool_call.arguments().clone()).map_err(|e| {
                                tracing::error!(
                                    error = %e,
                                    "Failed to deserialize ask_user request. JSON:\n{}",
                                    args_pretty
                                );
                                e
                            })?;

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

                        // Store questions in database with reference to tool call
                        self.database.store_questions(
                            session_id,
                            Some(tool_call_id),
                            &ask_user_request.questions,
                        )?;

                        // Mark the tool call as completed with the questions as response
                        let response = serde_json::json!({
                            "status": "questions_stored",
                            "question_count": ask_user_request.questions.len()
                        });
                        self.database
                            .complete_tool_call(tool_call_id, response, execution_time)?;

                        // Create a tool result message for the conversation
                        let message_to_llm = format!(
                            "Tool {} result:\nStored {} clarification questions. Waiting for user answers.",
                            tool_call.name(),
                            ask_user_request.questions.len()
                        );
                        self.database
                            .create_message(session_id, "tool", &message_to_llm)?;

                        // Pause session to wait for user input
                        self.database.pause_session_for_user_input(session_id)?;

                        return Ok(format!(
                            "Waiting for user to answer {} clarification questions",
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

/// Create a UserClarificationAgent with an in-memory database
pub fn create_user_clarification_agent(
    client: Arc<dyn LlmClient>,
) -> anyhow::Result<(UserClarificationAgent, Arc<Database>)> {
    let database = Arc::new(Database::new(&std::path::PathBuf::from(":memory:"))?);
    let tool_executor = Arc::new(manager_tools::ToolExecutor::new(std::path::PathBuf::from(
        ".",
    )));
    let agent = UserClarificationAgent::new(client, database.clone(), tool_executor);
    Ok((agent, database))
}
