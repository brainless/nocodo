use crate::{
    error::LlmError,
    types::{CompletionRequest, CompletionResponse, StreamChunk},
};
use async_trait::async_trait;
use futures_util::Stream;
use std::pin::Pin;

/// Core trait for LLM clients
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Complete a request (non-streaming)
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError>;

    /// Get provider name (e.g., "openai", "anthropic")
    fn provider_name(&self) -> &str;

    /// Get model name (e.g., "gpt-4o", "claude-sonnet-4-5")
    fn model_name(&self) -> &str;

    /// Check if streaming is supported
    fn supports_streaming(&self) -> bool {
        false
    }

    /// Stream completion (optional, returns error if not supported)
    fn stream_complete(
        &self,
        _request: CompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, LlmError>> + Send>>, LlmError> {
        Err(LlmError::not_supported("Streaming not supported"))
    }
}
