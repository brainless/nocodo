use crate::models::LlmProviderConfig;

use anyhow::Result;
use async_trait::async_trait;
use futures_util::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

// Re-export model constants from SDK for backward compatibility
pub use nocodo_llm_sdk::claude::{HAIKU_4_5 as CLAUDE_HAIKU_4_5_MODEL_ID, OPUS_4_1 as CLAUDE_OPUS_4_1_MODEL_ID, SONNET_4_5 as CLAUDE_SONNET_4_5_MODEL_ID};

/// LLM message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    // Tool calls in message (for assistant responses)
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
    // Tool/Function parameters for native tool calling support
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

/// LLM message delta for streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessageDelta {
    pub role: Option<String>,
    pub content: Option<String>,
    // Tool calls in streaming delta
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_tokens: Option<u32>,
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
    // Tool calls in streaming chunk
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

/// Completion result with tool calls
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CompletionResult {
    pub response: LlmCompletionResponse,
    pub tool_calls: Vec<LlmToolCall>,
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

/// SDK-based LLM client implementation
pub struct SdkLlmClient {
    provider: String,
    model: String,
    inner: ClientType,
}

enum ClientType {
    OpenAI(nocodo_llm_sdk::openai::client::OpenAIClient),
    Claude(nocodo_llm_sdk::claude::client::ClaudeClient),
    Grok(nocodo_llm_sdk::grok::xai::XaiGrokClient),
    Glm(nocodo_llm_sdk::glm::cerebras::CerebrasGlmClient),
    ZenGrok(nocodo_llm_sdk::grok::zen::ZenGrokClient),
    ZenGlm(nocodo_llm_sdk::glm::zen::ZenGlmClient),
}

impl SdkLlmClient {
    pub fn new(config: &LlmProviderConfig) -> Result<Self> {
        let provider = config.provider.clone();
        let model = config.model.clone();

        let inner = match config.provider.to_lowercase().as_str() {
            "openai" => {
                let mut client = nocodo_llm_sdk::openai::client::OpenAIClient::new(&config.api_key)?;
                if let Some(base_url) = &config.base_url {
                    client = client.with_base_url(base_url);
                }
                ClientType::OpenAI(client)
            }
            "anthropic" | "claude" => {
                let client = nocodo_llm_sdk::claude::client::ClaudeClient::new(&config.api_key)?;
                ClientType::Claude(client)
            }
            "grok" | "xai" => {
                let client = nocodo_llm_sdk::grok::xai::XaiGrokClient::new(&config.api_key)?;
                ClientType::Grok(client)
            }
            "cerebras" | "zai" | "glm" => {
                let client = nocodo_llm_sdk::glm::cerebras::CerebrasGlmClient::new(&config.api_key)?;
                ClientType::Glm(client)
            }
            "zen-grok" | "zengrok" => {
                // Zen Grok is free, but can optionally use API key for paid models
                let client = if config.api_key.is_empty() {
                    nocodo_llm_sdk::grok::zen::ZenGrokClient::new()?
                } else {
                    nocodo_llm_sdk::grok::zen::ZenGrokClient::with_api_key(&config.api_key)?
                };
                ClientType::ZenGrok(client)
            }
            "zen-glm" | "zenglm" | "zen" => {
                // Zen GLM is free, but can optionally use API key for paid models
                let client = if config.api_key.is_empty() {
                    nocodo_llm_sdk::glm::zen::ZenGlmClient::new()?
                } else {
                    nocodo_llm_sdk::glm::zen::ZenGlmClient::with_api_key(&config.api_key)?
                };
                ClientType::ZenGlm(client)
            }
            _ => anyhow::bail!("Unsupported provider: {}", config.provider),
        };

        Ok(Self {
            provider,
            model,
            inner,
        })
    }
}

#[async_trait]
impl LlmClient for SdkLlmClient {
    async fn complete(&self, request: LlmCompletionRequest) -> Result<LlmCompletionResponse> {
        let created = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        match &self.inner {
            ClientType::OpenAI(client) => {
                // Check if this is a gpt-5-codex model (uses Responses API)
                if request.model.contains("gpt-5") && request.model.contains("codex") {
                    // Use response_builder() instead of message_builder()
                    let mut builder = client.response_builder().model(&request.model);

                    // Responses API uses "input" instead of messages
                    let input_text = request
                        .messages
                        .iter()
                        .filter_map(|m| m.content.as_ref())
                        .cloned()
                        .collect::<Vec<_>>()
                        .join("\n");

                    builder = builder.input(&input_text);

                    let response = builder.send().await.map_err(|e| anyhow::anyhow!("{}", e))?;

                    // Convert Responses API response to manager format
                    // Extract text from output items
                    let content = response
                        .output
                        .iter()
                        .filter(|item| item.item_type == "message")
                        .filter_map(|item| item.content.as_ref())
                        .flat_map(|blocks| blocks.iter())
                        .filter(|block| block.content_type == "output_text")
                        .map(|block| block.text.clone())
                        .collect::<Vec<_>>()
                        .join("\n");

                    return Ok(LlmCompletionResponse {
                        id: response.id,
                        object: "response".to_string(),
                        created,
                        model: request.model.clone(),
                        choices: vec![LlmChoice {
                            index: 0,
                            message: Some(LlmMessage {
                                role: "assistant".to_string(),
                                content: Some(content),
                                tool_calls: None,
                                function_call: None,
                                tool_call_id: None,
                            }),
                            delta: None,
                            finish_reason: Some("stop".to_string()),
                            tool_calls: None,
                        }],
                        usage: Some(LlmUsage {
                            prompt_tokens: response.usage.input_tokens.unwrap_or(0),
                            completion_tokens: response.usage.output_tokens.unwrap_or(0),
                            total_tokens: response.usage.total_tokens,
                            reasoning_tokens: None,
                        }),
                    });
                }

                // Regular chat completions
                let mut builder = client.message_builder().model(&request.model);

                // Add max_tokens
                if let Some(max_tokens) = request.max_tokens {
                    builder = builder.max_completion_tokens(max_tokens);
                }

                // Add temperature
                if let Some(temp) = request.temperature {
                    builder = builder.temperature(temp);
                }

                // Add messages
                for msg in &request.messages {
                    match msg.role.as_str() {
                        "system" => {
                            if let Some(content) = &msg.content {
                                builder = builder.system_message(content);
                            }
                        }
                        "user" => {
                            if let Some(content) = &msg.content {
                                builder = builder.user_message(content);
                            }
                        }
                        "assistant" => {
                            if let Some(content) = &msg.content {
                                builder = builder.assistant_message(content);
                            }
                            // TODO: Handle tool calls in assistant messages
                        }
                        "tool" => {
                            // TODO: Handle tool result messages
                        }
                        _ => {}
                    }
                }

                // Add tools if present
                if let Some(_tools) = &request.tools {
                    // TODO: Convert manager tool format to SDK format
                    // Use builder.add_tool() method
                }

                // Send request
                let response = builder.send().await.map_err(|e| anyhow::anyhow!("{}", e))?;

                // Convert SDK response to manager format
                Ok(LlmCompletionResponse {
                    id: response.id,
                    object: "chat.completion".to_string(),
                    created,
                    model: response.model,
                    choices: response
                        .choices
                        .into_iter()
                        .enumerate()
                        .map(|(idx, choice)| LlmChoice {
                            index: idx as u32,
                            message: Some(LlmMessage {
                                role: match choice.message.role {
                                    nocodo_llm_sdk::openai::types::OpenAIRole::System => {
                                        "system".to_string()
                                    }
                                    nocodo_llm_sdk::openai::types::OpenAIRole::User => {
                                        "user".to_string()
                                    }
                                    nocodo_llm_sdk::openai::types::OpenAIRole::Assistant => {
                                        "assistant".to_string()
                                    }
                                    nocodo_llm_sdk::openai::types::OpenAIRole::Tool => {
                                        "tool".to_string()
                                    }
                                },
                                content: Some(choice.message.content),
                                tool_calls: choice.message.tool_calls.map(|tcs| {
                                    tcs.into_iter()
                                        .map(|tc| LlmToolCall {
                                            id: tc.id,
                                            r#type: tc.r#type,
                                            function: LlmToolCallFunction {
                                                name: tc.function.name,
                                                arguments: tc.function.arguments,
                                            },
                                        })
                                        .collect()
                                }),
                                function_call: None,
                                tool_call_id: None,
                            }),
                            delta: None,
                            finish_reason: choice.finish_reason,
                            tool_calls: None,
                        })
                        .collect(),
                    usage: Some(LlmUsage {
                        prompt_tokens: response.usage.prompt_tokens.unwrap_or(0),
                        completion_tokens: response.usage.completion_tokens.unwrap_or(0),
                        total_tokens: response.usage.total_tokens,
                        reasoning_tokens: None,
                    }),
                })
            }

            ClientType::Claude(client) => {
                // Build Claude request using SDK builder
                let mut builder = client.message_builder().model(&request.model);

                // Add max_tokens (required for Claude)
                let max_tokens = request.max_tokens.unwrap_or(1024);
                builder = builder.max_tokens(max_tokens);

                // Add temperature
                if let Some(temp) = request.temperature {
                    builder = builder.temperature(temp);
                }

                // Add messages (Claude doesn't support system in messages array)
                for msg in &request.messages {
                    match msg.role.as_str() {
                        "system" => {
                            if let Some(content) = &msg.content {
                                builder = builder.system(content);
                            }
                        }
                        "user" => {
                            if let Some(content) = &msg.content {
                                builder = builder.user_message(content);
                            }
                        }
                        "assistant" => {
                            if let Some(content) = &msg.content {
                                builder = builder.assistant_message(content);
                            }
                        }
                        _ => {}
                    }
                }

                // Add tools if present
                if let Some(_tools) = &request.tools {
                    // TODO: Convert manager tool format to SDK format
                }

                // Send request
                let response = builder.send().await.map_err(|e| anyhow::anyhow!("{}", e))?;

                // Convert SDK response to manager format
                let content_text = response
                    .content
                    .iter()
                    .filter_map(|block| match block {
                        nocodo_llm_sdk::claude::types::ClaudeContentBlock::Text { text } => {
                            Some(text.clone())
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                Ok(LlmCompletionResponse {
                    id: response.id,
                    object: "message".to_string(),
                    created,
                    model: response.model,
                    choices: vec![LlmChoice {
                        index: 0,
                        message: Some(LlmMessage {
                            role: "assistant".to_string(),
                            content: Some(content_text),
                            tool_calls: None, // TODO: Extract tool uses from content blocks
                            function_call: None,
                            tool_call_id: None,
                        }),
                        delta: None,
                        finish_reason: response.stop_reason,
                        tool_calls: None,
                    }],
                    usage: Some(LlmUsage {
                        prompt_tokens: response.usage.input_tokens,
                        completion_tokens: response.usage.output_tokens,
                        total_tokens: response.usage.input_tokens + response.usage.output_tokens,
                        reasoning_tokens: None,
                    }),
                })
            }

            ClientType::Grok(client) => {
                // Similar to OpenAI implementation
                let mut builder = client.message_builder().model(&request.model);

                if let Some(max_tokens) = request.max_tokens {
                    builder = builder.max_tokens(max_tokens);
                }

                if let Some(temp) = request.temperature {
                    builder = builder.temperature(temp);
                }

                for msg in &request.messages {
                    match msg.role.as_str() {
                        "system" => {
                            if let Some(content) = &msg.content {
                                builder = builder.system_message(content);
                            }
                        }
                        "user" => {
                            if let Some(content) = &msg.content {
                                builder = builder.user_message(content);
                            }
                        }
                        "assistant" => {
                            if let Some(content) = &msg.content {
                                builder = builder.assistant_message(content);
                            }
                        }
                        _ => {}
                    }
                }

                let response = builder.send().await.map_err(|e| anyhow::anyhow!("{}", e))?;

                // Convert response (similar to OpenAI)
                Ok(LlmCompletionResponse {
                    id: response.id,
                    object: "chat.completion".to_string(),
                    created,
                    model: response.model,
                    choices: response
                        .choices
                        .into_iter()
                        .enumerate()
                        .map(|(idx, choice)| LlmChoice {
                            index: idx as u32,
                            message: Some(LlmMessage {
                                role: match choice.message.role {
                                    nocodo_llm_sdk::grok::types::GrokRole::System => {
                                        "system".to_string()
                                    }
                                    nocodo_llm_sdk::grok::types::GrokRole::User => {
                                        "user".to_string()
                                    }
                                    nocodo_llm_sdk::grok::types::GrokRole::Assistant => {
                                        "assistant".to_string()
                                    }
                                },
                                content: Some(choice.message.content),
                                tool_calls: None, // TODO: Handle tool calls
                                function_call: None,
                                tool_call_id: None,
                            }),
                            delta: None,
                            finish_reason: choice.finish_reason,
                            tool_calls: None,
                        })
                        .collect(),
                    usage: Some(LlmUsage {
                        prompt_tokens: response.usage.prompt_tokens,
                        completion_tokens: response.usage.completion_tokens,
                        total_tokens: response.usage.total_tokens,
                        reasoning_tokens: None,
                    }),
                })
            }

            ClientType::Glm(client) => {
                // Similar to OpenAI/Grok implementation
                let mut builder = client.message_builder().model(&request.model);

                if let Some(max_tokens) = request.max_tokens {
                    builder = builder.max_tokens(max_tokens);
                }

                if let Some(temp) = request.temperature {
                    builder = builder.temperature(temp);
                }

                for msg in &request.messages {
                    match msg.role.as_str() {
                        "system" => {
                            if let Some(content) = &msg.content {
                                builder = builder.system_message(content);
                            }
                        }
                        "user" => {
                            if let Some(content) = &msg.content {
                                builder = builder.user_message(content);
                            }
                        }
                        "assistant" => {
                            if let Some(content) = &msg.content {
                                builder = builder.assistant_message(content);
                            }
                        }
                        _ => {}
                    }
                }

                let response = builder.send().await.map_err(|e| anyhow::anyhow!("{}", e))?;

                // Convert response
                Ok(LlmCompletionResponse {
                    id: response.id,
                    object: "chat.completion".to_string(),
                    created,
                    model: response.model,
                    choices: response
                        .choices
                        .into_iter()
                        .enumerate()
                        .map(|(idx, choice)| LlmChoice {
                            index: idx as u32,
                            message: Some(LlmMessage {
                                role: match choice.message.role {
                                    nocodo_llm_sdk::glm::types::GlmRole::System => {
                                        "system".to_string()
                                    }
                                    nocodo_llm_sdk::glm::types::GlmRole::User => {
                                        "user".to_string()
                                    }
                                    nocodo_llm_sdk::glm::types::GlmRole::Assistant => {
                                        "assistant".to_string()
                                    }
                                },
                                content: choice.message.content,
                                tool_calls: None, // TODO: Handle tool calls
                                function_call: None,
                                tool_call_id: None,
                            }),
                            delta: None,
                            finish_reason: choice.finish_reason,
                            tool_calls: None,
                        })
                        .collect(),
                    usage: Some(LlmUsage {
                        prompt_tokens: response.usage.prompt_tokens,
                        completion_tokens: response.usage.completion_tokens,
                        total_tokens: response.usage.total_tokens,
                        reasoning_tokens: None,
                    }),
                })
            }

            ClientType::ZenGrok(client) => {
                // Zen Grok uses same API as regular Grok (Chat Completions)
                let mut builder = client.message_builder().model(&request.model);

                if let Some(max_tokens) = request.max_tokens {
                    builder = builder.max_tokens(max_tokens);
                }

                if let Some(temp) = request.temperature {
                    builder = builder.temperature(temp);
                }

                for msg in &request.messages {
                    match msg.role.as_str() {
                        "system" => {
                            if let Some(content) = &msg.content {
                                builder = builder.system_message(content);
                            }
                        }
                        "user" => {
                            if let Some(content) = &msg.content {
                                builder = builder.user_message(content);
                            }
                        }
                        "assistant" => {
                            if let Some(content) = &msg.content {
                                builder = builder.assistant_message(content);
                            }
                        }
                        _ => {}
                    }
                }

                let response = builder.send().await.map_err(|e| anyhow::anyhow!("{}", e))?;

                Ok(LlmCompletionResponse {
                    id: response.id,
                    object: "chat.completion".to_string(),
                    created,
                    model: response.model,
                    choices: response
                        .choices
                        .into_iter()
                        .enumerate()
                        .map(|(idx, choice)| LlmChoice {
                            index: idx as u32,
                            message: Some(LlmMessage {
                                role: match choice.message.role {
                                    nocodo_llm_sdk::grok::types::GrokRole::System => {
                                        "system".to_string()
                                    }
                                    nocodo_llm_sdk::grok::types::GrokRole::User => {
                                        "user".to_string()
                                    }
                                    nocodo_llm_sdk::grok::types::GrokRole::Assistant => {
                                        "assistant".to_string()
                                    }
                                },
                                content: Some(choice.message.content),
                                tool_calls: None,
                                function_call: None,
                                tool_call_id: None,
                            }),
                            delta: None,
                            finish_reason: choice.finish_reason,
                            tool_calls: None,
                        })
                        .collect(),
                    usage: Some(LlmUsage {
                        prompt_tokens: response.usage.prompt_tokens,
                        completion_tokens: response.usage.completion_tokens,
                        total_tokens: response.usage.total_tokens,
                        reasoning_tokens: None,
                    }),
                })
            }

            ClientType::ZenGlm(client) => {
                // Zen GLM uses same API as regular GLM (Chat Completions)
                let mut builder = client.message_builder().model(&request.model);

                if let Some(max_tokens) = request.max_tokens {
                    builder = builder.max_tokens(max_tokens);
                }

                if let Some(temp) = request.temperature {
                    builder = builder.temperature(temp);
                }

                for msg in &request.messages {
                    match msg.role.as_str() {
                        "system" => {
                            if let Some(content) = &msg.content {
                                builder = builder.system_message(content);
                            }
                        }
                        "user" => {
                            if let Some(content) = &msg.content {
                                builder = builder.user_message(content);
                            }
                        }
                        "assistant" => {
                            if let Some(content) = &msg.content {
                                builder = builder.assistant_message(content);
                            }
                        }
                        _ => {}
                    }
                }

                let response = builder.send().await.map_err(|e| anyhow::anyhow!("{}", e))?;

                Ok(LlmCompletionResponse {
                    id: response.id,
                    object: "chat.completion".to_string(),
                    created,
                    model: response.model,
                    choices: response
                        .choices
                        .into_iter()
                        .enumerate()
                        .map(|(idx, choice)| LlmChoice {
                            index: idx as u32,
                            message: Some(LlmMessage {
                                role: match choice.message.role {
                                    nocodo_llm_sdk::glm::types::GlmRole::System => {
                                        "system".to_string()
                                    }
                                    nocodo_llm_sdk::glm::types::GlmRole::User => {
                                        "user".to_string()
                                    }
                                    nocodo_llm_sdk::glm::types::GlmRole::Assistant => {
                                        "assistant".to_string()
                                    }
                                },
                                content: choice.message.content,
                                tool_calls: None,
                                function_call: None,
                                tool_call_id: None,
                            }),
                            delta: None,
                            finish_reason: choice.finish_reason,
                            tool_calls: None,
                        })
                        .collect(),
                    usage: Some(LlmUsage {
                        prompt_tokens: response.usage.prompt_tokens,
                        completion_tokens: response.usage.completion_tokens,
                        total_tokens: response.usage.total_tokens,
                        reasoning_tokens: None,
                    }),
                })
            }
        }
    }

    fn stream_complete(
        &self,
        _request: LlmCompletionRequest,
    ) -> Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>> {
        // Streaming not implemented yet - return error stream
        Box::pin(futures_util::stream::once(async {
            Err(anyhow::anyhow!("Streaming not yet implemented in SDK wrapper"))
        }))
    }

    fn extract_tool_calls_from_response(
        &self,
        response: &LlmCompletionResponse,
    ) -> Vec<LlmToolCall> {
        response
            .choices
            .first()
            .and_then(|c| c.message.as_ref())
            .and_then(|m| m.tool_calls.clone())
            .unwrap_or_default()
    }

    fn provider(&self) -> &str {
        &self.provider
    }

    fn model(&self) -> &str {
        &self.model
    }
}

/// Create an LLM client using the SDK
pub fn create_llm_client(config: LlmProviderConfig) -> Result<Box<dyn LlmClient>> {
    Ok(Box::new(SdkLlmClient::new(&config)?))
}

/// Factory function to create LLM clients with model information
#[allow(dead_code)]
pub fn create_llm_client_with_model(config: LlmProviderConfig) -> Result<Box<dyn LlmClient>> {
    // For now, just use the regular create_llm_client
    // TODO: Implement proper model-aware client creation
    create_llm_client(config)
}
