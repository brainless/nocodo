use super::types::*;
use crate::error::LlmError;
use crate::tools::ProviderToolFormat;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};

/// Google Gemini API client
pub struct GeminiClient {
    api_key: String,
    base_url: String,
    http_client: reqwest::Client,
}

impl GeminiClient {
    pub fn new(api_key: impl Into<String>) -> Result<Self, LlmError> {
        let api_key = api_key.into();
        if api_key.is_empty() {
            return Err(LlmError::authentication("API key cannot be empty"));
        }

        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .map_err(|e| LlmError::Network { source: e })?;

        Ok(Self {
            api_key,
            base_url: "https://generativelanguage.googleapis.com".to_string(),
            http_client,
        })
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    pub async fn generate_content(
        &self,
        model: impl Into<String>,
        request: GeminiGenerateContentRequest,
    ) -> Result<GeminiGenerateContentResponse, LlmError> {
        let model = model.into();
        let url = format!("{}/v1beta/models/{}:generateContent", self.base_url, model);

        let mut headers = HeaderMap::new();
        headers.insert(
            "x-goog-api-key",
            HeaderValue::from_str(&self.api_key)
                .map_err(|e| LlmError::authentication(format!("Invalid API key format: {}", e)))?,
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        if should_log_payloads() {
            if let Ok(payload) = serde_json::to_string_pretty(&request) {
                tracing::debug!(
                    target: "nocodo_llm_sdk::gemini",
                    "Gemini request payload: {}",
                    truncate_payload(&payload)
                );
            }
        }

        let response = self
            .http_client
            .post(&url)
            .headers(headers)
            .json(&request)
            .send()
            .await
            .map_err(|e| LlmError::Network { source: e })?;

        let status = response.status();

        if !status.is_success() {
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unable to read error response".to_string());

            if let Ok(error_response) = serde_json::from_str::<GeminiErrorResponse>(&error_body) {
                return Err(Self::map_error(
                    error_response.error.code,
                    error_response.error.message,
                ));
            }

            return Err(LlmError::api_error(status.as_u16(), error_body));
        }

        let generate_response = response
            .json::<GeminiGenerateContentResponse>()
            .await
            .map_err(|e| LlmError::internal(format!("Failed to parse response: {}", e)))?;

        if should_log_payloads() {
            let payload = format!("{generate_response:#?}");
            tracing::debug!(
                target: "nocodo_llm_sdk::gemini",
                "Gemini response payload: {}",
                truncate_payload(&payload)
            );
        }

        Ok(generate_response)
    }

    fn map_error(status: u16, message: String) -> LlmError {
        match status {
            400 => LlmError::invalid_request(message),
            401 | 403 => LlmError::Authentication { message },
            404 => LlmError::api_error(status, message),
            429 => LlmError::rate_limit(message, None),
            500 | 503 => LlmError::api_error(status, message),
            _ => LlmError::api_error(status, message),
        }
    }

    pub fn provider_name(&self) -> &str {
        "Google"
    }

    pub fn message_builder(&self) -> super::builder::MessageBuilder<'_> {
        super::builder::MessageBuilder::new(self)
    }
}

fn should_log_payloads() -> bool {
    std::env::var("NOCODO_LLM_LOG_PAYLOADS")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn truncate_payload(payload: &str) -> String {
    const MAX_LEN: usize = 50_000;
    if payload.len() <= MAX_LEN {
        payload.to_string()
    } else {
        let mut truncated = payload[..MAX_LEN].to_string();
        truncated.push_str("\n...<truncated>...");
        truncated
    }
}

#[async_trait::async_trait]
impl crate::client::LlmClient for GeminiClient {
    async fn complete(
        &self,
        request: crate::types::CompletionRequest,
    ) -> Result<crate::types::CompletionResponse, LlmError> {
        let mut system_text = request.system;
        let mut contents = Vec::new();

        for msg in request.messages {
            match msg.role {
                crate::types::Role::System => {
                    let text = msg
                        .content
                        .into_iter()
                        .map(|block| match block {
                            crate::types::ContentBlock::Text { text } => Ok(text),
                            crate::types::ContentBlock::Image { .. } => Err(
                                LlmError::invalid_request("Image content not supported in v0.1"),
                            ),
                        })
                        .collect::<Result<Vec<String>, LlmError>>()?
                        .join("");

                    if !text.is_empty() {
                        if let Some(existing) = system_text.as_mut() {
                            if !existing.is_empty() {
                                existing.push('\n');
                            }
                            existing.push_str(&text);
                        } else {
                            system_text = Some(text);
                        }
                    }
                }
                crate::types::Role::User | crate::types::Role::Assistant => {
                    let role = match msg.role {
                        crate::types::Role::User => GeminiRole::User,
                        crate::types::Role::Assistant => GeminiRole::Model,
                        crate::types::Role::System => unreachable!(),
                    };

                    let parts = msg
                        .content
                        .into_iter()
                        .map(|block| match block {
                            crate::types::ContentBlock::Text { text } => Ok(GeminiPart {
                                text: Some(text),
                                ..Default::default()
                            }),
                            crate::types::ContentBlock::Image { .. } => Err(
                                LlmError::invalid_request("Image content not supported in v0.1"),
                            ),
                        })
                        .collect::<Result<Vec<GeminiPart>, LlmError>>()?;

                    if parts.is_empty() {
                        return Err(LlmError::invalid_request(
                            "Message content cannot be empty",
                        ));
                    }

                    contents.push(GeminiContent {
                        role: Some(role),
                        parts: Some(parts),
                        text: None,
                    });
                }
            }
        }

        if contents.is_empty() {
            return Err(LlmError::invalid_request(
                "At least one non-system message is required",
            ));
        }

        let tools = request.tools.map(|tools| {
            tools
                .into_iter()
                .map(|tool| super::tools::GeminiToolFormat::to_provider_tool(&tool))
                .collect::<Vec<_>>()
        });

        let tool_config = request.tool_choice.map(|choice| {
            let (mode, allowed_function_names) = match choice {
                crate::tools::ToolChoice::Auto => ("AUTO".to_string(), None),
                crate::tools::ToolChoice::Required => ("ANY".to_string(), None),
                crate::tools::ToolChoice::None => ("NONE".to_string(), None),
                crate::tools::ToolChoice::Specific { name } => {
                    ("ANY".to_string(), Some(vec![name]))
                }
            };

            GeminiToolConfig {
                function_calling_config: GeminiFunctionCallingConfig {
                    mode,
                    allowed_function_names,
                },
            }
        });

        let generation_config = GenerationConfig {
            temperature: request.temperature,
            top_p: request.top_p,
            top_k: None,
            max_output_tokens: Some(request.max_tokens),
            stop_sequences: request.stop_sequences,
            thinking_config: None,
            response_mime_type: request.response_format.map(|rf| match rf {
                crate::types::ResponseFormat::Text => "text/plain".to_string(),
                crate::types::ResponseFormat::JsonObject => "application/json".to_string(),
            }),
            response_json_schema: None,
        };

        let gemini_request = GeminiGenerateContentRequest {
            contents,
            system_instruction: system_text.map(|text| GeminiContent {
                role: Some(GeminiRole::User),
                parts: Some(vec![GeminiPart {
                    text: Some(text),
                    ..Default::default()
                }]),
                text: None,
            }),
            tools,
            tool_config,
            generation_config: Some(generation_config),
        };

        let gemini_response = self.generate_content(request.model, gemini_request).await?;

        let candidate = gemini_response
            .candidates
            .into_iter()
            .next()
            .ok_or_else(|| LlmError::api_error(500, "No candidates returned".to_string()))?;

        let mut content = Vec::new();
        let mut tool_calls = Vec::new();
        let mut tool_call_index = 0;

        if let Some(parts) = candidate.content.parts {
            for part in parts {
                if let Some(text) = part.text {
                    content.push(crate::types::ContentBlock::Text { text });
                }

                if let Some(function_call) = part.function_call {
                    tool_call_index += 1;
                    tool_calls.push(crate::tools::ToolCall::new(
                        format!("gemini-call-{}", tool_call_index),
                        function_call.name,
                        function_call.args,
                    ));
                }
            }
        } else if let Some(text) = candidate.content.text {
            content.push(crate::types::ContentBlock::Text { text });
        }

        let response = crate::types::CompletionResponse {
            content,
            role: crate::types::Role::Assistant,
            usage: crate::types::Usage {
                input_tokens: gemini_response
                    .usage_metadata
                    .as_ref()
                    .and_then(|u| u.prompt_token_count)
                    .unwrap_or(0),
                output_tokens: gemini_response
                    .usage_metadata
                    .as_ref()
                    .and_then(|u| u.candidates_token_count)
                    .unwrap_or(0),
            },
            stop_reason: Some(candidate.finish_reason),
            tool_calls: if tool_calls.is_empty() {
                None
            } else {
                Some(tool_calls)
            },
        };

        Ok(response)
    }

    fn provider_name(&self) -> &str {
        crate::providers::GOOGLE
    }

    fn model_name(&self) -> &str {
        crate::models::gemini::GEMINI_3_PRO_ID
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = GeminiClient::new("test-key");
        assert!(client.is_ok());
    }

    #[test]
    fn test_client_creation_empty_key() {
        let client = GeminiClient::new("");
        assert!(client.is_err());
    }
}
