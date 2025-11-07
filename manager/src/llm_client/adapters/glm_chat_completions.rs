use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;

use crate::llm_client::{
    LlmCompletionRequest, LlmCompletionResponse, LlmToolCall, LlmToolCallFunction,
    LlmMessage, LlmChoice, LlmUsage, ToolChoice,
};
use crate::llm_client::adapters::trait_adapter::{ProviderAdapter, ProviderRequest};
use crate::llm_client::types::{
    GlmChatCompletionsRequest, GlmChatCompletionsResponse, GlmThinkingConfig,
};
use crate::models::LlmProviderConfig;

/// Adapter for zAI's GLM models using Chat Completions API
pub struct GlmChatCompletionsAdapter {
    config: LlmProviderConfig,
    client: Client,
}

impl GlmChatCompletionsAdapter {
    pub fn new(config: LlmProviderConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()?;

        Ok(Self { config, client })
    }

    /// Convert LlmCompletionRequest to GlmChatCompletionsRequest
    fn convert_to_glm_request(&self, request: LlmCompletionRequest) -> Result<GlmChatCompletionsRequest> {
        // Convert messages to GLM format (mostly compatible with OpenAI)
        let messages: Vec<Value> = request
            .messages
            .iter()
            .map(|msg| {
                let mut message = serde_json::json!({
                    "role": msg.role
                });

                if let Some(content) = &msg.content {
                    message["content"] = Value::String(content.clone());
                }

                // Handle tool calls for assistant messages
                if msg.role == "assistant" {
                    if let Some(tool_calls) = &msg.tool_calls {
                        message["tool_calls"] = serde_json::to_value(tool_calls)
                            .unwrap_or(Value::Array(vec![]));
                    }
                }

                // Handle tool message with tool_call_id
                if msg.role == "tool" {
                    if let Some(tool_call_id) = &msg.tool_call_id {
                        message["tool_call_id"] = Value::String(tool_call_id.clone());
                    }
                }

                message
            })
            .collect();

        // Convert tools to GLM format (compatible with OpenAI format)
        let tools = request.tools.as_ref().map(|tools| {
            tools
                .iter()
                .map(|tool| serde_json::to_value(tool).unwrap_or(Value::Null))
                .collect()
        });

        // Convert tool_choice
        let tool_choice = match request.tool_choice {
            Some(ToolChoice::Auto(_)) => Some("auto".to_string()),
            Some(ToolChoice::None(_)) => Some("none".to_string()),
            Some(ToolChoice::Required(_)) => Some("required".to_string()),
            _ => Some("auto".to_string()),
        };

        Ok(GlmChatCompletionsRequest {
            model: request.model.clone(),
            messages,
            request_id: None, // Optional: could generate UUID
            tools,
            tool_choice,
            temperature: request.temperature,
            top_p: None, // Could add if needed
            max_tokens: request.max_tokens,
            stream: request.stream,
            thinking: Some(GlmThinkingConfig {
                r#type: "enabled".to_string(), // Enable chain-of-thought for GLM-4.6
            }),
            response_format: None, // Default to text
        })
    }

    /// Convert GlmChatCompletionsResponse to LlmCompletionResponse
    fn convert_from_glm_response(&self, response: GlmChatCompletionsResponse) -> Result<LlmCompletionResponse> {
        let choices = response
            .choices
            .iter()
            .map(|choice| {
                // Convert tool calls
                let tool_calls = choice.message.tool_calls.as_ref().map(|tcs| {
                    tcs.iter()
                        .map(|tc| {
                            // Handle arguments that could be object or string
                            let arguments = match &tc.function.arguments {
                                Value::String(s) => s.clone(),
                                Value::Object(_) => serde_json::to_string(&tc.function.arguments)
                                    .unwrap_or_else(|_| "{}".to_string()),
                                _ => "{}".to_string(),
                            };

                            LlmToolCall {
                                id: tc.id.clone(),
                                r#type: tc.r#type.clone(),
                                function: LlmToolCallFunction {
                                    name: tc.function.name.clone(),
                                    arguments,
                                },
                            }
                        })
                        .collect()
                });

                LlmChoice {
                    index: choice.index as u32,
                    message: Some(LlmMessage {
                        role: choice.message.role.clone(),
                        content: choice.message.content.clone(),
                        tool_calls,
                        function_call: None,
                        tool_call_id: None,
                    }),
                    delta: None,
                    finish_reason: Some(choice.finish_reason.clone()),
                    tool_calls: None,
                }
            })
            .collect();

        let usage = response.usage.map(|u| LlmUsage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        });

        Ok(LlmCompletionResponse {
            id: response.id,
            object: "chat.completion".to_string(),
            created: response.created as u64,
            model: response.model,
            choices,
            usage,
        })
    }
}

#[async_trait]
impl ProviderAdapter for GlmChatCompletionsAdapter {
    fn get_api_url(&self) -> String {
        // Support custom base_url for testing, otherwise use production
        if let Some(base_url) = &self.config.base_url {
            format!("{}/paas/v4/chat/completions", base_url.trim_end_matches('/'))
        } else {
            "https://api.z.ai/api/paas/v4/chat/completions".to_string()
        }
    }

    fn supports_native_tools(&self) -> bool {
        // GLM-4.6 series supports native function calling
        true
    }

    fn prepare_request(&self, request: LlmCompletionRequest) -> Result<Box<dyn ProviderRequest>> {
        let glm_request = self.convert_to_glm_request(request)?;
        Ok(Box::new(glm_request))
    }

    async fn send_request(&self, request: Box<dyn ProviderRequest>) -> Result<reqwest::Response> {
        let json = request.to_json()?;
        let custom_headers = request.custom_headers();

        tracing::debug!(
            provider = self.provider_name(),
            model = self.model_name(),
            "Sending request to GLM API"
        );

        let mut req_builder = self
            .client
            .post(self.get_api_url())
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json");

        // Add custom headers
        for (key, value) in custom_headers {
            req_builder = req_builder.header(key, value);
        }

        let response = req_builder.json(&json).send().await?;

        Ok(response)
    }

    fn parse_response(&self, response_text: &str) -> Result<LlmCompletionResponse> {
        let glm_response: GlmChatCompletionsResponse = serde_json::from_str(response_text)?;
        self.convert_from_glm_response(glm_response)
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