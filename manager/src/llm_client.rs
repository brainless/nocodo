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
#[allow(dead_code)]
pub struct LlmCompletionChunk {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<LlmChoice>,
}

/// Streaming response chunk
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
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
    #[allow(dead_code)]
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
                // Grok Code Fast 1 and newer models support native function calling
                self.config.model.to_lowercase().contains("grok-code-fast")
                    || self.config.model.to_lowercase().contains("grok-2")
                    || self.config.model.to_lowercase().contains("grok-3")
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
                "grok" | "xai" => {
                    // Grok uses OpenAI-compatible tool calling format
                    tracing::debug!(
                        provider = %self.config.provider,
                        "Using Grok/xAI OpenAI-compatible tool calling format"
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
        let req = self
            .client
            .post(self.get_api_url())
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request);

        // Add custom headers for different providers
        if self.config.provider.to_lowercase() == "grok" {
            // Grok uses Bearer token authentication, no additional headers needed
            // API documented at https://docs.x.ai/docs/api-reference
            // OpenAI-compatible API, uses standard Authorization header only
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

        // Log the raw request being sent to the LLM
        let request_json = serde_json::to_string_pretty(&prepared_request)
            .unwrap_or_else(|_| "Failed to serialize request".to_string());
        tracing::info!(
            provider = %self.config.provider,
            model = %request.model,
            api_url = %self.get_api_url(),
            raw_request = %request_json,
            "Raw LLM request being sent to provider"
        );

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

        // Get the raw response text first for logging
        let response_text = response.text().await?;

        // Log the raw response received from the LLM
        tracing::info!(
            provider = %self.config.provider,
            model = %request.model,
            status = %status,
            response_time_ms = %response_time.as_millis(),
            response_length = %response_text.len(),
            raw_response = %response_text,
            "Raw LLM response received from provider"
        );

        // Parse the response
        let completion: LlmCompletionResponse = serde_json::from_str(&response_text)?;

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

        // Log the raw streaming request being sent to the LLM
        let request_json = serde_json::to_string_pretty(&prepared_request)
            .unwrap_or_else(|_| "Failed to serialize request".to_string());
        tracing::info!(
            provider = %self.config.provider,
            model = %request.model,
            api_url = %api_url,
            raw_request = %request_json,
            "Raw LLM streaming request being sent to provider"
        );

        Box::pin(try_stream! {
            let req = client
                .post(&api_url)
                .header("Authorization", format!("Bearer {}", config.api_key))
                .header("Content-Type", "application/json")
                .json(&prepared_request);

            // Add custom headers for different providers
            if config.provider.to_lowercase() == "grok" {
                // Grok uses Bearer token authentication, no additional headers needed
                // API documented at https://docs.x.ai/docs/api-reference
                // OpenAI-compatible API, uses standard Authorization header only
            }

            let response = req.send().await?;
            let mut stream = response.bytes_stream();
            let mut accumulated_tool_calls = Vec::new();

            while let Some(chunk) = stream.next().await {
                let chunk = chunk?;
                let text = String::from_utf8_lossy(&chunk);

                // Log raw streaming chunk
                if !text.trim().is_empty() {
                    tracing::debug!(
                        provider = %config.provider,
                        model = %request.model,
                        chunk_size = %chunk.len(),
                        raw_chunk = %text,
                        "Raw streaming chunk received from provider"
                    );
                }

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
                                total_accumulated_tool_calls = %accumulated_tool_calls.len(),
                                "Streaming LLM API request completed successfully"
                            );
                            // Only yield if we haven't already yielded the final chunk via finish_reason
                            if accumulated_tool_calls.iter().any(|tc: &LlmToolCall| !tc.id.is_empty() || !tc.function.name.is_empty()) {
                                yield StreamChunk {
                                    content: String::new(),
                                    is_finished: true,
                                    tool_calls: accumulated_tool_calls,
                                };
                            } else {
                                yield StreamChunk {
                                    content: String::new(),
                                    is_finished: true,
                                    tool_calls: Vec::new(),
                                };
                            }
                            return;
                        }

                        if let Ok(chunk_value) = serde_json::from_str::<Value>(data) {
                            if let Some(choices) = chunk_value.get("choices").and_then(|v| v.as_array()) {
                                if let Some(choice) = choices.first() {
                                    let mut content = String::new();
                                    let mut tool_calls_in_chunk = Vec::new();

                                    // Check finish_reason to see if this is the final chunk
                                    let finish_reason = choice.get("finish_reason").and_then(|v| v.as_str());
                                    let is_finished = finish_reason.is_some();

                                    // Handle tool calls at choice level (Anthropic/Grok format)
                                    if let Some(choice_tool_calls) = choice.get("tool_calls").and_then(|v| v.as_array()) {
                                        for tool_call_value in choice_tool_calls {
                                            if let Ok(tool_call) = serde_json::from_value::<LlmToolCall>(tool_call_value.clone()) {
                                                accumulated_tool_calls.push(tool_call.clone());
                                                tool_calls_in_chunk.push(tool_call);
                                            }
                                        }
                                        tracing::debug!(
                                            provider = %config.provider,
                                            tool_calls_at_choice_level = %choice_tool_calls.len(),
                                            "Found tool calls at choice level"
                                        );
                                    }

                                    // Handle streaming delta (OpenAI format)
                                    if let Some(delta) = choice.get("delta") {
                                        // Parse the delta as LlmMessageDelta to handle both content and tool calls
                                        if let Ok(message_delta) = serde_json::from_value::<LlmMessageDelta>(delta.clone()) {
                                            // Extract content
                                            if let Some(delta_content) = message_delta.content {
                                                content = delta_content;
                                            }

                                            // Extract and accumulate tool calls from delta
                                            if let Some(delta_tool_calls) = message_delta.tool_calls {
                                                for delta_tool_call in delta_tool_calls {
                                                    // Convert streaming tool call delta to complete tool call
                                                    let index = delta_tool_call.index;
                                                    let index_usize = index as usize;

                                                    // Ensure we have enough space in accumulated_tool_calls
                                                    while accumulated_tool_calls.len() <= index_usize {
                                                        accumulated_tool_calls.push(LlmToolCall {
                                                            id: String::new(),
                                                            r#type: "function".to_string(),
                                                            function: LlmToolCallFunction {
                                                                name: String::new(),
                                                                arguments: String::new(),
                                                            },
                                                        });
                                                    }

                                                    // Update the tool call at this index
                                                    if let Some(existing_tool_call) = accumulated_tool_calls.get_mut(index_usize) {
                                                        // Update ID
                                                        if let Some(id) = delta_tool_call.id {
                                                            existing_tool_call.id = id;
                                                        }

                                                        // Update function name
                                                        if let Some(function_delta) = delta_tool_call.function {
                                                            if let Some(name) = function_delta.name {
                                                                existing_tool_call.function.name = name;
                                                            }
                                                            if let Some(arguments) = function_delta.arguments {
                                                                // Accumulate arguments (they come in pieces)
                                                                existing_tool_call.function.arguments.push_str(&arguments);
                                                            }
                                                        }
                                                    }

                                                    // Add to chunk tool calls for immediate processing
                                                    tool_calls_in_chunk.push(accumulated_tool_calls[index_usize].clone());
                                                }
                                            }
                                        } else {
                                            // Fallback: try to extract content directly if parsing as LlmMessageDelta fails
                                            if let Some(content_str) = delta.get("content").and_then(|v| v.as_str()) {
                                                content = content_str.to_string();
                                            }
                                        }
                                    }

                                    let response_time = start_time.elapsed();
                                    tracing::debug!(
                                        provider = %config.provider,
                                        model = %request.model,
                                        response_time_ms = %response_time.as_millis(),
                                        chunk_length = %content.len(),
                                        tool_calls_in_chunk = %tool_calls_in_chunk.len(),
                                        total_accumulated_tool_calls = %accumulated_tool_calls.len(),
                                        finish_reason = ?finish_reason,
                                        is_finished = %is_finished,
                                        content_preview = %if content.len() > 100 {
                                            format!("{}...", &content[..100])
                                        } else {
                                            content.clone()
                                        },
                                        "Received streaming chunk"
                                    );

                                    yield StreamChunk {
                                        content,
                                        is_finished,
                                        tool_calls: tool_calls_in_chunk,
                                    };

                                    // If this is the final chunk, we're done
                                    if is_finished {
                                        return;
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

/// Claude-specific message content blocks
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClaudeContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
}

/// Claude-specific message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeMessage {
    pub role: String,
    pub content: Vec<ClaudeContentBlock>,
}

/// Claude API request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeCompletionRequest {
    pub model: String,
    pub messages: Vec<ClaudeMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ClaudeToolDefinition>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ClaudeToolChoice>,
}

/// Claude tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// Claude tool choice
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClaudeToolChoice {
    Auto { r#type: String },               // {"type": "auto"}
    Any { r#type: String },                // {"type": "any"}
    Tool { r#type: String, name: String }, // {"type": "tool", "name": "tool_name"}
}

/// Claude API response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeCompletionResponse {
    pub id: String,
    pub r#type: String,
    pub role: String,
    pub content: Vec<ClaudeContentBlock>,
    pub model: String,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub usage: ClaudeUsage,
}

/// Claude usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}

/// Claude LLM client
pub struct ClaudeClient {
    client: reqwest::Client,
    config: LlmProviderConfig,
}

impl ClaudeClient {
    pub fn new(config: LlmProviderConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        Ok(Self { client, config })
    }

    fn get_api_url(&self) -> String {
        if let Some(base_url) = &self.config.base_url {
            format!("{}/v1/messages", base_url.trim_end_matches('/'))
        } else {
            "https://api.anthropic.com/v1/messages".to_string()
        }
    }

    /// Check if the provider supports native tool calling
    fn supports_native_tools(&self) -> bool {
        self.config.model.to_lowercase().contains("claude")
            || self.config.model.to_lowercase().contains("opus")
            || self.config.model.to_lowercase().contains("sonnet")
            || self.config.model.to_lowercase().contains("haiku")
    }

    /// Convert LlmMessage to ClaudeMessage
    fn convert_to_claude_message(&self, message: &LlmMessage) -> ClaudeMessage {
        let content = if message.role == "tool" {
            // Handle tool result messages - parse the stored tool result data
            if let Some(content_str) = &message.content {
                if let Ok(tool_result_data) = serde_json::from_str::<serde_json::Value>(content_str)
                {
                    // Check for Claude-specific tool result format (with tool_use_id)
                    if let (Some(tool_use_id), Some(content_value)) = (
                        tool_result_data.get("tool_use_id").and_then(|v| v.as_str()),
                        tool_result_data.get("content"),
                    ) {
                        // Convert the content value to a string
                        let content_string = match content_value {
                            serde_json::Value::String(s) => s.clone(),
                            _ => content_value.to_string(),
                        };

                        return ClaudeMessage {
                            role: "user".to_string(), // Tool results are sent as user messages in Claude
                            content: vec![ClaudeContentBlock::ToolResult {
                                tool_use_id: tool_use_id.to_string(),
                                content: content_string,
                                is_error: None,
                            }],
                        };
                    }
                    // If no tool_use_id, treat as simple tool result content
                    else {
                        return ClaudeMessage {
                            role: "user".to_string(),
                            content: vec![ClaudeContentBlock::Text {
                                text: tool_result_data.to_string(),
                            }],
                        };
                    }
                }
            }
            // Fallback for malformed tool results
            vec![ClaudeContentBlock::Text {
                text: message.content.as_deref().unwrap_or("").to_string(),
            }]
        } else if message.role == "assistant" {
            // Handle assistant messages - check if they contain structured tool call data
            if let Some(content_str) = &message.content {
                // Try to parse as structured tool call data first
                if let Ok(assistant_data) = serde_json::from_str::<serde_json::Value>(content_str) {
                    if let (Some(text), Some(tool_calls_array)) = (
                        assistant_data.get("text").and_then(|v| v.as_str()),
                        assistant_data.get("tool_calls").and_then(|v| v.as_array()),
                    ) {
                        // Build content blocks with text + tool_use blocks
                        let mut content_blocks = vec![];

                        // Add text block if present
                        if !text.trim().is_empty() {
                            content_blocks.push(ClaudeContentBlock::Text {
                                text: text.to_string(),
                            });
                        }

                        // Add tool_use blocks
                        for tool_call in tool_calls_array {
                            if let (Some(id), Some(name), Some(args_str)) = (
                                tool_call.get("id").and_then(|v| v.as_str()),
                                tool_call
                                    .get("function")
                                    .and_then(|f| f.get("name"))
                                    .and_then(|v| v.as_str()),
                                tool_call
                                    .get("function")
                                    .and_then(|f| f.get("arguments"))
                                    .and_then(|v| v.as_str()),
                            ) {
                                if let Ok(input) =
                                    serde_json::from_str::<serde_json::Value>(args_str)
                                {
                                    content_blocks.push(ClaudeContentBlock::ToolUse {
                                        id: id.to_string(),
                                        name: name.to_string(),
                                        input,
                                    });
                                }
                            }
                        }

                        return ClaudeMessage {
                            role: message.role.clone(),
                            content: content_blocks,
                        };
                    }
                }
                // Fallback to text content
                vec![ClaudeContentBlock::Text {
                    text: content_str.clone(),
                }]
            } else {
                vec![]
            }
        } else if let Some(content_str) = &message.content {
            vec![ClaudeContentBlock::Text {
                text: content_str.clone(),
            }]
        } else if let Some(tool_calls) = &message.tool_calls {
            // Convert tool calls to tool_result blocks (this seems wrong, but keeping for compatibility)
            tool_calls
                .iter()
                .map(|tool_call| ClaudeContentBlock::ToolResult {
                    tool_use_id: tool_call.id.clone(),
                    content: format!(
                        "Tool call: {} with args {}",
                        tool_call.function.name, tool_call.function.arguments
                    ),
                    is_error: None,
                })
                .collect()
        } else {
            vec![]
        };

        ClaudeMessage {
            role: message.role.clone(),
            content,
        }
    }

    /// Convert LlmCompletionRequest to ClaudeCompletionRequest
    fn convert_request(&self, request: LlmCompletionRequest) -> ClaudeCompletionRequest {
        // Separate system messages from regular messages
        let mut system_content = String::new();
        let mut regular_messages = Vec::new();

        for message in &request.messages {
            if message.role == "system" {
                if let Some(content) = &message.content {
                    if !system_content.is_empty() {
                        system_content.push('\n');
                    }
                    system_content.push_str(content);
                }
            } else {
                regular_messages.push(self.convert_to_claude_message(message));
            }
        }

        let tools = if self.supports_native_tools() && request.tools.is_some() {
            Some(
                request
                    .tools
                    .unwrap()
                    .into_iter()
                    .map(|tool| ClaudeToolDefinition {
                        name: tool.function.name,
                        description: tool.function.description,
                        input_schema: tool.function.parameters,
                    })
                    .collect(),
            )
        } else {
            None
        };

        let tool_choice = if self.supports_native_tools() && request.tool_choice.is_some() {
            match request.tool_choice.unwrap() {
                ToolChoice::Auto(_) => Some(ClaudeToolChoice::Auto {
                    r#type: "auto".to_string(),
                }),
                ToolChoice::Required(_) => Some(ClaudeToolChoice::Any {
                    r#type: "any".to_string(),
                }),
                ToolChoice::Specific { function, .. } => Some(ClaudeToolChoice::Tool {
                    r#type: "tool".to_string(),
                    name: function.name,
                }),
                ToolChoice::None(_) => None,
            }
        } else {
            None
        };

        ClaudeCompletionRequest {
            model: request.model,
            messages: regular_messages,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            system: if system_content.is_empty() {
                None
            } else {
                Some(system_content)
            },
            tools,
            tool_choice,
        }
    }

    /// Convert ClaudeCompletionResponse to LlmCompletionResponse
    fn convert_response(&self, response: ClaudeCompletionResponse) -> LlmCompletionResponse {
        // Extract text content and tool calls from Claude content blocks
        let mut content = String::new();
        let mut tool_calls = Vec::new();

        for block in response.content {
            match block {
                ClaudeContentBlock::Text { text } => {
                    content.push_str(&text);
                }
                ClaudeContentBlock::ToolUse { id, name, input } => {
                    tool_calls.push(LlmToolCall {
                        id,
                        r#type: "function".to_string(),
                        function: LlmToolCallFunction {
                            name,
                            arguments: serde_json::to_string(&input).unwrap_or_default(),
                        },
                    });
                }
                ClaudeContentBlock::ToolResult { .. } => {
                    // Tool results are handled in the message conversion
                }
            }
        }

        LlmCompletionResponse {
            id: response.id,
            object: "chat.completion".to_string(), // Mimic OpenAI format
            created: 0,                            // Claude doesn't provide this
            model: response.model,
            choices: vec![LlmChoice {
                index: 0,
                message: Some(LlmMessage {
                    role: "assistant".to_string(),
                    content: if content.is_empty() {
                        None
                    } else {
                        Some(content)
                    },
                    tool_calls: if tool_calls.is_empty() {
                        None
                    } else {
                        Some(tool_calls)
                    },
                    function_call: None,
                    tool_call_id: None,
                }),
                delta: None,
                finish_reason: response.stop_reason.map(|reason| match reason.as_str() {
                    "end_turn" => "stop".to_string(),
                    "max_tokens" => "length".to_string(),
                    "stop_sequence" => "stop".to_string(),
                    "tool_use" => "tool_calls".to_string(),
                    _ => "stop".to_string(),
                }),
                tool_calls: None, // Claude puts tool calls in the message, not at choice level
            }],
            usage: Some(LlmUsage {
                prompt_tokens: response.usage.input_tokens,
                completion_tokens: response.usage.output_tokens,
                total_tokens: response.usage.input_tokens + response.usage.output_tokens,
            }),
        }
    }
}

#[async_trait]
impl LlmClient for ClaudeClient {
    async fn complete(&self, request: LlmCompletionRequest) -> Result<LlmCompletionResponse> {
        // Apply config defaults
        let mut request = request;
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
            "Sending Claude completion request"
        );

        // Convert to Claude format
        let claude_request = self.convert_request(request.clone());

        // Log the raw request
        let request_json = serde_json::to_string_pretty(&claude_request)
            .unwrap_or_else(|_| "Failed to serialize request".to_string());
        tracing::info!(
            provider = %self.config.provider,
            model = %request.model,
            api_url = %self.get_api_url(),
            raw_request = %request_json,
            "Raw Claude request being sent"
        );

        // Make the request
        let response = self
            .client
            .post(self.get_api_url())
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&claude_request)
            .send()
            .await?;

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
                "Claude API request failed"
            );

            return Err(anyhow::anyhow!(
                "Claude API error: {} - {}",
                status,
                error_text
            ));
        }

        // Parse response
        let response_text = response.text().await?;

        // Log raw response for debugging
        tracing::debug!(
            provider = %self.config.provider,
            model = %request.model,
            status = %status,
            response_time_ms = %response_time.as_millis(),
            raw_response = %response_text,
            "Raw Claude API response received"
        );

        // Check if response contains an error
        if let Ok(error_response) = serde_json::from_str::<serde_json::Value>(&response_text) {
            if let Some(error) = error_response.get("error") {
                tracing::error!(
                    provider = %self.config.provider,
                    model = %request.model,
                    status = %status,
                    response_time_ms = %response_time.as_millis(),
                    error = %error,
                    raw_response = %response_text,
                    "Claude API returned error in response body"
                );
                return Err(anyhow::anyhow!("Claude API error: {}", error));
            }
        }

        let claude_response: ClaudeCompletionResponse = match serde_json::from_str(&response_text) {
            Ok(response) => response,
            Err(e) => {
                tracing::error!(
                    provider = %self.config.provider,
                    model = %request.model,
                    status = %status,
                    response_time_ms = %response_time.as_millis(),
                    parse_error = %e,
                    raw_response = %response_text,
                    "Failed to parse Claude API response"
                );
                return Err(anyhow::anyhow!(
                    "Failed to parse Claude API response: {} - Response: {}",
                    e,
                    response_text
                ));
            }
        };

        tracing::info!(
            provider = %self.config.provider,
            model = %request.model,
            status = %status,
            response_time_ms = %response_time.as_millis(),
            completion_id = %claude_response.id,
            usage = ?claude_response.usage,
            "Claude API request completed successfully"
        );

        // Convert back to standard format
        let llm_response = self.convert_response(claude_response);
        Ok(llm_response)
    }

    fn stream_complete(
        &self,
        _request: LlmCompletionRequest,
    ) -> Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>> {
        // TODO: Implement streaming for Claude
        // For now, return an error stream
        Box::pin(futures_util::stream::once(async {
            Err(anyhow::anyhow!("Claude streaming not yet implemented"))
        }))
    }

    fn extract_tool_calls_from_response(
        &self,
        response: &LlmCompletionResponse,
    ) -> Vec<LlmToolCall> {
        // Tool calls are already extracted in convert_response
        response
            .choices
            .first()
            .and_then(|choice| choice.message.as_ref())
            .and_then(|message| message.tool_calls.clone())
            .unwrap_or_default()
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
        "openai" | "grok" => {
            let client = OpenAiCompatibleClient::new(config)?;
            Ok(Box::new(client))
        }
        "anthropic" | "claude" => {
            let client = ClaudeClient::new(config)?;
            Ok(Box::new(client))
        }
        _ => Err(anyhow::anyhow!(
            "Unsupported LLM provider: {}",
            config.provider
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grok_supports_native_tools() {
        let config = LlmProviderConfig {
            provider: "grok".to_string(),
            model: "grok-code-fast-1".to_string(),
            api_key: "test".to_string(),
            base_url: Some("https://api.x.ai".to_string()),
            max_tokens: Some(1000),
            temperature: Some(0.7),
        };

        let client = OpenAiCompatibleClient::new(config).unwrap();

        // Test that grok-code-fast-1 supports native tools
        assert!(client.supports_native_tools());

        // Test case insensitive
        let config_upper = LlmProviderConfig {
            provider: "GROK".to_string(),
            model: "grok-code-fast-1".to_string(),
            api_key: "test".to_string(),
            base_url: Some("https://api.x.ai".to_string()),
            max_tokens: Some(1000),
            temperature: Some(0.7),
        };

        let client_upper = OpenAiCompatibleClient::new(config_upper).unwrap();
        assert!(client_upper.supports_native_tools());

        // Test that older grok models don't support native tools
        let config_old = LlmProviderConfig {
            provider: "grok".to_string(),
            model: "grok-1".to_string(),
            api_key: "test".to_string(),
            base_url: Some("https://api.x.ai".to_string()),
            max_tokens: Some(1000),
            temperature: Some(0.7),
        };

        let client_old = OpenAiCompatibleClient::new(config_old).unwrap();
        assert!(!client_old.supports_native_tools());
    }
}
