use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};

use crate::{
    error::LlmError,
    ollama::types::{OllamaChatRequest, OllamaChatResponse, OllamaRole},
    tools::ProviderToolFormat,
};

/// Ollama local LLM client
pub struct OllamaClient {
    base_url: String,
    http_client: reqwest::Client,
}

impl OllamaClient {
    /// Create a new Ollama client with default base URL
    pub fn new() -> Result<Self, LlmError> {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .map_err(|e| LlmError::Network { source: e })?;

        Ok(Self {
            base_url: "http://localhost:11434".to_string(),
            http_client,
        })
    }

    /// Set a custom base URL for the API
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Start building a chat request
    pub fn message_builder(&self) -> crate::ollama::builder::OllamaMessageBuilder<'_> {
        crate::ollama::builder::OllamaMessageBuilder::new(self)
    }

    /// Create a chat message using the Ollama /api/chat endpoint
    pub async fn create_chat(
        &self,
        request: OllamaChatRequest,
    ) -> Result<OllamaChatResponse, LlmError> {
        let url = format!("{}/api/chat", self.base_url);

        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        // Debug: log the request if env var is set
        if std::env::var("NOCODO_LLM_LOG_PAYLOADS").is_ok() {
            if let Ok(json_str) = serde_json::to_string_pretty(&request) {
                eprintln!("=== Ollama Request ===\n{}\n=== End Request ===", json_str);
            }
        }

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
            let ollama_response: OllamaChatResponse = response
                .json()
                .await
                .map_err(|e| LlmError::internal(format!("Failed to parse response: {}", e)))?;

            // Debug: log the response if env var is set
            if std::env::var("NOCODO_LLM_LOG_PAYLOADS").is_ok() {
                if let Ok(json_str) = serde_json::to_string_pretty(&ollama_response) {
                    eprintln!("=== Ollama Response ===\n{}\n=== End Response ===", json_str);
                }
            }

            Ok(ollama_response)
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
impl crate::client::LlmClient for OllamaClient {
    async fn complete(
        &self,
        request: crate::types::CompletionRequest,
    ) -> Result<crate::types::CompletionResponse, LlmError> {
        let mut messages = Vec::new();

        // Add system message first if provided
        if let Some(system_prompt) = request.system {
            messages.push(crate::ollama::types::OllamaMessage::new(
                OllamaRole::System,
                system_prompt,
            ));
        }

        // Convert and append the rest of the messages
        let converted_messages = request
            .messages
            .into_iter()
            .map(|msg| {
                let role = match msg.role {
                    crate::types::Role::User => OllamaRole::User,
                    crate::types::Role::Assistant => OllamaRole::Assistant,
                    crate::types::Role::System => OllamaRole::System,
                };

                let content = msg
                    .content
                    .into_iter()
                    .map(|block| match block {
                        crate::types::ContentBlock::Text { text } => Ok(text),
                        crate::types::ContentBlock::Image { .. } => Err(LlmError::invalid_request(
                            "Image content not supported in Ollama client",
                        )),
                    })
                    .collect::<Result<Vec<String>, LlmError>>()?
                    .join("");

                Ok(crate::ollama::types::OllamaMessage::new(role, content))
            })
            .collect::<Result<Vec<crate::ollama::types::OllamaMessage>, LlmError>>()?;

        messages.extend(converted_messages);

        let mut options = crate::ollama::types::OllamaOptions::default();
        let mut has_options = false;

        if let Some(temp) = request.temperature {
            options.temperature = Some(temp);
            has_options = true;
        }
        if let Some(top_p) = request.top_p {
            options.top_p = Some(top_p);
            has_options = true;
        }
        if let Some(stop) = request.stop_sequences.clone() {
            options.stop = Some(crate::ollama::types::OllamaStop::Multiple(stop));
            has_options = true;
        }
        if request.max_tokens > 0 {
            options.num_predict = Some(request.max_tokens);
            has_options = true;
        }

        let tools = request.tools.as_ref().map(|tools| {
            tools
                .iter()
                .map(|tool| {
                    use crate::ollama::tools::OllamaToolFormat;
                    use crate::tools::ProviderToolFormat;

                    let mut ollama_tool = OllamaToolFormat::to_provider_tool(tool);

                    // Strip $schema field from parameters as it causes issues with Ollama
                    // Convert RootSchema to Value, remove $schema, and convert back
                    let params_value = serde_json::to_value(&ollama_tool.function.parameters)
                        .unwrap_or(serde_json::json!({}));

                    if let Some(mut params_obj) = params_value.as_object().cloned() {
                        params_obj.remove("$schema");
                        if let Ok(cleaned_schema) = serde_json::from_value(serde_json::Value::Object(params_obj)) {
                            ollama_tool.function.parameters = cleaned_schema;
                        }
                    }

                    ollama_tool
                })
                .collect::<Vec<_>>()
        });

        let format = request.response_format.map(|rf| match rf {
            crate::types::ResponseFormat::Text => None,
            crate::types::ResponseFormat::JsonObject => Some(crate::ollama::types::OllamaFormat::json()),
        }).flatten();

        let ollama_request = OllamaChatRequest {
            model: request.model,
            messages,
            tools,
            format,
            options: if has_options { Some(options) } else { None },
            stream: Some(false), // Explicitly disable streaming to get a single JSON response
            think: None,
            keep_alive: None,
            logprobs: None,
            top_logprobs: None,
        };

        let ollama_response = self.create_chat(ollama_request).await?;

        let content = vec![crate::types::ContentBlock::Text {
            text: ollama_response.message.content.clone(),
        }];

        let usage = crate::types::Usage {
            input_tokens: ollama_response.prompt_eval_count.unwrap_or(0),
            output_tokens: ollama_response.eval_count.unwrap_or(0),
        };

        let tool_calls = ollama_response.message.tool_calls.as_ref().map(|calls| {
            calls
                .iter()
                .enumerate()
                .map(|(idx, call)| {
                    crate::tools::ToolCall::new(
                        format!("ollama_tool_call_{}", idx),
                        call.function.name.clone(),
                        call.function.arguments.clone(),
                    )
                })
                .collect::<Vec<_>>()
        });

        Ok(crate::types::CompletionResponse {
            content,
            role: crate::types::Role::Assistant,
            usage,
            stop_reason: ollama_response.done_reason.clone(),
            tool_calls,
        })
    }

    fn provider_name(&self) -> &str {
        crate::providers::OLLAMA
    }

    fn model_name(&self) -> &str {
        "ollama"
    }
}
