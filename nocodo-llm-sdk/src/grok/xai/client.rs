use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};

use crate::{
    error::LlmError,
    grok::types::{GrokChatCompletionRequest, GrokChatCompletionResponse, GrokErrorResponse},
};

/// xAI provider for Grok
pub struct XaiGrokClient {
    api_key: String,
    base_url: String,
    http_client: reqwest::Client,
}

impl XaiGrokClient {
    /// Create a new Grok client with the given API key
    pub fn new(api_key: impl Into<String>) -> Result<Self, LlmError> {
        let api_key = api_key.into();
        if api_key.is_empty() {
            return Err(LlmError::authentication("API key cannot be empty"));
        }

        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300)) // 5 minute timeout
            .build()
            .map_err(|e| LlmError::Network { source: e })?;

        Ok(Self {
            api_key,
            base_url: "https://api.x.ai".to_string(),
            http_client,
        })
    }

    /// Set a custom base URL for the API
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Create a chat completion using the Grok Chat Completions API
    pub async fn create_chat_completion(
        &self,
        request: GrokChatCompletionRequest,
    ) -> Result<GrokChatCompletionResponse, LlmError> {
        let url = format!("{}/v1/chat/completions", self.base_url);

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", self.api_key))
                .map_err(|_| LlmError::authentication("Invalid API key format"))?,
        );
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let response = self
            .http_client
            .post(&url)
            .headers(headers)
            .json(&request)
            .send()
            .await
            .map_err(|e| LlmError::Network { source: e })?;

        let status = response.status();

        if status.is_success() {
            let grok_response: GrokChatCompletionResponse = response
                .json()
                .await
                .map_err(|e| LlmError::internal(format!("Failed to parse response: {}", e)))?;
            Ok(grok_response)
        } else {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            // Try to parse as Grok error response
            if let Ok(error_response) = serde_json::from_str::<GrokErrorResponse>(&error_text) {
                match status {
                    reqwest::StatusCode::BAD_REQUEST => {
                        // xAI returns 400 for invalid API keys with "Incorrect API key" in the message
                        if error_response
                            .error
                            .message
                            .to_lowercase()
                            .contains("api key")
                        {
                            Err(LlmError::authentication(error_response.error.message))
                        } else {
                            Err(LlmError::invalid_request(error_response.error.message))
                        }
                    }
                    reqwest::StatusCode::UNAUTHORIZED => {
                        Err(LlmError::authentication(error_response.error.message))
                    }
                    reqwest::StatusCode::FORBIDDEN => {
                        Err(LlmError::authentication(error_response.error.message))
                    }
                    reqwest::StatusCode::NOT_FOUND => {
                        Err(LlmError::api_error(404, error_response.error.message))
                    }
                    reqwest::StatusCode::PAYLOAD_TOO_LARGE => {
                        Err(LlmError::invalid_request("Request too large"))
                    }
                    reqwest::StatusCode::TOO_MANY_REQUESTS => {
                        Err(LlmError::rate_limit(error_response.error.message, None))
                    }
                    reqwest::StatusCode::INTERNAL_SERVER_ERROR => {
                        Err(LlmError::api_error(500, error_response.error.message))
                    }
                    _ => Err(LlmError::api_error(
                        status.as_u16(),
                        error_response.error.message,
                    )),
                }
            } else {
                // Fallback for non-standard error responses
                match status {
                    reqwest::StatusCode::BAD_REQUEST => {
                        // Check if error text contains API key related error
                        if error_text.to_lowercase().contains("api key") {
                            Err(LlmError::authentication(error_text))
                        } else {
                            Err(LlmError::invalid_request(error_text))
                        }
                    }
                    reqwest::StatusCode::UNAUTHORIZED => Err(LlmError::authentication(error_text)),
                    reqwest::StatusCode::FORBIDDEN => Err(LlmError::authentication(error_text)),
                    reqwest::StatusCode::NOT_FOUND => Err(LlmError::api_error(404, error_text)),
                    reqwest::StatusCode::PAYLOAD_TOO_LARGE => {
                        Err(LlmError::invalid_request("Request too large"))
                    }
                    reqwest::StatusCode::TOO_MANY_REQUESTS => {
                        Err(LlmError::rate_limit(error_text, None))
                    }
                    reqwest::StatusCode::INTERNAL_SERVER_ERROR => {
                        Err(LlmError::api_error(500, error_text))
                    }
                    _ => Err(LlmError::api_error(status.as_u16(), error_text)),
                }
            }
        }
    }
}

