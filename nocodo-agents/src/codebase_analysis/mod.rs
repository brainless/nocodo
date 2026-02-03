use crate::{
    storage::AgentStorage,
    types::{
        Message, MessageRole, Session, SessionStatus, ToolCall as StorageToolCall, ToolCallStatus,
    },
    Agent, AgentTool,
};
use anyhow;
use async_trait::async_trait;
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::tools::ToolCall as LlmToolCall;
use nocodo_llm_sdk::tools::ToolChoice;
use nocodo_llm_sdk::types::{CompletionRequest, ContentBlock, Message as LlmMessage, Role};
use nocodo_tools::ToolExecutor;
use std::sync::Arc;
use std::time::Instant;

#[cfg(test)]
mod tests;

/// Agent specialized in analyzing codebase structure and identifying architectural patterns
pub struct CodebaseAnalysisAgent<S: AgentStorage> {
    client: Arc<dyn LlmClient>,
    storage: Arc<S>,
    tool_executor: Arc<ToolExecutor>,
}

impl<S: AgentStorage> CodebaseAnalysisAgent<S> {
    /// Create a new CodebaseAnalysisAgent with the given components
    pub fn new(
        client: Arc<dyn LlmClient>,
        storage: Arc<S>,
        tool_executor: Arc<ToolExecutor>,
    ) -> Self {
        Self {
            client,
            storage,
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
impl<S: AgentStorage> Agent for CodebaseAnalysisAgent<S> {
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
        let session_id_str = session_id.to_string();
        let user_message = Message {
            id: None,
            session_id: session_id_str.clone(),
            role: MessageRole::User,
            content: user_prompt.to_string(),
            created_at: chrono::Utc::now().timestamp(),
        };
        self.storage.create_message(user_message).await?;

        let tools = self.get_tool_definitions();
        let mut iteration = 0;
        let max_iterations = 30;

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
                max_tokens: 4000,
                model: self.client.model_name().to_string(),
                system: Some(self.system_prompt().to_string()),
                temperature: Some(0.7),
                top_p: None,
                stop_sequences: None,
                tools: Some(tools.clone()),
                tool_choice: Some(ToolChoice::Auto),
                response_format: None,
            };

            let response = self.client.complete(request).await?;

            let text = extract_text_from_content(&response.content);
            let assistant_message = Message {
                id: None,
                session_id: session_id_str.clone(),
                role: MessageRole::Assistant,
                content: text.clone(),
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
                    self.execute_tool_call(&session_id_str, Some(&message_id), &tool_call)
                        .await?;
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

impl<S: AgentStorage> CodebaseAnalysisAgent<S> {
    async fn get_session(&self, session_id: &str) -> anyhow::Result<Session> {
        self.storage
            .get_session(session_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))
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
        let call_id = self
            .storage
            .create_tool_call(tool_call_record.clone())
            .await?;

        let start = Instant::now();
        let result: anyhow::Result<nocodo_tools::types::ToolResponse> =
            self.tool_executor.execute(tool_request).await;
        let execution_time = start.elapsed().as_millis() as i64;

        match result {
            Ok(response) => {
                let response_json = serde_json::to_value(&response)?;
                tool_call_record.complete(response_json, execution_time);
                tool_call_record.id = Some(call_id);
                self.storage.update_tool_call(tool_call_record).await?;

                let result_text = crate::format_tool_response(&response);
                let message_to_llm = format!("Tool {} result:\n{}", tool_call.name(), result_text);

                tracing::debug!(
                    tool_name = tool_call.name(),
                    tool_id = tool_call.id(),
                    execution_time_ms = execution_time,
                    response = %message_to_llm,
                    "Sending tool response to model"
                );

                let tool_message = Message {
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
                tool_call_record.id = Some(call_id);
                self.storage.update_tool_call(tool_call_record).await?;

                let error_message_to_llm =
                    format!("Tool {} failed: {}", tool_call.name(), error_msg);

                tracing::debug!(
                    tool_name = tool_call.name(),
                    tool_id = tool_call.id(),
                    error = %error_msg,
                    response = %error_message_to_llm,
                    "Sending tool error to model"
                );

                let tool_error_message = Message {
                    id: None,
                    session_id: session_id.to_string(),
                    role: MessageRole::Tool,
                    content: error_message_to_llm,
                    created_at: chrono::Utc::now().timestamp(),
                };
                self.storage.create_message(tool_error_message).await?;
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
