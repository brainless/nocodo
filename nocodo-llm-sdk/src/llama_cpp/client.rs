use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};

use crate::{
    error::LlmError,
    llama_cpp::types::{
        LlamaCppChatCompletionRequest, LlamaCppChatCompletionResponse, LlamaCppRole,
    },
    tools::ProviderToolFormat,
};

/// llama.cpp local LLM client (OpenAI-compatible)
pub struct LlamaCppClient {
    api_key: Option<String>,
    base_url: String,
    http_client: reqwest::Client,
}

impl LlamaCppClient {
    /// Create a new llama.cpp client with default base URL
    pub fn new() -> Result<Self, LlmError> {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .map_err(|e| LlmError::Network { source: e })?;

        Ok(Self {
            api_key: None,
            base_url: "http://localhost:8080".to_string(),
            http_client,
        })
    }

    /// Set an API key (optional; only required if the server enforces it)
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// Set a custom base URL for the API
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Start building a chat request
    pub fn message_builder(&self) -> crate::llama_cpp::builder::LlamaCppMessageBuilder<'_> {
        crate::llama_cpp::builder::LlamaCppMessageBuilder::new(self)
    }

    /// Create a chat completion using the OpenAI-compatible endpoint
    pub async fn create_chat_completion(
        &self,
        request: LlamaCppChatCompletionRequest,
    ) -> Result<LlamaCppChatCompletionResponse, LlmError> {
        let url = format!("{}/v1/chat/completions", self.base_url);

        let mut headers = HeaderMap::new();
        if let Some(api_key) = &self.api_key {
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {}", api_key))
                    .map_err(|_| LlmError::authentication("Invalid API key format"))?,
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

        if status.is_success() {
            let llama_response: LlamaCppChatCompletionResponse = response
                .json()
                .await
                .map_err(|e| LlmError::internal(format!("Failed to parse response: {}", e)))?;
            Ok(llama_response)
        } else {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

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

#[async_trait]
impl crate::client::LlmClient for LlamaCppClient {
    async fn complete(
        &self,
        request: crate::types::CompletionRequest,
    ) -> Result<crate::types::CompletionResponse, LlmError> {
        let messages = request
            .messages
            .into_iter()
            .map(|msg| {
                let role = match msg.role {
                    crate::types::Role::User => LlamaCppRole::User,
                    crate::types::Role::Assistant => LlamaCppRole::Assistant,
                    crate::types::Role::System => LlamaCppRole::System,
                };

                let content = msg
                    .content
                    .into_iter()
                    .map(|block| match block {
                        crate::types::ContentBlock::Text { text } => Ok(text),
                        crate::types::ContentBlock::Image { .. } => Err(LlmError::invalid_request(
                            "Image content not supported in llama.cpp client",
                        )),
                    })
                    .collect::<Result<Vec<String>, LlmError>>()?
                    .join("");

                Ok(crate::llama_cpp::types::LlamaCppMessage::new(role, content))
            })
            .collect::<Result<Vec<crate::llama_cpp::types::LlamaCppMessage>, LlmError>>()?;

        let tools = request.tools.as_ref().map(|tools| {
            tools
                .iter()
                .map(crate::llama_cpp::tools::LlamaCppToolFormat::to_provider_tool)
                .collect::<Vec<_>>()
        });

        let llama_request = LlamaCppChatCompletionRequest {
            model: request.model,
            messages,
            max_tokens: Some(request.max_tokens),
            temperature: request.temperature,
            top_p: request.top_p,
            stop: request.stop_sequences,
            stream: None,
            tools,
            parallel_tool_calls: None,
        };

        let llama_response = self.create_chat_completion(llama_request).await?;

        let choice = llama_response
            .choices
            .first()
            .ok_or_else(|| LlmError::internal("No choices returned"))?;

        let content_text = choice.message.content.clone().unwrap_or_default();
        let content = vec![crate::types::ContentBlock::Text { text: content_text }];

        let usage = llama_response.usage.unwrap_or(crate::llama_cpp::types::LlamaCppUsage {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        });

        let tool_calls = choice.message.tool_calls.as_ref().map(|calls| {
            calls
                .iter()
                .enumerate()
                .map(|(idx, call)| {
                    match call {
                        crate::llama_cpp::types::LlamaCppToolCall::Simple { name, arguments } => {
                            let arguments: serde_json::Value =
                                serde_json::from_str(arguments).unwrap_or(serde_json::Value::Null);
                            crate::tools::ToolCall::new(
                                format!("llama_cpp_tool_call_{}", idx),
                                name.clone(),
                                arguments,
                            )
                        }
                        crate::llama_cpp::types::LlamaCppToolCall::OpenAI(call) => {
                            let arguments: serde_json::Value =
                                serde_json::from_str(&call.function.arguments)
                                    .unwrap_or(serde_json::Value::Null);
                            crate::tools::ToolCall::new(
                                call.id.clone(),
                                call.function.name.clone(),
                                arguments,
                            )
                        }
                    }
                })
                .collect::<Vec<_>>()
        });

        Ok(crate::types::CompletionResponse {
            content,
            role: crate::types::Role::Assistant,
            usage: crate::types::Usage {
                input_tokens: usage.prompt_tokens,
                output_tokens: usage.completion_tokens,
            },
            stop_reason: choice.finish_reason.clone(),
            tool_calls,
        })
    }

    fn provider_name(&self) -> &str {
        crate::providers::LLAMA_CPP
    }

    fn model_name(&self) -> &str {
        "llama_cpp"
    }
}
