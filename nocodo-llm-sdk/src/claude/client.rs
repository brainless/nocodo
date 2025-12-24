use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};

use crate::{
    claude::types::{ClaudeErrorResponse, ClaudeMessageRequest, ClaudeMessageResponse},
    error::LlmError,
};

/// Claude (Anthropic) LLM client
pub struct ClaudeClient {
    api_key: String,
    base_url: String,
    http_client: reqwest::Client,
}

impl ClaudeClient {
    /// Create a new Claude client with the given API key
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
            base_url: "https://api.anthropic.com".to_string(),
            http_client,
        })
    }

    /// Set a custom base URL for the API
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Create a message using the Claude Messages API
    pub async fn create_message(
        &self,
        request: ClaudeMessageRequest,
    ) -> Result<ClaudeMessageResponse, LlmError> {
        let url = format!("{}/v1/messages", self.base_url);

        let mut headers = HeaderMap::new();
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(&self.api_key)
                .map_err(|_| LlmError::authentication("Invalid API key format"))?,
        );
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
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
            let claude_response: ClaudeMessageResponse = response
                .json()
                .await
                .map_err(|e| LlmError::internal(format!("Failed to parse response: {}", e)))?;
            Ok(claude_response)
        } else {
            // Get retry-after header before consuming the response
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.parse().ok());

            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            // Try to parse as Claude error response
            if let Ok(error_response) = serde_json::from_str::<ClaudeErrorResponse>(&error_text) {
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

impl crate::claude::types::ClaudeMessageResponse {
    /// Extract tool calls from the response
    pub fn tool_calls(&self) -> Option<Vec<crate::tools::ToolCall>> {
        let tool_calls: Vec<crate::tools::ToolCall> = self
            .content
            .iter()
            .filter_map(|block| match block {
                crate::claude::types::ClaudeContentBlock::ToolUse { id, name, input } => Some(
                    crate::tools::ToolCall::new(id.clone(), name.clone(), input.clone()),
                ),
                _ => None,
            })
            .collect();

        if tool_calls.is_empty() {
            None
        } else {
            Some(tool_calls)
        }
    }
}

#[async_trait]
impl crate::client::LlmClient for ClaudeClient {
    async fn complete(
        &self,
        request: crate::types::CompletionRequest,
    ) -> Result<crate::types::CompletionResponse, LlmError> {
        // Convert generic request to Claude-specific request
        let claude_messages = request
            .messages
            .into_iter()
            .map(|msg| {
                let role = match msg.role {
                    crate::types::Role::User => crate::claude::types::ClaudeRole::User,
                    crate::types::Role::Assistant => crate::claude::types::ClaudeRole::Assistant,
                    crate::types::Role::System => {
                        return Err(LlmError::invalid_request(
                            "System messages should be provided via the system parameter",
                        ));
                    }
                };

                let content = msg
                    .content
                    .into_iter()
                    .map(|block| match block {
                        crate::types::ContentBlock::Text { text } => {
                            Ok(crate::claude::types::ClaudeContentBlock::Text { text })
                        }
                        crate::types::ContentBlock::Image { .. } => Err(LlmError::invalid_request(
                            "Image content not supported in v0.1",
                        )),
                    })
                    .collect::<Result<Vec<_>, _>>()?;

                Ok(crate::claude::types::ClaudeMessage { role, content })
            })
            .collect::<Result<Vec<_>, _>>()?;

        // Convert tools from generic format to Claude format
        let claude_tools = request.tools.map(|tools| {
            tools
                .into_iter()
                .map(|tool| crate::claude::types::ClaudeTool {
                    name: tool.name().to_string(),
                    description: tool.description().to_string(),
                    input_schema: tool.parameters().clone(),
                    cache_control: None,
                })
                .collect()
        });

        // Convert tool_choice
        let claude_tool_choice = request.tool_choice.map(|choice| match choice {
            crate::tools::ToolChoice::Auto => serde_json::json!({"type": "auto"}),
            crate::tools::ToolChoice::Required => serde_json::json!({"type": "any"}),
            crate::tools::ToolChoice::None => serde_json::json!({"type": "none"}),
            crate::tools::ToolChoice::Specific { name } => {
                serde_json::json!({"type": "tool", "name": name})
            }
        });

        let claude_request = crate::claude::types::ClaudeMessageRequest {
            model: request.model,
            max_tokens: request.max_tokens,
            messages: claude_messages,
            system: request.system,
            temperature: request.temperature,
            top_p: request.top_p,
            stop_sequences: request.stop_sequences,
            tools: claude_tools,
            tool_choice: claude_tool_choice,
        };

        // Send request and convert response
        let claude_response = self.create_message(claude_request).await?;

        // Extract text content and tool calls from response
        let mut content = Vec::new();
        let mut tool_calls = Vec::new();

        for block in claude_response.content {
            match block {
                crate::claude::types::ClaudeContentBlock::Text { text } => {
                    content.push(crate::types::ContentBlock::Text { text });
                }
                crate::claude::types::ClaudeContentBlock::ToolUse { id, name, input } => {
                    tool_calls.push(crate::tools::ToolCall::new(id, name, input));
                }
            }
        }

        let response = crate::types::CompletionResponse {
            content,
            role: match claude_response.role {
                crate::claude::types::ClaudeRole::User => crate::types::Role::User,
                crate::claude::types::ClaudeRole::Assistant => crate::types::Role::Assistant,
            },
            usage: crate::types::Usage {
                input_tokens: claude_response.usage.input_tokens,
                output_tokens: claude_response.usage.output_tokens,
            },
            stop_reason: claude_response.stop_reason,
            tool_calls: if tool_calls.is_empty() {
                None
            } else {
                Some(tool_calls)
            },
        };

        Ok(response)
    }

    fn provider_name(&self) -> &str {
        crate::providers::ANTHROPIC
    }

    fn model_name(&self) -> &str {
        crate::models::claude::SONNET_4_5_ID // Default model
    }
}
