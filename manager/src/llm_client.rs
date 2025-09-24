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
                let model_lower = self.config.model.to_lowercase();
                model_lower.contains("claude")
                    || model_lower.contains("opus")
                    || model_lower.contains("sonnet")
                    || model_lower.contains("haiku")
                    // Explicit support for current Claude model versions
                    || model_lower == "claude-3-5-sonnet-20241022"
                    || model_lower == "claude-3-5-haiku-20241022"
                    || model_lower == "claude-3-sonnet-20240229"
                    || model_lower == "claude-3-haiku-20240307"
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

/// Anthropic-specific request/response structures
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnthropicMessage {
    role: String,
    content: Vec<AnthropicContent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum AnthropicContent {
    Text { r#type: String, text: String },
    ToolUse { r#type: String, id: String, name: String, input: serde_json::Value },
    ToolResult { r#type: String, tool_use_id: String, content: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<AnthropicTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnthropicTool {
    name: String,
    description: String,
    input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnthropicResponse {
    id: String,
    r#type: String,
    role: String,
    content: Vec<AnthropicContent>,
    model: String,
    stop_reason: Option<String>,
    stop_sequence: Option<String>,
    usage: Option<AnthropicUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

/// Anthropic-specific LLM client
pub struct AnthropicClient {
    client: reqwest::Client,
    config: LlmProviderConfig,
}

impl AnthropicClient {
    pub fn new(config: LlmProviderConfig) -> Result<Self> {
        let client = reqwest::Client::new();
        Ok(Self { client, config })
    }

    fn get_api_url(&self) -> String {
        if let Some(base_url) = &self.config.base_url {
            format!("{}/v1/messages", base_url.trim_end_matches('/'))
        } else {
            "https://api.anthropic.com/v1/messages".to_string()
        }
    }

    /// Convert OpenAI-style request to Anthropic format
    fn convert_to_anthropic_request(&self, request: LlmCompletionRequest) -> Result<AnthropicRequest> {
        let mut anthropic_messages = Vec::new();
        let mut system_message = None;

        for message in request.messages {
            match message.role.as_str() {
                "system" => {
                    system_message = message.content;
                }
                "user" | "assistant" => {
                    let mut content = Vec::new();

                    // Handle tool results first (they take precedence)
                    if let Some(tool_call_id) = &message.tool_call_id {
                        if let Some(content_text) = &message.content {
                            content = vec![AnthropicContent::ToolResult {
                                r#type: "tool_result".to_string(),
                                tool_use_id: tool_call_id.clone(),
                                content: content_text.clone(),
                            }];
                        }
                    } else {
                        // Handle regular text content
                        if let Some(text) = &message.content {
                            if !text.is_empty() {
                                content.push(AnthropicContent::Text {
                                    r#type: "text".to_string(),
                                    text: text.clone(),
                                });
                            }
                        }

                        // Handle tool calls in assistant messages
                        if let Some(tool_calls) = message.tool_calls {
                            for tool_call in tool_calls {
                                if tool_call.r#type == "function" {
                                    let input: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
                                        .unwrap_or_else(|_| serde_json::json!({}));

                                    content.push(AnthropicContent::ToolUse {
                                        r#type: "tool_use".to_string(),
                                        id: tool_call.id,
                                        name: tool_call.function.name,
                                        input,
                                    });
                                }
                            }
                        }
                    }

                    if !content.is_empty() {
                        anthropic_messages.push(AnthropicMessage {
                            role: message.role,
                            content,
                        });
                    }
                }
                _ => {
                    // Skip unknown roles
                }
            }
        }

        // Convert tools
        let anthropic_tools = if let Some(tools) = request.tools {
            Some(tools.into_iter().map(|tool| {
                AnthropicTool {
                    name: tool.function.name,
                    description: tool.function.description,
                    input_schema: tool.function.parameters,
                }
            }).collect())
        } else {
            None
        };

        // Convert tool choice
        let anthropic_tool_choice = if let Some(tool_choice) = request.tool_choice {
            match tool_choice {
                ToolChoice::Auto(_) => Some(serde_json::json!({"type": "auto"})),
                ToolChoice::Required(_) => Some(serde_json::json!({"type": "any"})),
                ToolChoice::None(_) => None,
                ToolChoice::Specific { function, .. } => {
                    Some(serde_json::json!({"type": "tool", "name": function.name}))
                }
            }
        } else {
            None
        };

        Ok(AnthropicRequest {
            model: request.model,
            max_tokens: request.max_tokens.unwrap_or(self.config.max_tokens.unwrap_or(1024)),
            messages: anthropic_messages,
            system: system_message,
            tools: anthropic_tools,
            tool_choice: anthropic_tool_choice,
            temperature: request.temperature.or(self.config.temperature),
            stream: request.stream,
        })
    }

    /// Convert Anthropic response to OpenAI format
    fn convert_from_anthropic_response(&self, response: AnthropicResponse) -> LlmCompletionResponse {
        let mut message_content = String::new();
        let mut tool_calls = Vec::new();

        for content in response.content {
            match content {
                AnthropicContent::Text { text, .. } => {
                    if !message_content.is_empty() {
                        message_content.push(' ');
                    }
                    message_content.push_str(&text);
                }
                AnthropicContent::ToolUse { id, name, input, .. } => {
                    tool_calls.push(LlmToolCall {
                        id,
                        r#type: "function".to_string(),
                        function: LlmToolCallFunction {
                            name,
                            arguments: serde_json::to_string(&input).unwrap_or_else(|_| "{}".to_string()),
                        },
                    });
                }
                AnthropicContent::ToolResult { .. } => {
                    // Tool results are usually in user messages, not assistant messages
                }
            }
        }

        let message = LlmMessage {
            role: response.role,
            content: if message_content.is_empty() { None } else { Some(message_content) },
            tool_calls: if tool_calls.is_empty() { None } else { Some(tool_calls) },
            function_call: None,
            tool_call_id: None,
        };

        let usage = response.usage.map(|u| LlmUsage {
            prompt_tokens: u.input_tokens,
            completion_tokens: u.output_tokens,
            total_tokens: u.input_tokens + u.output_tokens,
        });

        LlmCompletionResponse {
            id: response.id,
            object: "chat.completion".to_string(),
            created: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            model: response.model,
            choices: vec![LlmChoice {
                index: 0,
                message: Some(message),
                delta: None,
                finish_reason: response.stop_reason,
                tool_calls: None,
            }],
            usage,
        }
    }

    async fn make_request(&self, request: AnthropicRequest) -> Result<reqwest::Response> {
        let req = self
            .client
            .post(self.get_api_url())
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request);

        Ok(req.send().await?)
    }
}

#[async_trait]
impl LlmClient for AnthropicClient {
    async fn complete(&self, request: LlmCompletionRequest) -> Result<LlmCompletionResponse> {
        let start_time = std::time::Instant::now();
        let message_count = request.messages.len();

        tracing::info!(
            provider = %self.config.provider,
            model = %request.model,
            message_count = %message_count,
            has_tools = %request.tools.is_some(),
            "Sending completion request to Anthropic API"
        );

        // Convert to Anthropic format
        let anthropic_request = self.convert_to_anthropic_request(request.clone())?;

        // Log the raw request being sent to Anthropic
        let request_json = serde_json::to_string_pretty(&anthropic_request)
            .unwrap_or_else(|_| "Failed to serialize request".to_string());
        tracing::info!(
            provider = %self.config.provider,
            model = %request.model,
            api_url = %self.get_api_url(),
            raw_request = %request_json,
            "Raw Anthropic request being sent"
        );

        let response = self.make_request(anthropic_request).await?;

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
                "Anthropic API request failed"
            );

            return Err(anyhow::anyhow!(
                "Anthropic API error: {} - {}",
                status,
                error_text
            ));
        }

        // Get the raw response text first for logging
        let response_text = response.text().await?;

        // Log the raw response received from Anthropic
        tracing::info!(
            provider = %self.config.provider,
            model = %request.model,
            status = %status,
            response_time_ms = %response_time.as_millis(),
            response_length = %response_text.len(),
            raw_response = %response_text,
            "Raw Anthropic response received"
        );

        // Parse the response
        let anthropic_response: AnthropicResponse = serde_json::from_str(&response_text)?;

        // Convert back to OpenAI format
        let completion = self.convert_from_anthropic_response(anthropic_response);

        tracing::info!(
            provider = %self.config.provider,
            model = %request.model,
            status = %status,
            response_time_ms = %response_time.as_millis(),
            completion_id = %completion.id,
            choices_count = %completion.choices.len(),
            usage = ?completion.usage,
            "Anthropic API request completed successfully"
        );

        Ok(completion)
    }

    fn stream_complete(
        &self,
        _request: LlmCompletionRequest,
    ) -> Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>> {
        // For now, return an error stream - we can implement streaming later if needed
        use futures_util::stream;
        Box::pin(stream::once(async {
            Err(anyhow::anyhow!("Streaming not yet implemented for Anthropic client"))
        }))
    }

    fn extract_tool_calls_from_response(
        &self,
        response: &LlmCompletionResponse,
    ) -> Vec<LlmToolCall> {
        let mut tool_calls = Vec::new();

        for choice in &response.choices {
            if let Some(message) = &choice.message {
                if let Some(message_tool_calls) = &message.tool_calls {
                    tool_calls.extend(message_tool_calls.clone());
                }
            }
        }

        tracing::info!(
            provider = %self.config.provider,
            total_tool_calls = %tool_calls.len(),
            "Extracted tool calls from Anthropic response"
        );

        tool_calls
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
            let client = AnthropicClient::new(config)?;
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
