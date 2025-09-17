use crate::models::LlmProviderConfig;
use anyhow::Result;
use async_stream::try_stream;
use async_trait::async_trait;
use futures_util::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::pin::Pin;
use uuid::Uuid;

/// LLM message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    // NEW: Tool calls in message (for assistant responses)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<LlmToolCall>>,
    // Legacy OpenAI function call
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<LlmFunctionCall>,
    // Tool call ID (for tool responses)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// LLM completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmCompletionRequest {
    pub model: String,
    pub messages: Vec<LlmMessage>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub stream: Option<bool>,
    // NEW: Tool/Function parameters for native tool calling support
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    // OpenAI legacy function calling (for backward compatibility)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub functions: Option<Vec<FunctionDefinition>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<FunctionCall>,
}

/// LLM completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<LlmChoice>,
    pub usage: Option<LlmUsage>,
}

/// LLM choice in completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmChoice {
    pub index: u32,
    pub message: Option<LlmMessage>,
    pub delta: Option<LlmMessageDelta>,
    pub finish_reason: Option<String>,
    // Anthropic-specific: tool calls in choice level
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<LlmToolCall>>,
}

/// Tool call in LLM response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub r#type: String, // Should be "function"
    pub function: LlmToolCallFunction,
}

/// Function call within tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmToolCallFunction {
    pub name: String,
    pub arguments: String, // JSON string of arguments
}

/// Legacy OpenAI function call in response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmFunctionCall {
    pub name: String,
    pub arguments: String, // JSON string of arguments
}

/// Provider capabilities structure
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ProviderCapabilities {
    pub supports_native_tools: bool,
    pub supports_legacy_functions: bool,
    pub supports_streaming: bool,
    pub supports_json_mode: bool,
}

/// Completion result with tool calls
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CompletionResult {
    pub response: LlmCompletionResponse,
    pub tool_calls: Vec<LlmToolCall>,
}

/// LLM message delta for streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessageDelta {
    pub role: Option<String>,
    pub content: Option<String>,
    // NEW: Tool calls in streaming delta
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<LlmToolCallDelta>>,
}

/// Tool call delta for streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmToolCallDelta {
    pub index: u32,
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub r#type: Option<String>,
    pub function: Option<LlmToolCallFunctionDelta>,
}

/// Function call delta for streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmToolCallFunctionDelta {
    pub name: Option<String>,
    pub arguments: Option<String>,
}

/// LLM token usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// LLM completion chunk for streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmCompletionChunk {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<LlmChoice>,
}

/// Streaming response chunk
#[derive(Debug, Clone, Default)]
pub struct StreamChunk {
    pub content: String,
    pub is_finished: bool,
    // NEW: Tool calls in streaming chunk
    pub tool_calls: Vec<LlmToolCall>,
}



/// Tool definition for native tool calling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub r#type: String, // Should be "function"
    pub function: FunctionDefinition,
}

/// Function definition within a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value, // JSON Schema object
}

/// Tool choice specification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolChoice {
    /// Use "none" to disable tool calling
    None(String),
    /// Use "auto" to let the model decide
    Auto(String),
    /// Use "required" to force tool calling
    Required(String),
    /// Specify a particular tool by name
    Specific {
        #[serde(rename = "type")]
        r#type: String, // Should be "function"
        function: ToolFunctionChoice,
    },
}

/// Function choice within tool choice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFunctionChoice {
    pub name: String,
}

/// Legacy OpenAI function call specification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FunctionCall {
    /// Disable function calling
    None(String),
    /// Let the model decide
    Auto(String),
    /// Force function calling
    Required(String),
    /// Specify a particular function by name
    Specific { name: String },
}

/// Abstract LLM client trait
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Complete a prompt without streaming
    #[allow(dead_code)]
    async fn complete(&self, request: LlmCompletionRequest) -> Result<LlmCompletionResponse>;

    /// Complete a prompt with streaming response
    fn stream_complete(
        &self,
        request: LlmCompletionRequest,
    ) -> Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>;

    /// Complete a prompt and extract tool calls
    #[allow(dead_code)]
    async fn complete_with_tools(&self, request: LlmCompletionRequest) -> Result<CompletionResult> {
        let response = self.complete(request).await?;
        let tool_calls = self.extract_tool_calls_from_response(&response);
        Ok(CompletionResult {
            response,
            tool_calls,
        })
    }

    /// Extract tool calls from a completion response
    fn extract_tool_calls_from_response(
        &self,
        response: &LlmCompletionResponse,
    ) -> Vec<LlmToolCall>;

    /// Get the provider name
    #[allow(dead_code)]
    fn provider(&self) -> &str;

    /// Get the model name
    #[allow(dead_code)]
    fn model(&self) -> &str;
}