impl crate::grok::types::GrokChatCompletionResponse {
    /// Extract tool calls from the response
    pub fn tool_calls(&self) -> Option<Vec<crate::tools::ToolCall>> {
        self.choices.first()?.message.tool_calls.as_ref().map(|calls| {
            calls.iter().map(|call| {
                let arguments: serde_json::Value = serde_json::from_str(&call.function.arguments)
                    .unwrap_or(serde_json::Value::Null);

                crate::tools::ToolCall::new(
                    call.id.clone(),
                    call.function.name.clone(),
                    arguments,
                )
            }).collect()
        })
    }
}

impl crate::client::LlmClient for XaiGrokClient {
    async fn complete(
        &self,
        request: crate::types::CompletionRequest,
    ) -> Result<crate::types::CompletionResponse, LlmError> {
        // Convert generic request to Grok-specific request
        let grok_messages = request
            .messages
            .into_iter()
            .map(|msg| {
                let role = match msg.role {
                    crate::types::Role::User => crate::grok::types::GrokRole::User,
                    crate::types::Role::Assistant => crate::grok::types::GrokRole::Assistant,
                    crate::types::Role::System => crate::grok::types::GrokRole::System,
                };

                // For now, only support text content
                let content = msg
                    .content
                    .into_iter()
                    .map(|block| match block {
                        crate::types::ContentBlock::Text { text } => Ok(text),
                        crate::types::ContentBlock::Image { .. } => Err(LlmError::invalid_request(
                            "Image content not supported in v0.1",
                        )),
                    })
                    .collect::<Result<Vec<String>, LlmError>>()?
                    .join(""); // Join multiple text blocks

                Ok(crate::grok::types::GrokMessage {
                    role,
                    content,
                    tool_calls: None,
                    tool_call_id: None,
                })
            })
            .collect::<Result<Vec<crate::grok::types::GrokMessage>, LlmError>>()?;

            let grok_request = crate::grok::types::GrokChatCompletionRequest {
                model: request.model,
                messages: grok_messages,
                max_tokens: Some(request.max_tokens),
                temperature: request.temperature,
                top_p: request.top_p,
                stop: request.stop_sequences,
                stream: None, // Non-streaming for now
                tools: None, // No tools for generic LlmClient interface
                tool_choice: None,
            };

        // Send request and convert response
        let grok_response = self.create_chat_completion(grok_request).await?;

        if grok_response.choices.is_empty() {
            return Err(LlmError::internal("No completion choices returned"));
        }

        let choice = &grok_response.choices[0];
        let content = vec![crate::types::ContentBlock::Text {
            text: choice.message.content.clone(),
        }];

        let response = crate::types::CompletionResponse {
            content,
            role: match choice.message.role {
                crate::grok::types::GrokRole::User => crate::types::Role::User,
                crate::grok::types::GrokRole::Assistant => crate::types::Role::Assistant,
                crate::grok::types::GrokRole::System => crate::types::Role::System,
            },
            usage: crate::types::Usage {
                input_tokens: grok_response.usage.as_ref().map(|u| u.prompt_tokens).unwrap_or(0),
                output_tokens: grok_response.usage.as_ref().map(|u| u.completion_tokens).unwrap_or(0),
            },
            stop_reason: choice.finish_reason.clone(),
        };

        Ok(response)
    }

    fn provider_name(&self) -> &str {
        crate::providers::XAI
    }

    fn model_name(&self) -> &str {
        crate::models::grok::BETA_ID // Default model
    }
}
