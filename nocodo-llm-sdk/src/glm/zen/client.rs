use crate::{
    error::LlmError,
    glm::types::{GlmChatCompletionRequest, GlmChatCompletionResponse, GlmErrorResponse},
};
use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};

/// Zen provider for GLM (OpenCode Zen - "Big Pickle")
///
/// Free for limited time, no authentication required for `big-pickle` model.
/// Note: "Big Pickle" routes to GLM 4.6 on the backend.
pub struct ZenGlmClient {
    api_key: Option<String>,
    base_url: String,
    http_client: reqwest::Client,
}

impl ZenGlmClient {
    /// Create a new Zen GLM client (no API key required for free models)
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
    /// Default model is "big-pickle" (free, limited time, routes to GLM 4.6)
    pub async fn create_chat_completion(
        &self,
        request: GlmChatCompletionRequest,
    ) -> Result<GlmChatCompletionResponse, LlmError> {
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

            // Try to parse as GLM error format
            if let Ok(error_response) = serde_json::from_str::<GlmErrorResponse>(&error_body) {
                return Err(LlmError::api_error(
                    status.as_u16(),
                    error_response.error.message,
                ));
            }

            return Err(LlmError::api_error(status.as_u16(), error_body));
        }

        let completion_response = response.json::<GlmChatCompletionResponse>().await?;

        Ok(completion_response)
    }

    /// Get the provider name
    pub fn provider_name(&self) -> &str {
        "Zen (OpenCode)"
    }

    /// Get the default model for free access
    pub fn default_model() -> &'static str {
        "big-pickle"
    }

    /// Start building a chat completion request
    pub fn message_builder(&self) -> crate::glm::GlmMessageBuilder<'_, ZenGlmClient> {
        crate::glm::GlmMessageBuilder::new(self)
    }
}

impl Default for ZenGlmClient {
    fn default() -> Self {
        Self::new().expect("Failed to create default ZenGlmClient")
    }
}

#[async_trait]
impl crate::client::LlmClient for ZenGlmClient {
    async fn complete(
        &self,
        request: crate::types::CompletionRequest,
    ) -> Result<crate::types::CompletionResponse, LlmError> {
        // Convert generic request to GLM-specific request
        let glm_messages = request
            .messages
            .into_iter()
            .map(|msg| {
                let role = match msg.role {
                    crate::types::Role::User => crate::glm::types::GlmRole::User,
                    crate::types::Role::Assistant => crate::glm::types::GlmRole::Assistant,
                    crate::types::Role::System => crate::glm::types::GlmRole::System,
                };

                // GLM uses content as Option<String>
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

                Ok(crate::glm::types::GlmMessage {
                    role,
                    content: Some(content),
                    reasoning: None,
                    tool_calls: None,
                    tool_call_id: None,
                })
            })
            .collect::<Result<Vec<crate::glm::types::GlmMessage>, LlmError>>()?;

        let glm_request = crate::glm::types::GlmChatCompletionRequest {
            model: request.model,
            messages: glm_messages,
            max_completion_tokens: Some(request.max_tokens),
            temperature: request.temperature,
            top_p: request.top_p,
            stop: request.stop_sequences,
            stream: None, // Non-streaming for now
            seed: None,
            tools: None, // No tools for generic LlmClient interface
            tool_choice: None,
        };

        // Send request and convert response
        let glm_response = self.create_chat_completion(glm_request).await?;

        if glm_response.choices.is_empty() {
            return Err(LlmError::internal("No completion choices returned"));
        }

        let choice = &glm_response.choices[0];
        let content = vec![crate::types::ContentBlock::Text {
            text: choice.message.get_text(),
        }];

        let response = crate::types::CompletionResponse {
            content,
            role: match choice.message.role {
                crate::glm::types::GlmRole::User => crate::types::Role::User,
                crate::glm::types::GlmRole::Assistant => crate::types::Role::Assistant,
                crate::glm::types::GlmRole::System => crate::types::Role::System,
            },
            usage: crate::types::Usage {
                input_tokens: glm_response
                    .usage
                    .as_ref()
                    .map(|u| u.prompt_tokens)
                    .unwrap_or(0),
                output_tokens: glm_response
                    .usage
                    .as_ref()
                    .map(|u| u.completion_tokens)
                    .unwrap_or(0),
            },
            stop_reason: choice.finish_reason.clone(),
            tool_calls: None, // TODO: Extract tool calls from Zen response
        };

        Ok(response)
    }

    fn provider_name(&self) -> &str {
        crate::providers::ZEN
    }

    fn model_name(&self) -> &str {
        crate::models::glm::ZAI_GLM_4_6_ID // Default model
    }
}