/// OpenAI-compatible LLM client
pub struct OpenAiCompatibleClient {
    client: reqwest::Client,
    config: LlmProviderConfig,
}

impl OpenAiCompatibleClient {
    pub fn new(config: LlmProviderConfig) -> Result<Self> {
        let client = reqwest::Client::new();
        Ok(Self { client, config })
    }

    fn get_api_url(&self) -> String {
        if let Some(base_url) = &self.config.base_url {
            format!("{}/v1/chat/completions", base_url.trim_end_matches('/'))
        } else {
            "https://api.openai.com/v1/chat/completions".to_string()
        }
    }

    /// Check if the provider supports native tool calling
    fn supports_native_tools(&self) -> bool {
        match self.config.provider.to_lowercase().as_str() {
            "openai" => {
                // OpenAI supports native tools for GPT-4 and newer models
                self.config.model.to_lowercase().starts_with("gpt-4")
                    || self.config.model.to_lowercase().contains("gpt-4")
            }
            "anthropic" | "claude" => {
                // Anthropic supports native tools for Claude models
                self.config.model.to_lowercase().contains("claude")
                    || self.config.model.to_lowercase().contains("opus")
                    || self.config.model.to_lowercase().contains("sonnet")
                    || self.config.model.to_lowercase().contains("haiku")
            }
            "grok" | "xai" => {
                // Grok/xAI does not support native tools yet
                false
            }
            _ => false,
        }
    }

    /// Check if the provider supports legacy function calling
    fn supports_legacy_functions(&self) -> bool {
        match self.config.provider.to_lowercase().as_str() {
            "openai" => {
                // OpenAI supports legacy functions for older models
                !self.supports_native_tools()
            }
            _ => false,
        }
    }

