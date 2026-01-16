use crate::{database::Database, Agent, AgentTool};
use anyhow;
use async_trait::async_trait;
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::types::{CompletionRequest, ContentBlock, Message, Role};
use shared_types::user_interaction::AskUserRequest;
use std::sync::Arc;

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
}

impl UserClarificationAgent {
    pub fn new(client: Arc<dyn LlmClient>, database: Arc<Database>) -> Self {
        Self { client, database }
    }

    fn generate_system_prompt() -> String {
        let type_defs = shared_types::generate_typescript_definitions(&[
            "AskUserRequest",
            "UserQuestion",
            "QuestionType",
        ])
        .unwrap_or_else(|_| "// Failed to generate type definitions".to_string());

        format!(
            r#"You are a JSON API that analyzes if user requests need clarification.

Your entire output MUST be a valid JSON object matching this TypeScript type:

<TYPE_DEFINITIONS>
{type_defs}
</TYPE_DEFINITIONS>

Return ONLY the JSON object. No markdown, no code blocks, no explanation text."#
        )
    }

    async fn validate_and_retry(
        &self,
        user_prompt: &str,
        session_id: i64,
        max_retries: u32,
    ) -> anyhow::Result<AskUserRequest> {
        let mut attempt = 0;
        let mut conversation_context = vec![];

        loop {
            attempt += 1;
            if attempt > max_retries {
                return Err(anyhow::anyhow!(
                    "Failed to get valid AskUserRequest after {} attempts",
                    max_retries
                ));
            }

            let messages = self.build_messages(user_prompt, &conversation_context, session_id)?;

            let request = CompletionRequest {
                messages,
                max_tokens: 2000,
                model: self.client.model_name().to_string(),
                system: Some(Self::generate_system_prompt()),
                temperature: Some(0.3),
                top_p: None,
                stop_sequences: None,
                tools: None,
                tool_choice: None,
                response_format: Some(nocodo_llm_sdk::types::ResponseFormat::JsonObject),
            };

            let response = self.client.complete(request).await?;
            let text = extract_text_from_content(&response.content);

            self.database
                .create_message(session_id, "assistant", &text)?;

            match self.parse_response(&text) {
                Ok(ask_user_request) => {
                    return Ok(ask_user_request);
                }
                Err(parse_error) => {
                    tracing::warn!(
                        attempt,
                        error = %parse_error,
                        "JSON parsing failed, retrying"
                    );

                    conversation_context.push((Role::Assistant, text.clone()));

                    let error_msg = format!(
                        "Your response was not valid JSON matching the AskUserRequest type. Error: {}\n\nPlease provide valid JSON matching this structure:\n{{\"questions\": [{{\"id\": \"q1\", \"question\": \"question text\", \"type\": \"text\"}}]}}",
                        parse_error
                    );

                    conversation_context.push((Role::User, error_msg.clone()));
                    self.database
                        .create_message(session_id, "user", &error_msg)?;
                }
            }
        }
    }

    fn build_messages(
        &self,
        user_prompt: &str,
        conversation_context: &[(Role, String)],
        _session_id: i64,
    ) -> anyhow::Result<Vec<Message>> {
        let mut messages = Vec::new();

        for (role, content) in conversation_context {
            messages.push(Message {
                role: role.clone(),
                content: vec![ContentBlock::Text {
                    text: content.clone(),
                }],
            });
        }

        messages.push(Message {
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: format!(
                    "Analyze this user request and determine if clarification is needed:\n\n{}",
                    user_prompt
                ),
            }],
        });

        Ok(messages)
    }

    fn parse_response(&self, response: &str) -> anyhow::Result<AskUserRequest> {
        tracing::debug!("Parsing response: {}", response);

        let parsed: AskUserRequest = serde_json::from_str(response).map_err(|e| {
            anyhow::anyhow!("Failed to parse LLM response as AskUserRequest: {}", e)
        })?;

        parsed
            .validate()
            .map_err(|e| anyhow::anyhow!("Invalid AskUserRequest: {}", e))?;

        Ok(parsed)
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
        vec![]
    }

    async fn execute(&self, user_prompt: &str, session_id: i64) -> anyhow::Result<String> {
        self.database
            .create_message(session_id, "user", user_prompt)?;

        let ask_user_request = self.validate_and_retry(user_prompt, session_id, 3).await?;

        let result = serde_json::to_string_pretty(&ask_user_request)?;
        self.database.complete_session(session_id, &result)?;

        Ok(result)
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
    let agent = UserClarificationAgent::new(client, database.clone());
    Ok((agent, database))
}
