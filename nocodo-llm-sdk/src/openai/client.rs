use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};

use crate::{
    error::LlmError,
    openai::types::{
        OpenAIChatCompletionRequest, OpenAIChatCompletionResponse, OpenAIErrorResponse,
        OpenAIResponseRequest, OpenAIResponseResponse,
    },
};

/// OpenAI LLM client
pub struct OpenAIClient {
    api_key: String,
    base_url: String,
    http_client: reqwest::Client,
}

impl OpenAIClient {
    /// Create a new OpenAI client with the given API key
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
            base_url: "https://api.openai.com".to_string(),
            http_client,
        })
    }

    /// Set a custom base URL for the API
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Create a chat completion using the OpenAI Chat Completions API
    pub async fn create_chat_completion(
        &self,
        request: OpenAIChatCompletionRequest,
    ) -> Result<OpenAIChatCompletionResponse, LlmError> {
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
            let openai_response: OpenAIChatCompletionResponse = response
                .json()
                .await
                .map_err(|e| LlmError::internal(format!("Failed to parse response: {}", e)))?;
            Ok(openai_response)
        } else {
            // Extract retry-after header before consuming the response
            let retry_after = if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                response
                    .headers()
                    .get("retry-after")
                    .and_then(|h| h.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
            } else {
                None
            };

            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            // Try to parse as OpenAI error response
            if let Ok(error_response) = serde_json::from_str::<OpenAIErrorResponse>(&error_text) {
                match status {
                    reqwest::StatusCode::BAD_REQUEST => {
                        // OpenAI returns 400 for various validation errors
                        Err(LlmError::invalid_request(error_response.error.message))
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
                    reqwest::StatusCode::TOO_MANY_REQUESTS => Err(LlmError::rate_limit(
                        error_response.error.message,
                        retry_after,
                    )),
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
                    reqwest::StatusCode::BAD_REQUEST => Err(LlmError::invalid_request(error_text)),
                    reqwest::StatusCode::UNAUTHORIZED => Err(LlmError::authentication(error_text)),
                    reqwest::StatusCode::FORBIDDEN => Err(LlmError::authentication(error_text)),
                    reqwest::StatusCode::NOT_FOUND => Err(LlmError::api_error(404, error_text)),
                    reqwest::StatusCode::PAYLOAD_TOO_LARGE => {
                        Err(LlmError::invalid_request("Request too large"))
                    }
                    reqwest::StatusCode::TOO_MANY_REQUESTS => {
                        Err(LlmError::rate_limit(error_text, retry_after))
                    }
                    reqwest::StatusCode::INTERNAL_SERVER_ERROR => {
                        Err(LlmError::api_error(500, error_text))
                    }
                    _ => Err(LlmError::api_error(status.as_u16(), error_text)),
                }
            }
        }
    }

    /// Create a response using the OpenAI Responses API
    pub async fn create_response(
        &self,
        request: OpenAIResponseRequest,
    ) -> Result<OpenAIResponseResponse, LlmError> {
        let url = format!("{}/v1/responses", self.base_url);

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
            let openai_response: OpenAIResponseResponse = response
                .json()
                .await
                .map_err(|e| LlmError::internal(format!("Failed to parse response: {}", e)))?;
            Ok(openai_response)
        } else {
            // Extract retry-after header before consuming the response
            let retry_after = if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                response
                    .headers()
                    .get("retry-after")
                    .and_then(|h| h.to_str().ok())
                    .and_then(|s| s.parse::<u64>().ok())
            } else {
                None
            };

            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            // Try to parse as OpenAI error response
            if let Ok(error_response) = serde_json::from_str::<OpenAIErrorResponse>(&error_text) {
                match status {
                    reqwest::StatusCode::BAD_REQUEST => {
                        Err(LlmError::invalid_request(error_response.error.message))
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
                    reqwest::StatusCode::TOO_MANY_REQUESTS => Err(LlmError::rate_limit(
                        error_response.error.message,
                        retry_after,
                    )),
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
                    reqwest::StatusCode::BAD_REQUEST => Err(LlmError::invalid_request(error_text)),
                    reqwest::StatusCode::UNAUTHORIZED => Err(LlmError::authentication(error_text)),
                    reqwest::StatusCode::FORBIDDEN => Err(LlmError::authentication(error_text)),
                    reqwest::StatusCode::NOT_FOUND => Err(LlmError::api_error(404, error_text)),
                    reqwest::StatusCode::PAYLOAD_TOO_LARGE => {
                        Err(LlmError::invalid_request("Request too large"))
                    }
                    reqwest::StatusCode::TOO_MANY_REQUESTS => {
                        Err(LlmError::rate_limit(error_text, retry_after))
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

impl crate::openai::types::OpenAIChatCompletionResponse {
    /// Get the content of the first choice
    pub fn content(&self) -> &str {
        &self.choices.first().unwrap().message.content
    }

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
impl crate::client::LlmClient for OpenAIClient {
    /// Routes requests to the appropriate OpenAI API based on model:
    ///
    /// ## Responses API (`/v1/responses`)
    /// Used for GPT-5.1+ models with extended reasoning capabilities:
    /// - `gpt-5.1-codex` - Optimized for code generation with extended reasoning
    /// - `gpt-5.1` - General reasoning model
    /// - `gpt-5.1-*` - All GPT-5.1 variant models
    ///
    /// ## Chat Completions API (`/v1/chat/completions`)
    /// Used for all other GPT models:
    /// - `gpt-4o` - GPT-4 Optimized for general tasks
    /// - `gpt-4-turbo` - Fast GPT-4 variant
    /// - `gpt-3.5-turbo` - Legacy models
    /// - All other GPT models
    ///
    /// ## API Differences
    /// - **Responses API**: Supports extended reasoning, background processing, and conversation continuation
    /// - **Chat Completions API**: Standard chat format with message history and role-based conversation
    ///
    /// Note: The Responses API is required for GPT-5.1 models to access
    /// extended reasoning capabilities and background processing features.
    /// When using the Responses API, all messages are concatenated into a single input string.
    async fn complete(
        &self,
        request: crate::types::CompletionRequest,
    ) -> Result<crate::types::CompletionResponse, LlmError> {
        // Check if this is a GPT-5.1-Codex model that should use Responses API
        if request.model.starts_with("gpt-5.1-codex") || request.model.starts_with("gpt-5.1") {
            // Use Responses API for GPT-5.1 models
            let input = request
                .messages
                .into_iter()
                .map(|msg| {
                    // For now, only support text content
                    msg.content
                        .into_iter()
                        .map(|block| match block {
                            crate::types::ContentBlock::Text { text } => Ok(text),
                            crate::types::ContentBlock::Image { .. } => {
                                Err(LlmError::invalid_request(
                                    "Image content not supported in Responses API",
                                ))
                            }
                        })
                        .collect::<Result<Vec<String>, LlmError>>()
                })
                .collect::<Result<Vec<Vec<String>>, LlmError>>()?
                .into_iter()
                .flatten()
                .collect::<Vec<String>>()
                .join("\n"); // Join all messages into single input

            let openai_request = crate::openai::types::OpenAIResponseRequest {
                model: request.model,
                input,
                stream: None,
                previous_response_id: None,
                background: None,
                prompt_cache_retention: None,
                tools: None, // No tools for generic LlmClient interface
                tool_choice: None,
                parallel_tool_calls: None,
            };

            // Send request and convert response
            let openai_response = self.create_response(openai_request).await?;

            // Extract text from message output items
            let mut text_content = String::new();
            for item in &openai_response.output {
                if item.item_type == "message" {
                    if let Some(content_blocks) = &item.content {
                        for block in content_blocks {
                            if block.content_type == "output_text" {
                                text_content.push_str(&block.text);
                            }
                        }
                    }
                }
            }

            let content = vec![crate::types::ContentBlock::Text { text: text_content }];

            let response = crate::types::CompletionResponse {
                content,
                role: crate::types::Role::Assistant,
                usage: crate::types::Usage {
                    input_tokens: openai_response
                        .usage
                        .input_tokens
                        .unwrap_or(openai_response.usage.prompt_tokens.unwrap_or(0)),
                    output_tokens: openai_response
                        .usage
                        .output_tokens
                        .unwrap_or(openai_response.usage.completion_tokens.unwrap_or(0)),
                },
                stop_reason: Some("completed".to_string()), // Responses API doesn't have finish_reason like Chat Completions
                tool_calls: None, // TODO: Extract tool calls from OpenAI response
            };

            Ok(response)
        } else {
            // Use Chat Completions API for other models
            let openai_messages = request
                .messages
                .into_iter()
                .map(|msg| {
                    let role = match msg.role {
                        crate::types::Role::User => crate::openai::types::OpenAIRole::User,
                        crate::types::Role::Assistant => {
                            crate::openai::types::OpenAIRole::Assistant
                        }
                        crate::types::Role::System => crate::openai::types::OpenAIRole::System,
                    };

                    // For now, only support text content
                    let content = msg
                        .content
                        .into_iter()
                        .map(|block| match block {
                            crate::types::ContentBlock::Text { text } => Ok(text),
                            crate::types::ContentBlock::Image { .. } => Err(
                                LlmError::invalid_request("Image content not supported in v0.1"),
                            ),
                        })
                        .collect::<Result<Vec<String>, LlmError>>()?
                        .join(""); // Join multiple text blocks

                    Ok(crate::openai::types::OpenAIMessage {
                        role,
                        content,
                        tool_calls: None,
                        tool_call_id: None,
                    })
                })
                .collect::<Result<Vec<crate::openai::types::OpenAIMessage>, LlmError>>()?;

            let openai_request = crate::openai::types::OpenAIChatCompletionRequest {
                model: request.model,
                messages: openai_messages,
                max_tokens: None, // Use max_completion_tokens instead
                max_completion_tokens: Some(request.max_tokens),
                temperature: request.temperature,
                top_p: request.top_p,
                stop: request.stop_sequences,
                stream: None,           // Non-streaming for now
                reasoning_effort: None, // Default reasoning effort
                tools: None,            // No tools for generic LlmClient interface
                tool_choice: None,
                parallel_tool_calls: None,
            };

            // Send request and convert response
            let openai_response = self.create_chat_completion(openai_request).await?;

            if openai_response.choices.is_empty() {
                return Err(LlmError::internal("No completion choices returned"));
            }

            let choice = &openai_response.choices[0];
            let content = vec![crate::types::ContentBlock::Text {
                text: choice.message.content.clone(),
            }];

            let response = crate::types::CompletionResponse {
                content,
                role: match choice.message.role {
                    crate::openai::types::OpenAIRole::User => crate::types::Role::User,
                    crate::openai::types::OpenAIRole::Assistant => crate::types::Role::Assistant,
                    crate::openai::types::OpenAIRole::System => crate::types::Role::System,
                    crate::openai::types::OpenAIRole::Tool => crate::types::Role::Assistant, // Map tool to assistant
                },
                usage: crate::types::Usage {
                    input_tokens: openai_response
                        .usage
                        .prompt_tokens
                        .unwrap_or(openai_response.usage.input_tokens.unwrap_or(0)),
                    output_tokens: openai_response
                        .usage
                        .completion_tokens
                        .unwrap_or(openai_response.usage.output_tokens.unwrap_or(0)),
                },
                stop_reason: choice.finish_reason.clone(),
                tool_calls: None, // TODO: Extract tool calls from OpenAI response
            };

            Ok(response)
        }
    }

    fn provider_name(&self) -> &str {
        crate::providers::OPENAI
    }

    fn model_name(&self) -> &str {
        crate::models::openai::GPT_4O_ID // Default to GPT-4o
    }
}
