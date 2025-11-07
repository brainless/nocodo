use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;

use crate::llm_client::{
    LlmCompletionRequest, LlmCompletionResponse, LlmToolCall, LlmToolCallFunction, LlmMessage,
    LlmChoice, LlmUsage, ToolChoice,
};
use crate::llm_client::adapters::trait_adapter::{ProviderAdapter, ProviderRequest};
use crate::llm_client::types::{
    ResponsesApiRequest, ResponsesApiResponse, ResponsesToolDefinition,
    ResponseItem, ContentItem,
};
use crate::models::LlmProviderConfig;

/// Adapter for OpenAI's Responses API (used by GPT-5 and GPT-5-Codex)
pub struct ResponsesApiAdapter {
    config: LlmProviderConfig,
    client: Client,
}

impl ResponsesApiAdapter {
    pub fn new(config: LlmProviderConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()?;

        Ok(Self { config, client })
    }

    /// Get Codex-specific instructions for gpt-5-codex model
    fn get_codex_instructions(&self) -> String {
        r#"You are Codex, based on GPT-5. You are running as a coding agent in the Codex CLI on a user's computer.

## General

- The arguments to `shell` will be passed to execvp(). Most terminal commands should be prefixed with ["bash", "-lc"].
- Always set the `workdir` param when using the shell function. Do not use `cd` unless absolutely necessary.
- When searching for text or files, prefer using `rg` or `rg --files` respectively because `rg` is much faster than alternatives like `grep`. (If the `rg` command is not found, then use alternatives.)

## Editing constraints

- Default to ASCII when editing or creating files. Only introduce non-ASCII or other Unicode characters when there is a clear justification and the file already uses them.
- Add succinct code comments that explain what is going on if code is not self-explanatory. You should not add comments like "Assigns the value to the variable", but a brief comment might be useful ahead of a complex code block that the user would otherwise have to spend time parsing out. Usage of these comments should be rare.

## Plan tool

When using the planning tool:
- Skip using the planning tool for straightforward tasks (roughly the easiest 25%).
- Do not make single-step plans.
- When you made a plan, update it after having performed one of the sub-tasks that you shared on the plan.

## Codex CLI harness, sandboxing, and approvals

The Codex CLI harness supports different configurations for sandboxing and escalation approvals.

Filesystem sandboxing defines which files can be read or written. Network sandboxing defines whether network can be accessed without approval. Approvals are your mechanism to get user consent to run shell commands without the sandbox.

You will be told what filesystem sandboxing, network sandboxing, and approval mode are active. If you are not told about this, assume that you are running with workspace-write, network sandboxing enabled, and approval on-failure.

## Presenting your work and final message

You are producing plain text that will later be styled by the CLI. Be very concise; friendly coding teammate tone. Ask only when needed; suggest ideas; mirror the user's style.

For code changes: Lead with a quick explanation of the change, and then give more details on the context. If there are natural next steps, suggest them at the end.

File References: When referencing files, include the relevant start line and always follow the format: `file_path:line_number`."#.to_string()
    }

    /// Get default instructions for gpt-5 model
    fn get_default_instructions(&self) -> String {
        "You are a helpful AI assistant.".to_string()
    }

