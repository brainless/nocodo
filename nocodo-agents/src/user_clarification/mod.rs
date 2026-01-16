use crate::{database::Database, Agent, AgentTool};
use anyhow;
use async_trait::async_trait;
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::types::{CompletionRequest, ContentBlock, Message, Role};
use shared_types::user_interaction::{AskUserRequest, QuestionType};
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
        String::from(
            r#"You are a JSON API that analyzes if user requests need clarification.

Your entire output MUST be a valid JSON object with this field:
- "questions": An array of question objects (empty array if no clarification needed)

Each question must have:
- "id": "q1", "q2", etc.
- "question": The clarifying question
- "type": "text"

Examples:

Input: Build me a website
Output: {"questions": [{"id": "q1", "question": "What is the website's purpose?", "type": "text"}]}

Input: Add 2 plus 2
Output: {"questions": []}

Return ONLY the JSON object. No markdown, no code blocks."#,
        )
    }

    async fn call_llm(&self, user_prompt: &str, session_id: i64) -> anyhow::Result<String> {
        let messages = vec![Message {
            role: Role::User,
            content: vec![ContentBlock::Text {
                text: format!(
                    "Analyze this user request and determine if clarification is needed:\n\n{}",
                    user_prompt
                ),
            }],
        }];

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

        Ok(text)
    }

    fn parse_response(&self, response: &str) -> anyhow::Result<AskUserRequest> {
        let json_str = self.extract_json(response)?;

        let parsed: AskUserRequest = serde_json::from_str(&json_str).map_err(|e| {
            anyhow::anyhow!("Failed to parse LLM response as AskUserRequest: {}", e)
        })?;

        for question in &parsed.questions {
            match question.response_type {
                QuestionType::Text => {}
            }
        }

        parsed
            .validate()
            .map_err(|e| anyhow::anyhow!("Invalid AskUserRequest: {}", e))?;

        Ok(parsed)
    }

    fn extract_json(&self, response: &str) -> anyhow::Result<String> {
        let trimmed = response.trim();

        if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
            return self.convert_to_ask_user(v);
        }

        let markdown_json_pattern = r#"```(?:json)?\s*([\s\S]*?)\s*```"#;
        if let Some(caps) = regex::Regex::new(markdown_json_pattern)
            .unwrap()
            .captures(trimmed)
        {
            if let Some(json_match) = caps.get(1) {
                let json_candidate = json_match.as_str().trim();
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_candidate) {
                    return self.convert_to_ask_user(v);
                }
            }
        }

        let brace_patterns = [r#"\{[\s\S]*\}"#, r#"\{[\s\S]*"questions"[\s\S]*\}"#];

        for pattern in brace_patterns {
            if let Some(caps) = regex::Regex::new(pattern).unwrap().captures(trimmed) {
                let json_candidate = caps.get(0).unwrap().as_str();
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_candidate) {
                    return self.convert_to_ask_user(v);
                }
            }
        }

        Err(anyhow::anyhow!(
            "Could not extract valid JSON from response"
        ))
    }

    fn convert_to_ask_user(&self, value: serde_json::Value) -> anyhow::Result<String> {
        // Handle case where LLM returns {"answer": "..."} format
        if let Some(answer) = value.get("answer") {
            let answer_text = answer.as_str().unwrap_or("");
            let questions = vec![serde_json::json!({
                "id": "q1",
                "question": answer_text,
                "type": "text"
            })];

            let result = serde_json::json!({
                "questions": questions
            });

            return Ok(serde_json::to_string(&result)?);
        }

        // If it has "questions" field, pass through; otherwise wrap it
        if value.get("questions").is_some() {
            Ok(serde_json::to_string(&value)?)
        } else {
            // Unexpected format, wrap empty
            Ok(r#"{"questions":[]}"#.to_string())
        }
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

        let response = self.call_llm(user_prompt, session_id).await?;
        let ask_user_request = self.parse_response(&response)?;

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
