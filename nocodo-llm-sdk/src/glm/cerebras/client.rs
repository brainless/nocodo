use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};

use crate::{
    error::LlmError,
    glm::types::{GlmChatCompletionRequest, GlmChatCompletionResponse, GlmErrorResponse},
};

/// Cerebras provider for GLM
pub struct CerebrasGlmClient {
    api_key: String,
    base_url: String,
    http_client: reqwest::Client,
}

impl CerebrasGlmClient {
    /// Create a new GLM client with the given API key
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
            base_url: "https://api.cerebras.ai".to_string(),
            http_client,
        })
    }

    /// Set a custom base URL for the API
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Create a chat completion using the GLM Chat Completions API
    pub async fn create_chat_completion(
        &self,
        request: GlmChatCompletionRequest,
    ) -> Result<GlmChatCompletionResponse, LlmError> {
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
            let glm_response: GlmChatCompletionResponse = response
                .json()
                .await
                .map_err(|e| LlmError::internal(format!("Failed to parse response: {}", e)))?;
            Ok(glm_response)
        } else {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            // Try to parse as GLM error response
            if let Ok(error_response) = serde_json::from_str::<GlmErrorResponse>(&error_text) {
                match status {
                    reqwest::StatusCode::BAD_REQUEST => {
                        // Check for API key related errors
                        if error_response
                            .error
                            .message
                            .to_lowercase()
                            .contains("api key")
                            || error_response
                                .error
                                .message
                                .to_lowercase()
                                .contains("authorization")
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
                        if error_text.to_lowercase().contains("api key")
                            || error_text.to_lowercase().contains("authorization")
                        {
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

impl crate::glm::types::GlmChatCompletionResponse {
    /// Extract tool calls from the response
    pub fn tool_calls(&self) -> Option<Vec<crate::tools::ToolCall>> {
        self.choices
            .first()?
            .message
            .tool_calls
            .as_ref()
            .map(|calls| {
                calls
                    .iter()
                    .map(|call| {
                        let arguments: serde_json::Value =
                            serde_json::from_str(&call.function.arguments)
                                .unwrap_or(serde_json::Value::Null);

                        crate::tools::ToolCall::new(
                            call.id.clone(),
                            call.function.name.clone(),
                            arguments,
                        )
                    })
                    .collect()
            })
    }
}

#[async_trait]
impl crate::client::LlmClient for CerebrasGlmClient {
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

                Ok(crate::glm::types::GlmMessage {
                    role,
                    content: Some(content),
                    reasoning: None,
                    tool_calls: None,
                    tool_call_id: None,
                })
            })
            .collect::<Result<Vec<crate::glm::types::GlmMessage>, LlmError>>()?;

        let response_format = request.response_format.map(|rf| match rf {
            crate::types::ResponseFormat::Text => crate::glm::types::GlmResponseFormat::text(),
            crate::types::ResponseFormat::JsonObject => {
                crate::glm::types::GlmResponseFormat::json_object()
            }
        });

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
            response_format,
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
            tool_calls: None, // TODO: Extract tool calls from Cerebras response
        };

        Ok(response)
    }

    fn provider_name(&self) -> &str {
        crate::providers::CEREBRAS
    }

    fn model_name(&self) -> &str {
        crate::models::glm::LLAMA_3_3_70B_ID // Default model
    }
}