    /// Convert LlmCompletionRequest to ResponsesApiRequest
    fn convert_to_responses_request(&self, request: LlmCompletionRequest) -> Result<ResponsesApiRequest> {
        // Extract instructions from system messages or use model-specific instructions
        let instructions = if let Some(system_msg) = request.messages.iter().find(|m| m.role == "system") {
            system_msg.content.clone().unwrap_or_else(|| self.get_default_instructions())
        } else if self.config.model == "gpt-5-codex" {
            self.get_codex_instructions()
        } else {
            self.get_default_instructions()
        };

        // Convert messages to input array
        let mut input = Vec::new();

        for message in &request.messages {
            match message.role.as_str() {
                "system" => {
                    // System messages are handled in instructions, skip here
                    continue;
                }
                "user" => {
                    if let Some(content) = &message.content {
                        input.push(serde_json::json!({
                            "role": "user",
                            "content": content
                        }));
                    }
                }
                "assistant" => {
                    let mut msg_obj = serde_json::json!({"role": "assistant"});

                    // Always include content field - use empty string if no text content
                    let content = message.content.as_deref().unwrap_or("");
                    msg_obj["content"] = Value::String(content.to_string());

                    // Add tool calls if present (for conversation history)
                    if let Some(tool_calls) = &message.tool_calls {
                        let tool_calls_json: Vec<Value> = tool_calls
                            .iter()
                            .map(|tc| {
                                serde_json::json!({
                                    "id": tc.id,
                                    "type": "function",
                                    "function": {
                                        "name": tc.function.name,
                                        "arguments": tc.function.arguments
                                    }
                                })
                            })
                            .collect();
                        msg_obj["tool_calls"] = Value::Array(tool_calls_json);
                    }

                    input.push(msg_obj);
                }
                "tool" => {
                    // Tool results - add as function_call_output type
                    if let Some(content) = &message.content {
                        if let Some(tool_call_id) = &message.tool_call_id {
                            input.push(serde_json::json!({
                                "type": "function_call_output",
                                "call_id": tool_call_id,
                                "output": content
                            }));
                        } else {
                            // Fallback: treat as user message
                            input.push(serde_json::json!({
                                "role": "user",
                                "content": content
                            }));
                        }
                    }
                }
                _ => {
                    // Skip unknown roles
                }
            }
        }

        // Convert tools to ResponsesToolDefinition format
        let tools = request.tools.as_ref().map(|tools| {
            tools
                .iter()
                .map(|tool| {
                    ResponsesToolDefinition {
                        r#type: tool.r#type.clone(),
                        name: tool.function.name.clone(),
                        description: tool.function.description.clone(),
                        strict: true, // Enable strict mode for better tool calling
                        parameters: tool.function.parameters.clone(),
                    }
                })
                .collect()
        });

        // Determine tool_choice
        let tool_choice = match &request.tool_choice {
            Some(ToolChoice::None(_)) => "none".to_string(),
            Some(ToolChoice::Auto(_)) => "auto".to_string(),
            Some(ToolChoice::Required(_)) => "required".to_string(),
            Some(ToolChoice::Specific { .. }) => "required".to_string(), // For specific tools, use required
            None => "auto".to_string(),
        };

        Ok(ResponsesApiRequest {
            model: request.model,
            instructions,
            input,
            tools,
            tool_choice,
            stream: request.stream.unwrap_or(false),
        })
    }

