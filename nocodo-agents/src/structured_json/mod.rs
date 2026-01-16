use crate::{database::Database, Agent};
use anyhow;
use async_trait::async_trait;
use manager_tools::ToolExecutor;
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::types::{CompletionRequest, ContentBlock, Message, ResponseFormat, Role};
use std::sync::Arc;

mod validator;
use validator::TypeValidator;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone)]
pub struct StructuredJsonAgentConfig {
    pub type_names: Vec<String>,
    pub domain_description: String,
}

pub struct StructuredJsonAgent {
    client: Arc<dyn LlmClient>,
    database: Arc<Database>,
    #[allow(dead_code)]
    tool_executor: Arc<ToolExecutor>,
    validator: TypeValidator,
    system_prompt: String,
    #[allow(dead_code)]
    config: StructuredJsonAgentConfig,
}

impl StructuredJsonAgent {
    pub fn new(
        client: Arc<dyn LlmClient>,
        database: Arc<Database>,
        tool_executor: Arc<ToolExecutor>,
        config: StructuredJsonAgentConfig,
    ) -> anyhow::Result<Self> {
        let type_names: Vec<&str> = config.type_names.iter().map(|s| s.as_str()).collect();

        let type_definitions = shared_types::generate_typescript_definitions(&type_names)
            .map_err(|e| anyhow::anyhow!("Failed to generate TypeScript definitions: {}", e))?;

        let validator = TypeValidator::new(
            config.type_names.clone(),
            type_definitions
                .lines()
                .filter(|l| !l.is_empty())
                .map(String::from)
                .collect(),
        )?;

        let system_prompt = Self::generate_system_prompt(
            &validator.get_type_definitions(),
            &config.domain_description,
        );

        Ok(Self {
            client,
            database,
            tool_executor,
            validator,
            system_prompt,
            config,
        })
    }

    fn generate_system_prompt(type_defs: &str, domain_desc: &str) -> String {
        format!(
            r#"You are a specialized AI assistant that responds exclusively in structured JSON.

Your responses must conform to one or more of the following TypeScript types:

<TYPE_DEFINITIONS>
{type_defs}
</TYPE_DEFINITIONS>

IMPORTANT RULES:
1. Your entire response must be valid JSON
2. The JSON must match one of the provided TypeScript types exactly
3. Do not include any text outside the JSON structure
4. All required fields must be present
5. Field types must match exactly (string, number, boolean, etc.)
6. Use proper JSON formatting (double quotes, no trailing commas, etc.)
7. Return the JSON object directly, not wrapped in markdown code blocks

Domain: {domain_desc}

When responding:
- Analyze the user's request
- Determine which type(s) best represent the response
- Generate valid JSON matching those types
- Include all required fields with appropriate values
- Return ONLY the JSON, nothing else
"#
        )
    }

    async fn validate_and_retry(
        &self,
        user_prompt: &str,
        session_id: i64,
        max_retries: u32,
    ) -> anyhow::Result<serde_json::Value> {
        let mut attempt = 0;
        let mut conversation_context = vec![];

        loop {
            attempt += 1;
            if attempt > max_retries {
                return Err(anyhow::anyhow!(
                    "Failed to get valid JSON response after {} attempts",
                    max_retries
                ));
            }

            let messages = self.build_messages(user_prompt, &conversation_context, session_id)?;

            let request = CompletionRequest {
                messages,
                max_tokens: 4000,
                model: self.client.model_name().to_string(),
                system: Some(self.system_prompt.clone()),
                temperature: Some(0.3),
                top_p: None,
                stop_sequences: None,
                tools: None,
                tool_choice: None,
                response_format: Some(ResponseFormat::JsonObject),
            };

            let response = self.client.complete(request).await?;

            let text = extract_text_from_content(&response.content);
            self.database
                .create_message(session_id, "assistant", &text)?;

            let json_result = self.validator.validate_json_syntax(&text);

            match json_result {
                Ok(json_value) => match self.validator.validate_structure(&json_value) {
                    Ok(_) => {
                        return Ok(json_value);
                    }
                    Err(validation_error) => {
                        tracing::warn!(
                            attempt,
                            error = %validation_error,
                            "JSON validation failed, retrying"
                        );

                        conversation_context.push((Role::Assistant, text.clone()));

                        let error_msg = format!(
                                "Your response was invalid. Error: {}\n\nPlease fix the JSON to match one of these types: {}",
                                validation_error.message,
                                self.validator.get_expected_types_summary()
                            );

                        conversation_context.push((Role::User, error_msg.clone()));
                        self.database
                            .create_message(session_id, "user", &error_msg)?;
                    }
                },
                Err(syntax_error) => {
                    tracing::warn!(
                        attempt,
                        error = %syntax_error.message,
                        "JSON syntax validation failed, retrying"
                    );

                    conversation_context.push((Role::Assistant, text.clone()));

                    let error_msg = format!(
                        "Your response was not valid JSON. Error: {}\n\nPlease provide valid JSON that matches one of these types: {}",
                        syntax_error.message,
                        self.validator.get_expected_types_summary()
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
                text: user_prompt.to_string(),
            }],
        });

        Ok(messages)
    }
}

#[async_trait]
impl Agent for StructuredJsonAgent {
    fn objective(&self) -> &str {
        "Generate structured JSON responses conforming to specified TypeScript types"
    }

    fn system_prompt(&self) -> String {
        self.system_prompt.clone()
    }

    fn tools(&self) -> Vec<crate::AgentTool> {
        vec![]
    }

    async fn execute(&self, user_prompt: &str, session_id: i64) -> anyhow::Result<String> {
        self.database
            .create_message(session_id, "user", user_prompt)?;

        let json_value = self.validate_and_retry(user_prompt, session_id, 3).await?;

        let formatted = serde_json::to_string_pretty(&json_value)?;

        self.database.complete_session(session_id, &formatted)?;

        Ok(formatted)
    }
}

pub(crate) fn extract_text_from_content(content: &[ContentBlock]) -> String {
    content
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}