    /// Get provider capabilities as a structured object
    #[allow(dead_code)]
    fn get_provider_capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_native_tools: self.supports_native_tools(),
            supports_legacy_functions: self.supports_legacy_functions(),
            supports_streaming: true, // All current providers support streaming
            supports_json_mode: matches!(self.config.provider.to_lowercase().as_str(), "openai"),
        }
    }

    /// Extract tool calls from a completion response (internal method)
    fn extract_tool_calls_from_response_internal(
        &self,
        response: &LlmCompletionResponse,
    ) -> Vec<LlmToolCall> {
        let mut tool_calls = Vec::new();
        let mut message_tool_calls_count = 0;
        let mut choice_tool_calls_count = 0;
        let mut legacy_function_calls_count = 0;

        for choice in &response.choices {
            if let Some(message) = &choice.message {
                // Check for tool calls in the message (OpenAI format)
                if let Some(message_tool_calls) = &message.tool_calls {
                    message_tool_calls_count += message_tool_calls.len();
                    tool_calls.extend(message_tool_calls.clone());
                    tracing::debug!(
                        provider = %self.config.provider,
                        choice_index = %choice.index,
                        tool_calls_in_message = %message_tool_calls.len(),
                        "Found tool calls in message (OpenAI format)"
                    );
                }

                // Check for legacy function calls (older OpenAI models)
                if let Some(function_call) = &message.function_call {
                    legacy_function_calls_count += 1;
                    tool_calls.push(LlmToolCall {
                        id: format!("legacy-{}", Uuid::new_v4()),
                        r#type: "function".to_string(),
                        function: LlmToolCallFunction {
                            name: function_call.name.clone(),
                            arguments: function_call.arguments.clone(),
                        },
                    });
                    tracing::debug!(
                        provider = %self.config.provider,
                        choice_index = %choice.index,
                        function_name = %function_call.name,
                        "Found legacy function call in message"
                    );
                }
            }

            // Check for tool calls at choice level (Anthropic format)
            if let Some(choice_tool_calls) = &choice.tool_calls {
                choice_tool_calls_count += choice_tool_calls.len();
                tool_calls.extend(choice_tool_calls.clone());
                tracing::debug!(
                    provider = %self.config.provider,
                    choice_index = %choice.index,
                    tool_calls_in_choice = %choice_tool_calls.len(),
                    "Found tool calls at choice level (Anthropic format)"
                );
            }
        }

        tracing::info!(
            provider = %self.config.provider,
            total_tool_calls = %tool_calls.len(),
            message_tool_calls = %message_tool_calls_count,
            choice_tool_calls = %choice_tool_calls_count,
            legacy_function_calls = %legacy_function_calls_count,
            "Extracted tool calls from completion response"
        );

        tool_calls
    }

    /// Prepare request for the specific provider by converting tools to appropriate format
    fn prepare_request_for_provider(
        &self,
        mut request: LlmCompletionRequest,
    ) -> LlmCompletionRequest {
        // If no tools are specified, return as-is
        if request.tools.is_none() && request.functions.is_none() {
            return request;
        }

        if self.supports_native_tools() {
            // Provider supports native tools - ensure tools are in the right format
            match self.config.provider.to_lowercase().as_str() {
                "anthropic" | "claude" => {
                    // Anthropic uses similar format to OpenAI, but may have some differences
                    // For now, we use the same format and can refine later if needed
                    tracing::debug!(
                        provider = %self.config.provider,
                        "Using Anthropic native tool calling format"
                    );
                }
                "openai" => {
                    // OpenAI native tools format
                    tracing::debug!(
                        provider = %self.config.provider,
                        "Using OpenAI native tool calling format"
                    );
                }
                _ => {
                    tracing::debug!(
                        provider = %self.config.provider,
                        "Using generic native tool calling format"
                    );
                }
            }
        } else if self.supports_legacy_functions() {
            // Convert tools to legacy functions format for older OpenAI models
            if let Some(tools) = request.tools.take() {
                let functions: Vec<FunctionDefinition> = tools
                    .into_iter()
                    .filter_map(|tool| {
                        if tool.r#type == "function" {
                            Some(tool.function)
                        } else {
                            None
                        }
                    })
                    .collect();

                if !functions.is_empty() {
                    request.functions = Some(functions);
                    // Convert tool_choice to function_call if needed
                    if let Some(tool_choice) = request.tool_choice.take() {
                        match tool_choice {
                            ToolChoice::None(_) => {
                                request.function_call =
                                    Some(FunctionCall::None("none".to_string()));
                            }
                            ToolChoice::Auto(_) => {
                                request.function_call =
                                    Some(FunctionCall::Auto("auto".to_string()));
                            }
                            ToolChoice::Required(_) => {
                                request.function_call =
                                    Some(FunctionCall::Required("required".to_string()));
                            }
                            ToolChoice::Specific { function, .. } => {
                                request.function_call = Some(FunctionCall::Specific {
                                    name: function.name,
                                });
                            }
                        }
                    }
                }
            }
            tracing::debug!(
                provider = %self.config.provider,
                "Converted tools to legacy function calling format"
            );
        } else {
            // Provider doesn't support native tools - remove tools from request
            // The LLM agent will fall back to JSON parsing in the system prompt
            request.tools = None;
            request.tool_choice = None;
            request.functions = None;
            request.function_call = None;

            tracing::info!(
                provider = %self.config.provider,
                model = %self.config.model,
                "Provider does not support native tools, falling back to JSON parsing. Tools removed from request."
            );
        }

        request
    }

    #[allow(dead_code)]
    async fn make_request(&self, request: LlmCompletionRequest) -> Result<reqwest::Response> {
        let mut req = self
            .client
            .post(self.get_api_url())
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request);

        // Add custom headers for different providers
        if self.config.provider.to_lowercase() == "grok" {
            req = req.header("x-api-key", &self.config.api_key);
            req = req.header("x-api-version", "2024-06-01");
        }

        Ok(req.send().await?)
    }
}

#[async_trait]
impl LlmClient for OpenAiCompatibleClient {
    async fn complete(&self, mut request: LlmCompletionRequest) -> Result<LlmCompletionResponse> {
        // Apply config defaults
        if request.max_tokens.is_none() {
            request.max_tokens = self.config.max_tokens;
        }
        if request.temperature.is_none() {
            request.temperature = self.config.temperature;
        }

        let start_time = std::time::Instant::now();
        let message_count = request.messages.len();
        let total_input_tokens: usize = request
            .messages
            .iter()
            .map(|m| m.content.as_ref().map(|c| c.len()).unwrap_or(0))
            .sum();

        tracing::info!(
            provider = %self.config.provider,
            model = %request.model,
            message_count = %message_count,
            estimated_input_tokens = %total_input_tokens,
            max_tokens = ?request.max_tokens,
            temperature = ?request.temperature,
            has_tools = %request.tools.is_some(),
            supports_native_tools = %self.supports_native_tools(),
            supports_legacy_functions = %self.supports_legacy_functions(),
            "Sending non-streaming completion request to LLM provider"
        );

        // Prepare request for the specific provider
        let prepared_request = self.prepare_request_for_provider(request.clone());

        let response = self.make_request(prepared_request).await?;

        let response_time = start_time.elapsed();
        let status = response.status();

        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            tracing::error!(
                provider = %self.config.provider,
                model = %request.model,
                status = %status,
                response_time_ms = %response_time.as_millis(),
                error = %error_text,
                "LLM API request failed"
            );

            return Err(anyhow::anyhow!(
                "LLM API error: {} - {}",
                status,
                error_text
            ));
        }