    /// Convert ResponsesApiResponse to LlmCompletionResponse
    fn convert_from_responses_response(&self, response: ResponsesApiResponse) -> Result<LlmCompletionResponse> {
        tracing::debug!(
            "Converting Responses API response with {} output items",
            response.output.len()
        );

        let mut content_text = String::new();
        let mut tool_calls = Vec::new();

        // Aggregate all text content and collect all tool calls
        for item in &response.output {
            match item {
                ResponseItem::Message { content, .. } => {
                    for content_item in content {
                        if let ContentItem::OutputText { text, .. } = content_item {
                            if !content_text.is_empty() {
                                content_text.push('\n');
                            }
                            content_text.push_str(text);
                        }
                    }
                }
                ResponseItem::Reasoning { .. } => {
                    // Reasoning items are internal to the model and don't contribute to the final response
                    // Just skip them
                }
                ResponseItem::FunctionCall {
                    name,
                    arguments,
                    call_id,
                    ..
                } => {
                    tool_calls.push(LlmToolCall {
                        id: call_id.clone(),
                        r#type: "function".to_string(),
                        function: LlmToolCallFunction {
                            name: name.clone(),
                            arguments: arguments.clone(),
                        },
                    });
                }
            }
        }

        // Create a single choice with all aggregated content
        let choice = LlmChoice {
            index: 0,
            message: Some(LlmMessage {
                role: "assistant".to_string(),
                content: if content_text.is_empty() {
                    None
                } else {
                    Some(content_text)
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
            finish_reason: Some("stop".to_string()),
            tool_calls: None, // Tool calls are in the message, not at choice level
        };

        Ok(LlmCompletionResponse {
            id: response.id,
            object: "response".to_string(),
            created: 0, // Responses API doesn't provide this
            model: response.model,
            choices: vec![choice],
            usage: response.usage.map(|u| LlmUsage {
                prompt_tokens: u.input_tokens,
                completion_tokens: u.output_tokens,
                total_tokens: u.total_tokens,
            }),
        })
    }
}

impl ProviderRequest for ResponsesApiRequest {
    fn to_json(&self) -> Result<Value> {
        Ok(serde_json::to_value(self)?)
    }

    fn custom_headers(&self) -> Vec<(String, String)> {
        vec![
            ("OpenAI-Beta".to_string(), "responses=experimental".to_string()),
        ]
    }
}

#[async_trait]
impl ProviderAdapter for ResponsesApiAdapter {
    fn get_api_url(&self) -> String {
        if let Some(base_url) = &self.config.base_url {
            format!("{}/v1/responses", base_url.trim_end_matches('/'))
        } else {
            "https://api.openai.com/v1/responses".to_string()
        }
    }

    fn supports_native_tools(&self) -> bool {
        true // Both GPT-5 and GPT-5-Codex support native function calling
    }

    fn prepare_request(&self, request: LlmCompletionRequest) -> Result<Box<dyn ProviderRequest>> {
        let responses_request = self.convert_to_responses_request(request)?;
        Ok(Box::new(responses_request))
    }

    async fn send_request(&self, request: Box<dyn ProviderRequest>) -> Result<reqwest::Response> {
        let json = request.to_json()?;

        let mut request_builder = self.client
            .post(self.get_api_url())
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&json);

        // Add custom headers
        for (key, value) in request.custom_headers() {
            request_builder = request_builder.header(&key, &value);
        }

        let response = request_builder.send().await?;

        Ok(response)
    }

    fn parse_response(&self, response_text: &str) -> Result<LlmCompletionResponse> {
        let responses_response: ResponsesApiResponse = serde_json::from_str(response_text)?;
        self.convert_from_responses_response(responses_response)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::LlmProviderConfig;

    fn create_test_config(model: &str) -> LlmProviderConfig {
        LlmProviderConfig {
            provider: "openai".to_string(),
            model: model.to_string(),
            api_key: "test-key".to_string(),
            base_url: None,
            max_tokens: Some(1000),
            temperature: Some(0.7),
        }
    }

    #[test]
    fn test_get_api_url() {
        let config = create_test_config("gpt-5");
        let adapter = ResponsesApiAdapter::new(config).unwrap();

        assert_eq!(adapter.get_api_url(), "https://api.openai.com/v1/responses");
    }

    #[test]
    fn test_supports_native_tools() {
        let config = create_test_config("gpt-5");
        let adapter = ResponsesApiAdapter::new(config).unwrap();

        assert!(adapter.supports_native_tools());
    }

    #[test]
    fn test_convert_request_with_system_message() {
        let config = create_test_config("gpt-5");
        let adapter = ResponsesApiAdapter::new(config).unwrap();

        let request = LlmCompletionRequest {
            model: "gpt-5".to_string(),
            messages: vec![
                LlmMessage {
                    role: "system".to_string(),
                    content: Some("You are a helpful assistant.".to_string()),
                    tool_calls: None,
                    function_call: None,
                    tool_call_id: None,
                },
                LlmMessage {
                    role: "user".to_string(),
                    content: Some("Hello!".to_string()),
                    tool_calls: None,
                    function_call: None,
                    tool_call_id: None,
                },
            ],
            tools: None,
            tool_choice: None,
            max_tokens: Some(1000),
            temperature: Some(0.7),
            stream: Some(false),
            functions: None,
            function_call: None,
        };

        let responses_request = adapter.convert_to_responses_request(request).unwrap();

        assert_eq!(responses_request.instructions, "You are a helpful assistant.");
        assert_eq!(responses_request.input.len(), 1);
    }

    #[test]
    fn test_convert_request_codex_instructions() {
        let config = create_test_config("gpt-5-codex");
        let adapter = ResponsesApiAdapter::new(config).unwrap();

        let request = LlmCompletionRequest {
            model: "gpt-5-codex".to_string(),
            messages: vec![
                LlmMessage {
                    role: "user".to_string(),
                    content: Some("Hello!".to_string()),
                    tool_calls: None,
                    function_call: None,
                    tool_call_id: None,
                },
            ],
            tools: None,
            tool_choice: None,
            max_tokens: Some(1000),
            temperature: Some(0.7),
            stream: Some(false),
            functions: None,
            function_call: None,
        };

        let responses_request = adapter.convert_to_responses_request(request).unwrap();

        // Should use Codex-specific instructions
        assert!(responses_request.instructions.contains("Codex"));
    }

    #[test]
    fn test_parse_response_with_function_call() {
        let config = create_test_config("gpt-5");
        let adapter = ResponsesApiAdapter::new(config).unwrap();

        let response_json = r#"{
            "id": "resp_123",
            "model": "gpt-5",
            "output": [
                {
                    "type": "function_call",
                    "call_id": "call_abc123",
                    "name": "read_file",
                    "arguments": "{\"path\":\"test.txt\"}"
                }
            ]
        }"#;

        let llm_response = adapter.parse_response(response_json).unwrap();

        assert_eq!(llm_response.choices.len(), 1);
        let tool_calls = &llm_response.choices[0].message.as_ref().unwrap().tool_calls;
        assert!(tool_calls.is_some());
        assert_eq!(tool_calls.as_ref().unwrap().len(), 1);
        assert_eq!(tool_calls.as_ref().unwrap()[0].function.name, "read_file");
    }

    #[test]
    fn test_parse_response_with_text() {
        let config = create_test_config("gpt-5");
        let adapter = ResponsesApiAdapter::new(config).unwrap();

        let response_json = r#"{
            "id": "resp_123",
            "model": "gpt-5",
            "output": [
                {
                    "type": "message",
                    "role": "assistant",
                    "content": [
                        {
                            "type": "output_text",
                            "text": "Hello, world!"
                        }
                    ]
                }
            ]
        }"#;

        let llm_response = adapter.parse_response(response_json).unwrap();

        assert_eq!(llm_response.choices.len(), 1);
        let content = &llm_response.choices[0].message.as_ref().unwrap().content;
        assert_eq!(content.as_ref().unwrap(), "Hello, world!");
    }

    #[test]
    fn test_convert_request_assistant_message_without_content() {
        let config = create_test_config("gpt-5");
        let adapter = ResponsesApiAdapter::new(config).unwrap();

        let request = LlmCompletionRequest {
            model: "gpt-5".to_string(),
            messages: vec![
                LlmMessage {
                    role: "user".to_string(),
                    content: Some("Hello!".to_string()),
                    tool_calls: None,
                    function_call: None,
                    tool_call_id: None,
                },
                LlmMessage {
                    role: "assistant".to_string(),
                    content: None, // No text content
                    tool_calls: Some(vec![LlmToolCall {
                        id: "call_123".to_string(),
                        r#type: "function".to_string(),
                        function: LlmToolCallFunction {
                            name: "read_file".to_string(),
                            arguments: "{\"path\":\"test.txt\"}".to_string(),
                        },
                    }]),
                    function_call: None,
                    tool_call_id: None,
                },
            ],
            tools: None,
            tool_choice: None,
            max_tokens: Some(1000),
            temperature: Some(0.7),
            stream: Some(false),
            functions: None,
            function_call: None,
        };

        let responses_request = adapter.convert_to_responses_request(request).unwrap();

        // Should have 2 items in input array (user + assistant)
        assert_eq!(responses_request.input.len(), 2);

        // First item should be user message
        let user_msg = &responses_request.input[0];
        assert_eq!(user_msg["role"], "user");
        assert_eq!(user_msg["content"], "Hello!");

        // Second item should be assistant message with empty content and tool calls
        let assistant_msg = &responses_request.input[1];
        assert_eq!(assistant_msg["role"], "assistant");
        assert_eq!(assistant_msg["content"], ""); // Should be empty string
        assert!(assistant_msg.get("tool_calls").is_some());
    }

    #[test]
    fn test_factory_creates_responses_adapter() {
        use crate::llm_client::create_llm_client;
        use crate::models::LlmProviderConfig;

        // Test gpt-5-codex
        let config_codex = LlmProviderConfig {
            provider: "openai".to_string(),
            model: "gpt-5-codex".to_string(),
            api_key: "test-key".to_string(),
            base_url: None,
            max_tokens: Some(1000),
            temperature: Some(0.7),
        };

        let client_codex = create_llm_client(config_codex).unwrap();
        assert_eq!(client_codex.provider(), "openai");
        assert_eq!(client_codex.model(), "gpt-5-codex");

        // Test gpt-5
        let config_gpt5 = LlmProviderConfig {
            provider: "openai".to_string(),
            model: "gpt-5".to_string(),
            api_key: "test-key".to_string(),
            base_url: None,
            max_tokens: Some(1000),
            temperature: Some(0.7),
        };

        let client_gpt5 = create_llm_client(config_gpt5).unwrap();
        assert_eq!(client_gpt5.provider(), "openai");
        assert_eq!(client_gpt5.model(), "gpt-5");
    }
}