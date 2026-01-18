use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};

use crate::{
    error::LlmError,
    glm::types::{GlmChatCompletionRequest, GlmChatCompletionResponse},
};

/// Z.AI provider for GLM with support for regular and coding plan APIs
pub struct ZaiGlmClient {
    api_key: String,
    base_url: String,
    http_client: reqwest::Client,
    coding_plan: bool,
}

impl ZaiGlmClient {
    /// Create a new Z.AI GLM client with the given API key
    pub fn new(api_key: impl Into<String>) -> Result<Self, LlmError> {
        Self::with_coding_plan(api_key, false)
    }

    /// Create a new Z.AI GLM client with coding plan mode
    pub fn with_coding_plan(
        api_key: impl Into<String>,
        coding_plan: bool,
    ) -> Result<Self, LlmError> {
        let api_key = api_key.into();
        if api_key.is_empty() {
            return Err(LlmError::authentication("API key cannot be empty"));
        }

        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300)) // 5 minute timeout
            .build()
            .map_err(|e| LlmError::Network { source: e })?;

        let base_url = if coding_plan {
            "https://api.z.ai/api/coding/paas/v4".to_string()
        } else {
            "https://api.z.ai/api/paas/v4".to_string()
        };

        Ok(Self {
            api_key,
            base_url,
            http_client,
            coding_plan,
        })
    }

    /// Set a custom base URL for the API
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Check if this client is configured for coding plan mode
    pub fn is_coding_plan(&self) -> bool {
        self.coding_plan
    }

    /// Create a chat completion using the Z.AI GLM Chat Completions API
    pub async fn create_chat_completion(
        &self,
        request: ZaiChatCompletionRequest,
    ) -> Result<ZaiChatCompletionResponse, LlmError> {
        let url = format!("{}/chat/completions", self.base_url);

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
            let zai_response: ZaiChatCompletionResponse = response
                .json()
                .await
                .map_err(|e| LlmError::internal(format!("Failed to parse response: {}", e)))?;

            eprintln!("DEBUG - Parsed ZAI response: {:?}", zai_response);
            Ok(zai_response)
        } else {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            // Try to parse as ZAI error response
            if let Ok(error_response) = serde_json::from_str::<ZaiErrorResponse>(&error_text) {
                match status {
                    reqwest::StatusCode::BAD_REQUEST => {
                        // Check for insufficient balance error (code 1113) or authentication issues
                        if error_response
                            .error
                            .code
                            .as_ref()
                            .is_some_and(|code| *code == 1113)
                            || error_response
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

impl crate::glm::builder::GlmClientTrait for ZaiGlmClient {
    fn create_chat_completion(
        &self,
        request: GlmChatCompletionRequest,
    ) -> impl std::future::Future<Output = Result<GlmChatCompletionResponse, LlmError>> + Send {
        // Convert the standard GLM request to ZAI-specific request
        let zai_request = ZaiChatCompletionRequest {
            model: request.model,
            messages: request.messages.into_iter().map(ZaiMessage::from).collect(),
            request_id: None,
            do_sample: Some(true),
            stream: request.stream,
            thinking: None, // Can be added later if needed
            temperature: request.temperature,
            top_p: request.top_p,
            max_tokens: request.max_completion_tokens,
            tool_stream: None,
            tools: request.tools,
            tool_choice: request.tool_choice,
            stop: request.stop,
            response_format: request.response_format.map(|rf| match rf.format_type {
                crate::glm::types::GlmResponseFormatType::Text => ZaiResponseFormat::text(),
                crate::glm::types::GlmResponseFormatType::JsonObject => {
                    ZaiResponseFormat::json_object()
                }
            }),
            user_id: None,
        };

        async move {
            let response = self.create_chat_completion(zai_request).await?;
            // Convert ZAI response back to GLM response format
            Ok(GlmChatCompletionResponse {
                id: response.id,
                object: response.object,
                created: response.created,
                model: response.model,
                choices: response
                    .choices
                    .into_iter()
                    .map(|choice| crate::glm::types::GlmChoice {
                        index: choice.index,
                        message: crate::glm::types::GlmMessage {
                            role: match choice.message.role.as_str() {
                                "system" => crate::glm::types::GlmRole::System,
                                "user" => crate::glm::types::GlmRole::User,
                                "assistant" => crate::glm::types::GlmRole::Assistant,
                                _ => crate::glm::types::GlmRole::User,
                            },
                            content: choice.message.content.map(|v| match v {
                                serde_json::Value::String(s) => s,
                                _ => v.to_string(),
                            }),
                            reasoning: choice.message.reasoning_content, // Use reasoning_content from ZAI response
                            tool_calls: choice.message.tool_calls.map(|calls| {
                                calls
                                    .into_iter()
                                    .map(|call| crate::glm::types::GlmToolCall {
                                        id: call.id,
                                        r#type: call.r#type,
                                        function: crate::glm::types::GlmFunctionCall {
                                            name: call.function.name,
                                            arguments: match call.function.arguments {
                                                serde_json::Value::String(s) => s,
                                                _ => call.function.arguments.to_string(),
                                            },
                                        },
                                    })
                                    .collect()
                            }),
                            tool_call_id: None,
                        },
                        finish_reason: choice.finish_reason,
                    })
                    .collect(),
                usage: response.usage.map(|u| crate::glm::types::GlmUsage {
                    prompt_tokens: u.prompt_tokens,
                    completion_tokens: u.completion_tokens,
                    total_tokens: u.total_tokens,
                }),
            })
        }
    }
}

/// Z.AI-specific chat completion request
#[derive(Debug, Clone, serde::Serialize)]
pub struct ZaiChatCompletionRequest {
    /// The model to use for generation
    pub model: String,
    /// Input messages
    pub messages: Vec<ZaiMessage>,
    /// Unique ID for tracking (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// Whether to use sampling (default: true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub do_sample: Option<bool>,
    /// Whether to stream response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// Thinking mode configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ZaiThinking>,
    /// Temperature for randomness
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Top-p sampling parameter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Maximum number of tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Whether to stream tool calls
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_stream: Option<bool>,
    /// Available tools for model to use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<crate::glm::types::GlmTool>>,
    /// Tool choice strategy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<serde_json::Value>,
    /// Custom stop sequences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    /// Response format (text or json_object)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ZaiResponseFormat>,
    /// User ID for tracking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

/// Z.AI-specific message format
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ZaiMessage {
    /// Role of message sender
    pub role: String,
    /// Content of message (can be string for text or array for multimodal)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<serde_json::Value>,
    /// Reasoning content for ZAI coding plan responses
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
    /// Tool calls made by assistant
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ZaiToolCall>>,
    /// Tool call ID for tool result messages
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl ZaiMessage {
    /// Get text content from message
    pub fn get_text(&self) -> String {
        // First check content field
        match &self.content {
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(other) if !other.as_str().unwrap_or("").trim().is_empty() => other.to_string(),
            Some(_) => {
                // content is empty/non-string, check reasoning_content
                self.reasoning_content.clone().unwrap_or_default()
            }
            None => {
                // content is None, check reasoning_content
                self.reasoning_content.clone().unwrap_or_default()
            }
        }
    }
}

impl From<crate::glm::types::GlmMessage> for ZaiMessage {
    fn from(glm_msg: crate::glm::types::GlmMessage) -> Self {
        Self {
            role: match glm_msg.role {
                crate::glm::types::GlmRole::System => "system".to_string(),
                crate::glm::types::GlmRole::User => "user".to_string(),
                crate::glm::types::GlmRole::Assistant => "assistant".to_string(),
            },
            content: glm_msg.content.map(serde_json::Value::String),
            reasoning_content: glm_msg.reasoning,
            tool_calls: glm_msg.tool_calls.map(|calls| {
                calls
                    .into_iter()
                    .map(|call| ZaiToolCall {
                        id: call.id,
                        r#type: call.r#type,
                        function: ZaiFunctionCall {
                            name: call.function.name,
                            arguments: serde_json::Value::String(call.function.arguments),
                        },
                    })
                    .collect()
            }),
            tool_call_id: glm_msg.tool_call_id,
        }
    }
}

/// Z.AI thinking mode configuration
#[derive(Debug, Clone, serde::Serialize)]
pub struct ZaiThinking {
    /// Whether to enable thinking mode
    #[serde(rename = "type")]
    pub thinking_type: ZaiThinkingType,
}

impl ZaiThinking {
    /// Create an enabled thinking configuration
    pub fn enabled() -> Self {
        Self {
            thinking_type: ZaiThinkingType::Enabled,
        }
    }

    /// Create a disabled thinking configuration
    pub fn disabled() -> Self {
        Self {
            thinking_type: ZaiThinkingType::Disabled,
        }
    }
}

/// Thinking mode types
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ZaiThinkingType {
    /// Enable thinking mode
    Enabled,
    /// Disable thinking mode
    Disabled,
}

/// Z.AI response format
#[derive(Debug, Clone, serde::Serialize)]
pub struct ZaiResponseFormat {
    /// Response format type
    #[serde(rename = "type")]
    pub format_type: ZaiResponseType,
}

impl ZaiResponseFormat {
    /// Create a text response format
    pub fn text() -> Self {
        Self {
            format_type: ZaiResponseType::Text,
        }
    }

    /// Create a JSON object response format
    pub fn json_object() -> Self {
        Self {
            format_type: ZaiResponseType::JsonObject,
        }
    }
}

/// Response format types
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ZaiResponseType {
    /// Plain text response
    Text,
    /// JSON object response
    JsonObject,
}

/// Z.AI-specific tool call
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ZaiToolCall {
    /// Tool call ID
    pub id: String,
    /// Tool type
    pub r#type: String,
    /// Function call details
    pub function: ZaiFunctionCall,
}

/// Z.AI function call details
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ZaiFunctionCall {
    /// Function name
    pub name: String,
    /// Function arguments (JSON string or value)
    pub arguments: serde_json::Value,
}

/// Z.AI chat completion response
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ZaiChatCompletionResponse {
    /// Unique identifier for response
    pub id: String,
    /// Request ID
    pub request_id: Option<String>,
    /// Unix timestamp of creation
    pub created: u64,
    /// Model used for generation
    pub model: String,
    /// Object type (usually "chat.completion")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object: Option<String>,
    /// Completion choices
    pub choices: Vec<ZaiChoice>,
    /// Token usage information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<ZaiUsage>,
}

/// Z.AI completion choice
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ZaiChoice {
    /// Index of choice
    pub index: u32,
    /// The message content
    pub message: ZaiMessage,
    /// Reason why generation stopped
    #[serde(rename = "finish_reason")]
    pub finish_reason: Option<String>,
}

/// Z.AI token usage information
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ZaiUsage {
    /// Number of prompt tokens
    #[serde(rename = "prompt_tokens")]
    pub prompt_tokens: u32,
    /// Number of completion tokens
    #[serde(rename = "completion_tokens")]
    pub completion_tokens: u32,
    /// Total number of tokens
    #[serde(rename = "total_tokens")]
    pub total_tokens: u32,
}

/// Z.AI API error response
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ZaiErrorResponse {
    /// Error details
    pub error: ZaiError,
}

/// Z.AI API error details
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ZaiError {
    /// Error message
    pub message: String,
    /// Error type
    #[serde(rename = "type")]
    pub error_type: String,
    /// Error code
    pub code: Option<i32>,
}

// Message builder support
impl ZaiGlmClient {
    /// Start building a chat completion request
    pub fn message_builder(&self) -> crate::glm::builder::GlmMessageBuilder<'_, Self> {
        crate::glm::builder::GlmMessageBuilder::new(self)
    }
}

#[async_trait::async_trait]
impl crate::client::LlmClient for ZaiGlmClient {
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

        // Convert tools to ZAI format
        let tools = request.tools.map(|tools| {
            tools
                .into_iter()
                .map(|tool| crate::glm::types::GlmTool {
                    r#type: "function".to_string(),
                    function: crate::glm::types::GlmFunction {
                        name: tool.name().to_string(),
                        description: tool.description().to_string(),
                        parameters: tool.parameters().clone(),
                    },
                })
                .collect()
        });

        let zai_request = ZaiChatCompletionRequest {
            model: request.model,
            messages: glm_messages.into_iter().map(ZaiMessage::from).collect(),
            request_id: None,
            do_sample: Some(true),
            stream: None,
            thinking: Some(ZaiThinking::disabled()),
            temperature: request.temperature,
            top_p: request.top_p,
            max_tokens: Some(request.max_tokens),
            tool_stream: None,
            tools,
            tool_choice: request.tool_choice.map(|choice| match choice {
                crate::tools::ToolChoice::Auto => serde_json::json!("auto"),
                crate::tools::ToolChoice::Required => serde_json::json!("required"),
                crate::tools::ToolChoice::None => serde_json::json!(null),
                crate::tools::ToolChoice::Specific { name } => {
                    serde_json::json!({ "type": "function", "function": { "name": name } })
                }
            }),
            stop: request.stop_sequences,
            response_format: request.response_format.map(|rf| match rf {
                crate::types::ResponseFormat::Text => ZaiResponseFormat::text(),
                crate::types::ResponseFormat::JsonObject => ZaiResponseFormat::json_object(),
            }),
            user_id: None,
        };

        // Send request and convert response
        let zai_response = self.create_chat_completion(zai_request).await?;

        if zai_response.choices.is_empty() {
            return Err(LlmError::internal("No completion choices returned"));
        }

        let choice = &zai_response.choices[0];
        let content = vec![crate::types::ContentBlock::Text {
            text: choice.message.get_text(),
        }];

        // Extract tool calls from response
        let tool_calls = choice.message.tool_calls.as_ref().map(|calls| {
            calls
                .iter()
                .map(|call| {
                    let arguments = match &call.function.arguments {
                        serde_json::Value::String(s) => serde_json::from_str(s).unwrap_or_default(),
                        v => v.clone(),
                    };
                    crate::tools::ToolCall::new(
                        call.id.clone(),
                        call.function.name.clone(),
                        arguments,
                    )
                })
                .collect()
        });

        let response = crate::types::CompletionResponse {
            content,
            role: match choice.message.role.as_str() {
                "user" => crate::types::Role::User,
                "assistant" => crate::types::Role::Assistant,
                "system" => crate::types::Role::System,
                _ => crate::types::Role::Assistant, // Default to assistant
            },
            usage: crate::types::Usage {
                input_tokens: zai_response
                    .usage
                    .as_ref()
                    .map(|u| u.prompt_tokens)
                    .unwrap_or(0),
                output_tokens: zai_response
                    .usage
                    .as_ref()
                    .map(|u| u.completion_tokens)
                    .unwrap_or(0),
            },
            stop_reason: choice.finish_reason.clone(),
            tool_calls,
        };

        Ok(response)
    }

    fn provider_name(&self) -> &str {
        if self.coding_plan {
            "zai-coding"
        } else {
            "zai"
        }
    }

    fn model_name(&self) -> &str {
        // ZAI API expects "glm-4.6" not "zai-glm-4.6"
        // Note: The constant ZAI_GLM_4_6_ID is "zai-glm-4.6" which is for identification,
        // but the actual API call needs "glm-4.6"
        "glm-4.6"
    }
}
