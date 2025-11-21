use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;

use crate::llm_client::adapters::trait_adapter::{ProviderAdapter, ProviderRequest};
use crate::llm_client::types::{GlmChatCompletionsRequest, GlmChatCompletionsResponse};
use crate::llm_client::{
    LlmChoice, LlmCompletionRequest, LlmCompletionResponse, LlmMessage, LlmToolCall,
    LlmToolCallFunction, LlmUsage,
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
    fn convert_to_glm_request(
        &self,
        request: LlmCompletionRequest,
    ) -> Result<GlmChatCompletionsRequest> {
        // Convert messages to GLM format (mostly compatible with OpenAI)
        let messages: Vec<Value> = request
            .messages
            .iter()
            .map(|msg| {
                let mut message = serde_json::json!({
                    "role": msg.role
                });

                // Add content field - required for most message types
                if let Some(content) = &msg.content {
                    message["content"] = Value::String(content.clone());
                } else if msg.role == "tool" {
                    // Tool messages require content field
                    message["content"] = Value::String(String::new());
                }

                // Handle tool calls for assistant messages
                if msg.role == "assistant" {
                    if let Some(tool_calls) = &msg.tool_calls {
                        message["tool_calls"] =
                            serde_json::to_value(tool_calls).unwrap_or(Value::Array(vec![]));
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

        // Convert tools to GLM format and fix required fields
        // IMPORTANT: If tools array is empty, set to None to avoid sending empty array which causes 400 error
        let tools = request.tools.as_ref().and_then(|tools| {
            if tools.is_empty() {
                tracing::info!("Tools array is empty, setting tools=None to avoid 400 error");
                return None;
            }

            Some(tools
                .iter()
                .map(|tool| {
                    let mut tool_value = serde_json::to_value(tool).unwrap_or(Value::Null);

                    // Fix the tool schema for GLM compatibility
                    if let Some(function) = tool_value.get_mut("function") {
                        // Get function name early for logging
                        let func_name = function.get("name").and_then(|n| n.as_str()).unwrap_or("unknown").to_string();

                        if let Some(params) = function.get_mut("parameters") {
                            // Collect field names that should NOT be required:
                            // 1. Fields with default values
                            // 2. Conditionally optional fields (search/replace in write_file, etc.)
                            let optional_fields: Vec<String> = params
                                .get("properties")
                                .and_then(|p| p.as_object())
                                .map(|properties| {
                                    properties
                                        .iter()
                                        .filter_map(|(name, prop)| {
                                            // Has a default value = optional
                                            if prop.get("default").is_some() {
                                                return Some(name.clone());
                                            }
                                            None
                                        })
                                        .collect()
                                })
                                .unwrap_or_default();

                            // Also remove known conditionally-optional fields
                            // These are fields that are only needed in certain contexts
                            let mut fields_to_remove = optional_fields;
                            fields_to_remove.extend(vec![
                                "search".to_string(),      // write_file: only for search-replace
                                "replace".to_string(),     // write_file: only for search-replace
                                "append".to_string(),      // write_file: optional mode
                                "create_dirs".to_string(), // write_file: optional mode
                                "create_if_not_exists".to_string(), // write_file: optional mode
                                "include_pattern".to_string(), // grep: optional file filter
                                "exclude_pattern".to_string(), // grep: optional file filter
                            ]);

                            // Remove optional fields from required array
                            if let Some(required) = params.get_mut("required").and_then(|r| r.as_array_mut()) {
                                let before = required.clone();
                                required.retain(|req_field| {
                                    if let Some(field_name) = req_field.as_str() {
                                        !fields_to_remove.contains(&field_name.to_string())
                                    } else {
                                        true
                                    }
                                });
                                let after = required.clone();

                                tracing::error!(
                                    "Tool schema fix for {}: required before: {:?}, after: {:?}, removed: {:?}",
                                    func_name, before, after, fields_to_remove
                                );
                            }

                            // Remove additionalProperties: false as it may be too strict
                            params.as_object_mut().and_then(|obj| obj.remove("additionalProperties"));
                        }
                    }

                    tool_value
                })
                .collect())
        });

        // Convert tool_choice - GLM only supports "auto" according to docs
        // IMPORTANT: Only set tool_choice if we have tools, otherwise None (to avoid 400 error)
        let tool_choice = if tools.is_some() {
            Some("auto".to_string())
        } else {
            None
        };

        Ok(GlmChatCompletionsRequest {
            model: request.model.clone(),
            messages,
            request_id: None, // Optional: could generate UUID
            tools,
            tool_choice,
            temperature: request.temperature, // Custom serializer handles rounding
            top_p: None,                      // Could add if needed
            max_tokens: request.max_tokens,
            stream: None,          // Omit stream parameter - let API use default
            thinking: None, // Disable thinking for now - might not be supported in Coding Plan
            response_format: None, // Default to text
        })
    }

    /// Convert GlmChatCompletionsResponse to LlmCompletionResponse
    fn convert_from_glm_response(
        &self,
        response: GlmChatCompletionsResponse,
    ) -> Result<LlmCompletionResponse> {
        let choices = response
            .choices
            .iter()
            .map(|choice| {
                // Convert tool calls
                let tool_calls = choice.message.tool_calls.as_ref().map(|tcs| {
                    tcs.iter()
                        .map(|tc| {
                            // Handle arguments that could be object or string
                            // Parse and re-serialize strings to normalize/deduplicate keys
                            let arguments = match &tc.function.arguments {
                                Value::String(s) => {
                                    // Parse and re-serialize to normalize (deduplicates keys)
                                    serde_json::from_str::<Value>(s)
                                        .and_then(|v| serde_json::to_string(&v))
                                        .unwrap_or_else(|_| s.clone())
                                }
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
            format!(
                "{}/paas/v4/chat/completions",
                base_url.trim_end_matches('/')
            )
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
        let api_url = self.get_api_url();

        tracing::info!(
            provider = self.provider_name(),
            model = self.model_name(),
            url = %api_url,
            "Sending request to GLM API"
        );

        // Log the full request for debugging (using error level to ensure it shows)
        tracing::error!(
            request = %serde_json::to_string_pretty(&json).unwrap_or_else(|_| "Failed to serialize".to_string()),
            "GLM API Request (DEBUG)"
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
