pub mod database;
pub mod models;
pub mod storage;

#[cfg(test)]
mod migrations_test;

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
use nocodo_llm_sdk::tools::{ToolCall as LlmToolCall, ToolChoice};
use nocodo_llm_sdk::types::{CompletionRequest, ContentBlock, Message as LlmMessage, Role};
use nocodo_tools::ToolExecutor;
use std::sync::Arc;
use std::time::Instant;
use storage::RequirementsStorage;

#[cfg(test)]
mod tests;

/// Agent that analyzes user requests and determines if clarification is needed.
///
/// This agent takes user's original prompt and asks the LLM to determine
/// if any clarifying questions are needed before proceeding. It returns an
/// `AskUserRequest` with the clarifying questions, or an empty questions list
/// if no clarification is needed.
pub struct UserClarificationAgent<S: AgentStorage, R: RequirementsStorage> {
    client: Arc<dyn LlmClient>,
    storage: Arc<S>,
    requirements_storage: Arc<R>,
    tool_executor: Arc<ToolExecutor>,
}

impl<S: AgentStorage, R: RequirementsStorage> UserClarificationAgent<S, R> {
    pub fn new(
        client: Arc<dyn LlmClient>,
        storage: Arc<S>,
        requirements_storage: Arc<R>,
        tool_executor: Arc<ToolExecutor>,
    ) -> Self {
        Self {
            client,
            storage,
            requirements_storage,
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
- You can ask clarifying questions using ask_user tool
- You should focus on high-level process understanding, not technical implementation details
- You can ask about data source types/names (not authentication details)
- You can request specific examples (e.g., sample emails, messages to process)
- You should understand goal and desired outcome of automation

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
                    "Tool execution completed successfully"
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
                    "Tool execution failed"
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
}

#[async_trait]
impl<S: AgentStorage, R: RequirementsStorage> Agent for UserClarificationAgent<S, R> {
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

            let assistant_message = Message {
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
                    if tool_call.name() == "ask_user" {
                        tracing::info!(
                            session_id = session_id,
                            "Agent requesting user clarification"
                        );

                        let args_pretty = serde_json::to_string_pretty(tool_call.arguments())
                            .unwrap_or_else(|_| format!("{:?}", tool_call.arguments()));
                        tracing::info!("Raw ask_user tool call arguments:\n{}", args_pretty);

                        let ask_user_request: shared_types::user_interaction::AskUserRequest =
                            serde_json::from_value(tool_call.arguments().clone()).map_err(|e| {
                                tracing::error!(
                                    error = %e,
                                    "Failed to deserialize ask_user request. JSON:\n{}",
                                    args_pretty
                                );
                                e
                            })?;

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
                        let tool_call_id_str = self
                            .storage
                            .create_tool_call(tool_call_record.clone())
                            .await?;
                        let execution_time = start.elapsed().as_millis() as i64;

                        self.requirements_storage
                            .store_questions(
                                &session_id_str,
                                Some(&tool_call_id_str),
                                &ask_user_request.questions,
                            )
                            .await?;

                        let response = serde_json::json!({
                            "status": "questions_stored",
                            "question_count": ask_user_request.questions.len()
                        });
                        tool_call_record.complete(response, execution_time);
                        tool_call_record.id = Some(tool_call_id_str);
                        self.storage.update_tool_call(tool_call_record).await?;

                        let message_to_llm = format!(
                            "Tool {} result:\nStored {} clarification questions. Waiting for user answers.",
                            tool_call.name(),
                            ask_user_request.questions.len()
                        );
                        let tool_message = Message {
                            id: None,
                            session_id: session_id_str.clone(),
                            role: MessageRole::Tool,
                            content: message_to_llm,
                            created_at: chrono::Utc::now().timestamp(),
                        };
                        self.storage.create_message(tool_message).await?;

                        let mut session = self.get_session(&session_id_str).await?;
                        session.status = SessionStatus::WaitingForUserInput;
                        self.storage.update_session(session).await?;

                        return Ok(format!(
                            "Waiting for user to answer {} clarification questions",
                            ask_user_request.questions.len()
                        ));
                    } else {
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
