use crate::{
    error::LlmError,
    grok::types::{GrokChatCompletionRequest, GrokChatCompletionResponse, GrokErrorResponse},
};
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};

/// Zen provider for Grok (OpenCode Zen)
///
/// Free during beta, no authentication required for `grok-code` model.
pub struct ZenGrokClient {
    api_key: Option<String>,
    base_url: String,
    http_client: reqwest::Client,
}

impl ZenGrokClient {
    /// Create a new Zen Grok client (no API key required for free models)
    pub fn new() -> Result<Self, LlmError> {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300)) // 5 minute timeout
            .build()
            .map_err(|e| LlmError::Network { source: e })?;

        Ok(Self {
            api_key: None,
            base_url: "https://opencode.ai/zen".to_string(),
            http_client,
        })
    }

    /// Create a client with API key (for paid Zen models)
    pub fn with_api_key(api_key: impl Into<String>) -> Result<Self, LlmError> {
        let api_key = api_key.into();
        if api_key.is_empty() {
            return Err(LlmError::authentication("API key cannot be empty"));
        }

        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .map_err(|e| LlmError::Network { source: e })?;

        Ok(Self {
            api_key: Some(api_key),
            base_url: "https://opencode.ai/zen".to_string(),
            http_client,
        })
    }

    /// Set a custom base URL for the API
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Create a chat completion using the Zen Chat Completions API
    ///
    /// Default model is "grok-code" (free during beta)
    pub async fn create_chat_completion(
        &self,
        request: GrokChatCompletionRequest,
    ) -> Result<GrokChatCompletionResponse, LlmError> {
        let url = format!("{}/v1/chat/completions", self.base_url);

        let mut headers = HeaderMap::new();

        // Add authorization header if API key is provided
        if let Some(ref api_key) = self.api_key {
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {}", api_key)).map_err(|e| {
                    LlmError::authentication(format!("Invalid API key format: {}", e))
                })?,
            );
        }

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

        if !status.is_success() {
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unable to read error response".to_string());

            // Try to parse as Grok error format
            if let Ok(error_response) = serde_json::from_str::<GrokErrorResponse>(&error_body) {
                return Err(LlmError::api_error(
                    status.as_u16(),
                    error_response.error.message,
                ));
            }

            return Err(LlmError::api_error(status.as_u16(), error_body));
        }

        let completion_response = response.json::<GrokChatCompletionResponse>().await?;

        Ok(completion_response)
    }

    /// Get the provider name
    pub fn provider_name(&self) -> &str {
        "Zen (OpenCode)"
    }

    /// Get the default model for free access
    pub fn default_model() -> &'static str {
        "grok-code"
    }

    /// Start building a chat completion request
    pub fn message_builder(&self) -> crate::grok::GrokMessageBuilder<'_, ZenGrokClient> {
        crate::grok::GrokMessageBuilder::new(self)
    }
}

impl Default for ZenGrokClient {
    fn default() -> Self {
        Self::new().expect("Failed to create default ZenGrokClient")
    }
}

#[async_trait]
impl crate::client::LlmClient for ZenGrokClient {
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

        let response_format = request.response_format.map(|rf| match rf {
            crate::types::ResponseFormat::Text => crate::grok::types::GrokResponseFormat::text(),
            crate::types::ResponseFormat::JsonObject => {
                crate::grok::types::GrokResponseFormat::json_object()
            }
        });

        let grok_request = crate::grok::types::GrokChatCompletionRequest {
            model: request.model,
            messages: grok_messages,
            max_tokens: Some(request.max_tokens),
            temperature: request.temperature,
            top_p: request.top_p,
            stop: request.stop_sequences,
            stream: None, // Non-streaming for now
            tools: None,  // No tools for generic LlmClient interface
            tool_choice: None,
            response_format,
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
                input_tokens: grok_response
                    .usage
                    .as_ref()
                    .map(|u| u.prompt_tokens)
                    .unwrap_or(0),
                output_tokens: grok_response
                    .usage
                    .as_ref()
                    .map(|u| u.completion_tokens)
                    .unwrap_or(0),
            },
            stop_reason: choice.finish_reason.clone(),
            tool_calls: None, // TODO: Extract tool calls from Grok Zen response
        };

        Ok(response)
    }

    fn provider_name(&self) -> &str {
        crate::providers::ZEN
    }

    fn model_name(&self) -> &str {
        crate::models::grok::BETA_ID // Default model
    }
}