        let completion: LlmCompletionResponse = response.json().await?;

        tracing::info!(
            provider = %self.config.provider,
            model = %request.model,
            status = %status,
            response_time_ms = %response_time.as_millis(),
            completion_id = %completion.id,
            created = %completion.created,
            choices_count = %completion.choices.len(),
            usage = ?completion.usage,
            "LLM API request completed successfully"
        );

        Ok(completion)
    }

    fn stream_complete(
        &self,
        mut request: LlmCompletionRequest,
    ) -> Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>> {
        request.stream = Some(true);

        let start_time = std::time::Instant::now();
        let client = self.client.clone();
        let config = self.config.clone();
        let api_url = self.get_api_url();

        let message_count = request.messages.len();
        let total_input_tokens = message_count * 10; // rough estimate

        tracing::info!(
            provider = %self.config.provider,
            model = %request.model,
            message_count = %message_count,
            estimated_input_tokens = %total_input_tokens,
            max_tokens = ?request.max_tokens,
            temperature = ?request.temperature,
            has_tools = %request.tools.is_some(),
            "Sending streaming completion request to LLM provider"
        );

        // Prepare request for the specific provider
        let prepared_request = self.prepare_request_for_provider(request.clone());

        Box::pin(try_stream! {
            let mut req = client
                .post(&api_url)
                .header("Authorization", format!("Bearer {}", config.api_key))
                .header("Content-Type", "application/json")
                .json(&prepared_request);

            // Add custom headers for different providers
            if config.provider.to_lowercase() == "grok" {
                req = req.header("x-api-key", &config.api_key);
                req = req.header("x-api-version", "2024-06-01");
            }

            let response = req.send().await?;
            let mut stream = response.bytes_stream();

            while let Some(chunk) = stream.next().await {
                let chunk = chunk?;
                let text = String::from_utf8_lossy(&chunk);

                // Process SSE-style stream
                for line in text.lines() {
                    let line = line.trim();
                    if let Some(data) = line.strip_prefix("data: ") {
                        if data == "[DONE]" {
                            let response_time = start_time.elapsed();
                            tracing::info!(
                                provider = %config.provider,
                                model = %request.model,
                                response_time_ms = %response_time.as_millis(),
                                "Streaming LLM API request completed successfully"
                            );
                            yield StreamChunk {
                                content: String::new(),
                                is_finished: true,
                                tool_calls: Vec::new(),
                            };
                            return;
                        }

                        if let Ok(chunk_value) = serde_json::from_str::<Value>(data) {
                            if let Some(choices) = chunk_value.get("choices").and_then(|v| v.as_array()) {
                                if let Some(choice) = choices.first() {
                                    if let Some(delta) = choice.get("delta") {
                                        if let Some(content) = delta.get("content").and_then(|v| v.as_str()) {
                                            let response_time = start_time.elapsed();
                                            tracing::trace!(
                                                provider = %config.provider,
                                                model = %request.model,
                                                response_time_ms = %response_time.as_millis(),
                                                chunk_length = %content.len(),
                                                "Received streaming chunk"
                                            );
                                            yield StreamChunk {
                                                content: content.to_string(),
                                                is_finished: false,
                                                tool_calls: Vec::new(),
                                            };
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        })
    }

    fn extract_tool_calls_from_response(
        &self,
        response: &LlmCompletionResponse,
    ) -> Vec<LlmToolCall> {
        self.extract_tool_calls_from_response_internal(response)
    }

    fn provider(&self) -> &str {
        &self.config.provider
    }

    fn model(&self) -> &str {
        &self.config.model
    }
}

/// Factory function to create LLM clients
pub fn create_llm_client(config: LlmProviderConfig) -> Result<Box<dyn LlmClient>> {
    match config.provider.to_lowercase().as_str() {
        "openai" | "grok" | "anthropic" | "claude" => {
            let client = OpenAiCompatibleClient::new(config)?;
            Ok(Box::new(client))
        }
        _ => Err(anyhow::anyhow!(
            "Unsupported LLM provider: {}",
            config.provider
        )),
    }
}
