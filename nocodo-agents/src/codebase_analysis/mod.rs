use crate::{database::Database, Agent, AgentTool};
use anyhow;
use async_trait::async_trait;
use manager_tools::ToolExecutor;
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::tools::ToolCall;
use nocodo_llm_sdk::tools::ToolChoice;
use nocodo_llm_sdk::types::{CompletionRequest, ContentBlock, Message, Role};
use std::sync::Arc;
use std::time::Instant;

#[cfg(test)]
mod tests;

/// Agent specialized in analyzing codebase structure and identifying architectural patterns
pub struct CodebaseAnalysisAgent {
    client: Arc<dyn LlmClient>,
    database: Arc<Database>,
    tool_executor: Arc<ToolExecutor>,
}

impl CodebaseAnalysisAgent {
    /// Create a new CodebaseAnalysisAgent with the given components
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

    /// Get tool definitions for this agent
    fn get_tool_definitions(&self) -> Vec<nocodo_llm_sdk::tools::Tool> {
        self.tools()
            .into_iter()
            .map(|tool| tool.to_tool_definition())
            .collect()
    }
}

#[async_trait]
impl Agent for CodebaseAnalysisAgent {
    fn objective(&self) -> &str {
        "Analyze codebase structure and identify architectural patterns"
    }

    fn system_prompt(&self) -> String {
        "You are a codebase analysis expert. Your role is to examine code repositories, \
         understand their structure, identify architectural patterns, and provide clear insights \
         about the codebase organization. You should analyze file structures, dependencies, \
         design patterns, and architectural decisions."
            .to_string()
    }

    fn tools(&self) -> Vec<AgentTool> {
        vec![AgentTool::ListFiles, AgentTool::ReadFile, AgentTool::Grep]
    }

    async fn execute(&self, user_prompt: &str, session_id: i64) -> anyhow::Result<String> {
        // 1. Create initial user message
        self.database
            .create_message(session_id, "user", user_prompt)?;

        // 3. Get tool definitions
        let tools = self.get_tool_definitions();

        // 4. Execution loop (max 10 iterations)
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
                system: Some(self.system_prompt().to_string()),
                temperature: Some(0.7),
                top_p: None,
                stop_sequences: None,
                tools: Some(tools.clone()),
                tool_choice: Some(ToolChoice::Auto),
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
                    // No more tool calls, we're done
                    self.database.complete_session(session_id, &text)?;
                    return Ok(text);
                }

                // 9. Execute tools
                for tool_call in tool_calls {
                    self.execute_tool_call(session_id, Some(message_id), &tool_call)
                        .await?;
                }

                // Continue loop to send results back to LLM
            } else {
                // No tool calls in response, we're done
                self.database.complete_session(session_id, &text)?;
                return Ok(text);
            }
        }
    }
}

impl CodebaseAnalysisAgent {
    fn build_messages(&self, session_id: i64) -> anyhow::Result<Vec<Message>> {
        let db_messages = self.database.get_messages(session_id)?;

        db_messages
            .into_iter()
            .map(|msg| {
                let role = match msg.role.as_str() {
                    "user" => Role::User,
                    "assistant" => Role::Assistant,
                    "system" => Role::System,
                    "tool" => Role::User, // Tool results sent as user messages
                    _ => Role::User,
                };

                Ok(Message {
                    role,
                    content: vec![ContentBlock::Text { text: msg.content }],
                })
            })
            .collect()
    }

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

        // 3. Execute tool with typed request ✅
        let start = Instant::now();
        let result: anyhow::Result<manager_tools::types::ToolResponse> = self
            .tool_executor
            .execute(tool_request) // ✅ Typed execution
            .await;
        let execution_time = start.elapsed().as_millis() as i64;

        // 4. Update database with typed result
        match result {
            Ok(response) => {
                // Convert ToolResponse to JSON for storage
                let response_json = serde_json::to_value(&response)?;
                self.database
                    .complete_tool_call(call_id, response_json.clone(), execution_time)?;

                // Add tool result as a message for next LLM call
                let result_text = crate::format_tool_response(&response);
                let message_to_llm = format!("Tool {} result:\n{}", tool_call.name(), result_text);

                tracing::debug!(
                    tool_name = tool_call.name(),
                    tool_id = tool_call.id(),
                    execution_time_ms = execution_time,
                    response = %message_to_llm,
                    "Sending tool response to model"
                );

                self.database
                    .create_message(session_id, "tool", &message_to_llm)?;
            }
            Err(e) => {
                let error_msg = format!("{:?}", e);
                self.database.fail_tool_call(call_id, &error_msg)?;

                // Send error back to LLM
                let error_message_to_llm =
                    format!("Tool {} failed: {}", tool_call.name(), error_msg);

                tracing::debug!(
                    tool_name = tool_call.name(),
                    tool_id = tool_call.id(),
                    error = %error_msg,
                    response = %error_message_to_llm,
                    "Sending tool error to model"
                );

                self.database
                    .create_message(session_id, "tool", &error_message_to_llm)?;
            }
        }

        Ok(())
    }
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
