use crate::models::LlmProviderConfig;
use anyhow::Result;
use async_stream::try_stream;
use async_trait::async_trait;
use futures_util::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::pin::Pin;

/// LLM message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    pub role: String,
    pub content: String,
}

/// LLM completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmCompletionRequest {
    pub model: String,
    pub messages: Vec<LlmMessage>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub stream: Option<bool>,
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
}

/// LLM message delta for streaming
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessageDelta {
    pub role: Option<String>,
    pub content: Option<String>,
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
#[derive(Debug, Clone)]
pub struct StreamChunk {
    pub content: String,
    pub is_finished: bool,
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
        let total_input_tokens: usize = request.messages.iter().map(|m| m.content.len()).sum();

        tracing::info!(
            provider = %self.config.provider,
            model = %request.model,
            message_count = %message_count,
            estimated_input_tokens = %total_input_tokens,
            max_tokens = ?request.max_tokens,
            temperature = ?request.temperature,
            "Sending non-streaming completion request to LLM provider"
        );

        let response = self.make_request(request.clone()).await?;

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
            "Sending streaming completion request to LLM provider"
        );

        Box::pin(try_stream! {
            let mut req = client
                .post(&api_url)
                .header("Authorization", format!("Bearer {}", config.api_key))
                .header("Content-Type", "application/json")
                .json(&request);

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
