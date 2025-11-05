use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

use crate::llm_client::adapters::{ProviderAdapter, ProviderRequest};
use crate::llm_client::types::{
    ClaudeCompletionRequest, ClaudeCompletionResponse, ClaudeContentBlock, ClaudeMessage,
    ClaudeToolChoice, ClaudeToolDefinition,
};
use crate::llm_client::{
    LlmChoice, LlmCompletionRequest, LlmCompletionResponse, LlmMessage, LlmToolCall,
    LlmToolCallFunction, LlmUsage, ToolChoice,
};
use crate::models::LlmProviderConfig;

/// ClaudeMessagesAdapter - Handles Claude 4.5/4.1 models using native Messages API
pub struct ClaudeMessagesAdapter {
    config: LlmProviderConfig,
    client: reqwest::Client,
}

impl ClaudeMessagesAdapter {
    pub fn new(config: LlmProviderConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()?;
        Ok(Self { config, client })
    }

    /// Convert LlmMessage to ClaudeMessage
    fn convert_to_claude_message(&self, message: &LlmMessage) -> ClaudeMessage {
        let content = if message.role == "tool" {
            // Handle tool result messages - parse the stored tool result data
            if let Some(content_str) = &message.content {
                if let Ok(tool_result_data) = serde_json::from_str::<Value>(content_str) {
                    // Check for tool result format (tool_use_id for Claude, tool_call_id for OpenAI-compatible)
                    if let (Some(tool_use_id), Some(content_value)) = (
                        tool_result_data
                            .get("tool_use_id")
                            .or_else(|| tool_result_data.get("tool_call_id"))
                            .and_then(|v| v.as_str()),
                        tool_result_data.get("content"),
                    ) {
                        // Convert the content value to a string
                        let content_string = match content_value {
                            Value::String(s) => s.clone(),
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
            // Handle assistant messages - prioritize tool_calls field over content parsing
            let mut content_blocks = vec![];

            // Add text content if present
            if let Some(content_str) = &message.content {
                if !content_str.trim().is_empty() {
                    content_blocks.push(ClaudeContentBlock::Text {
                        text: content_str.clone(),
                    });
                }
            }

            // Add tool calls if present (from conversation reconstruction)
            if let Some(tool_calls) = &message.tool_calls {
                for tool_call in tool_calls {
                    if let Ok(input) = serde_json::from_str::<Value>(&tool_call.function.arguments)
                    {
                        content_blocks.push(ClaudeContentBlock::ToolUse {
                            id: tool_call.id.clone(),
                            name: tool_call.function.name.clone(),
                            input,
                        });
                    }
                }
            }

            // If no content blocks were created, try parsing content as JSON (fallback for old format)
            if content_blocks.is_empty() {
                if let Some(content_str) = &message.content {
                    if let Ok(assistant_data) = serde_json::from_str::<Value>(content_str) {
                        if let (Some(text), Some(tool_calls_array)) = (
                            assistant_data.get("text").and_then(|v| v.as_str()),
                            assistant_data.get("tool_calls").and_then(|v| v.as_array()),
                        ) {
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
                                    if let Ok(input) = serde_json::from_str::<Value>(args_str) {
                                        content_blocks.push(ClaudeContentBlock::ToolUse {
                                            id: id.to_string(),
                                            name: name.to_string(),
                                            input,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if content_blocks.is_empty() {
                // Final fallback
                vec![ClaudeContentBlock::Text {
                    text: message.content.as_deref().unwrap_or("").to_string(),
                }]
            } else {
                content_blocks
            }
        } else if let Some(content_str) = &message.content {
            vec![ClaudeContentBlock::Text {
                text: content_str.clone(),
            }]
        } else if let Some(tool_calls) = &message.tool_calls {
            // This shouldn't happen in normal flow, but keeping for safety
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

        // Convert tools
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

        // Convert tool choice
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

// Implement ProviderRequest for ClaudeCompletionRequest
impl ProviderRequest for ClaudeCompletionRequest {
    fn to_json(&self) -> Result<Value> {
        Ok(serde_json::to_value(self)?)
    }

    fn custom_headers(&self) -> Vec<(String, String)> {
        vec![("anthropic-version".to_string(), "2023-06-01".to_string())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::LlmProviderConfig;

    #[test]
    fn test_claude_adapter_creation() {
        let config = LlmProviderConfig {
            provider: "anthropic".to_string(),
            model: "claude-sonnet-4-5-20250929".to_string(),
            api_key: "test-key".to_string(),
            base_url: None,
            max_tokens: Some(1000),
            temperature: Some(0.7),
        };

        let adapter = ClaudeMessagesAdapter::new(config).unwrap();
        assert_eq!(adapter.provider_name(), "anthropic");
        assert_eq!(adapter.model_name(), "claude-sonnet-4-5-20250929");
        assert!(adapter.supports_native_tools());
    }

    #[test]
    fn test_message_conversion() {
        let config = LlmProviderConfig {
            provider: "anthropic".to_string(),
            model: "claude-sonnet-4-5-20250929".to_string(),
            api_key: "test-key".to_string(),
            base_url: None,
            max_tokens: Some(1000),
            temperature: Some(0.7),
        };

        let adapter = ClaudeMessagesAdapter::new(config).unwrap();

        // Test user message conversion
        let user_message = LlmMessage {
            role: "user".to_string(),
            content: Some("Hello Claude".to_string()),
            tool_calls: None,
            function_call: None,
            tool_call_id: None,
        };

        let claude_message = adapter.convert_to_claude_message(&user_message);
        assert_eq!(claude_message.role, "user");
        assert_eq!(claude_message.content.len(), 1);
        match &claude_message.content[0] {
            ClaudeContentBlock::Text { text } => assert_eq!(text, "Hello Claude"),
            _ => panic!("Expected text block"),
        }
    }

    #[test]
    fn test_assistant_message_with_tool_calls() {
        let config = LlmProviderConfig {
            provider: "anthropic".to_string(),
            model: "claude-sonnet-4-5-20250929".to_string(),
            api_key: "test-key".to_string(),
            base_url: None,
            max_tokens: Some(1000),
            temperature: Some(0.7),
        };

        let adapter = ClaudeMessagesAdapter::new(config).unwrap();

        // Test assistant message with tool calls (populated from conversation reconstruction)
        let tool_call = crate::llm_client::LlmToolCall {
            id: "call_123".to_string(),
            r#type: "function".to_string(),
            function: crate::llm_client::LlmToolCallFunction {
                name: "read_file".to_string(),
                arguments: r#"{"path":"test.txt"}"#.to_string(),
            },
        };

        let assistant_message = LlmMessage {
            role: "assistant".to_string(),
            content: Some("I'll read that file for you.".to_string()),
            tool_calls: Some(vec![tool_call]),
            function_call: None,
            tool_call_id: None,
        };

        let claude_message = adapter.convert_to_claude_message(&assistant_message);
        assert_eq!(claude_message.role, "assistant");
        assert_eq!(claude_message.content.len(), 2); // text + tool_use

        match &claude_message.content[0] {
            ClaudeContentBlock::Text { text } => assert_eq!(text, "I'll read that file for you."),
            _ => panic!("Expected text block first"),
        }

        match &claude_message.content[1] {
            ClaudeContentBlock::ToolUse { id, name, input } => {
                assert_eq!(id, "call_123");
                assert_eq!(name, "read_file");
                assert_eq!(input["path"], "test.txt");
            }
            _ => panic!("Expected tool_use block second"),
        }
    }
}

#[async_trait]
impl ProviderAdapter for ClaudeMessagesAdapter {
    fn get_api_url(&self) -> String {
        if let Some(base_url) = &self.config.base_url {
            format!("{}/v1/messages", base_url.trim_end_matches('/'))
        } else {
            "https://api.anthropic.com/v1/messages".to_string()
        }
    }

    fn supports_native_tools(&self) -> bool {
        true // All Claude 4.5/4.1 models support native tools
    }

    fn prepare_request(&self, request: LlmCompletionRequest) -> Result<Box<dyn ProviderRequest>> {
        // Apply config defaults
        let mut request = request;
        if request.max_tokens.is_none() {
            request.max_tokens = self.config.max_tokens;
        }
        if request.temperature.is_none() {
            request.temperature = self.config.temperature;
        }

        let claude_request = self.convert_request(request);
        Ok(Box::new(claude_request))
    }

    async fn send_request(&self, request: Box<dyn ProviderRequest>) -> Result<reqwest::Response> {
        let json = request.to_json()?;
        let headers = request.custom_headers();

        let mut req = self
            .client
            .post(self.get_api_url())
            .header("x-api-key", &self.config.api_key)
            .header("Content-Type", "application/json");

        // Add custom headers
        for (key, value) in headers {
            req = req.header(&key, &value);
        }

        let response = req.json(&json).send().await?;

        Ok(response)
    }

    fn parse_response(&self, response_text: &str) -> Result<LlmCompletionResponse> {
        let claude_response: ClaudeCompletionResponse = serde_json::from_str(response_text)?;
        Ok(self.convert_response(claude_response))
    }

    fn extract_tool_calls(&self, response: &LlmCompletionResponse) -> Vec<LlmToolCall> {
        // Tool calls are already in the message from conversion
        response
            .choices
            .first()
            .and_then(|choice| choice.message.as_ref())
            .and_then(|message| message.tool_calls.clone())
            .unwrap_or_default()
    }

    fn provider_name(&self) -> &str {
        &self.config.provider
    }

    fn model_name(&self) -> &str {
        &self.config.model
    }
}
