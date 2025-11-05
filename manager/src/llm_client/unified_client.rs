use anyhow::Result;
use async_trait::async_trait;
use futures_util::Stream;
use std::pin::Pin;

use crate::llm_client::adapters::ProviderAdapter;
use crate::llm_client::{
    LlmClient, LlmCompletionRequest, LlmCompletionResponse, LlmToolCall, StreamChunk,
};
use crate::models::LlmProviderConfig;

/// Unified LLM client that delegates to provider-specific adapters
pub struct UnifiedLlmClient {
    adapter: Box<dyn ProviderAdapter>,
    #[allow(dead_code)]
    config: LlmProviderConfig,
}

impl UnifiedLlmClient {
    pub fn new(adapter: Box<dyn ProviderAdapter>, config: LlmProviderConfig) -> Result<Self> {
        Ok(Self { adapter, config })
    }
}

#[async_trait]
impl LlmClient for UnifiedLlmClient {
    async fn complete(&self, request: LlmCompletionRequest) -> Result<LlmCompletionResponse> {
        let start_time = std::time::Instant::now();

        tracing::info!(
            provider = %self.adapter.provider_name(),
            model = %self.adapter.model_name(),
            message_count = %request.messages.len(),
            "Sending request via adapter"
        );

        // Prepare provider-specific request
        let provider_request = self.adapter.prepare_request(request)?;

        // Send request
        let response = self.adapter.send_request(provider_request).await?;

        let response_time = start_time.elapsed();
        let status = response.status();

        if !status.is_success() {
            let error_text = response.text().await?;
            tracing::error!(
                provider = %self.adapter.provider_name(),
                status = %status,
                error = %error_text,
                "Request failed"
            );
            return Err(anyhow::anyhow!("API error: {} - {}", status, error_text));
        }

        // Parse response
        let response_text = response.text().await?;

        tracing::info!(
            provider = %self.adapter.provider_name(),
            response_time_ms = %response_time.as_millis(),
            response_length = %response_text.len(),
            "Received response"
        );

        let llm_response = self.adapter.parse_response(&response_text)?;

        Ok(llm_response)
    }

    fn stream_complete(
        &self,
        _request: LlmCompletionRequest,
    ) -> Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>> {
        // TODO: Implement streaming via adapter
        Box::pin(futures_util::stream::once(async {
            Err(anyhow::anyhow!(
                "Streaming not yet implemented for unified client"
            ))
        }))
    }

    fn extract_tool_calls_from_response(
        &self,
        response: &LlmCompletionResponse,
    ) -> Vec<LlmToolCall> {
        self.adapter.extract_tool_calls(response)
    }

    fn provider(&self) -> &str {
        self.adapter.provider_name()
    }

    fn model(&self) -> &str {
        self.adapter.model_name()
    }
}
