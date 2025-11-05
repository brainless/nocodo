use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

use crate::llm_client::{LlmCompletionRequest, LlmCompletionResponse, LlmToolCall};

/// Trait for provider-specific request serialization
pub trait ProviderRequest: Send + Sync {
    /// Serialize to JSON for HTTP request
    fn to_json(&self) -> Result<Value>;

    /// Get any custom headers needed for this provider
    fn custom_headers(&self) -> Vec<(String, String)> {
        vec![]
    }
}

/// Adapter trait for provider-specific LLM API handling
#[async_trait]
pub trait ProviderAdapter: Send + Sync {
    /// Get the API endpoint URL for this provider
    fn get_api_url(&self) -> String;

    /// Check if this provider supports native tool calling
    fn supports_native_tools(&self) -> bool;

    /// Convert unified request to provider-specific format
    fn prepare_request(&self, request: LlmCompletionRequest) -> Result<Box<dyn ProviderRequest>>;

    /// Send request to provider and get raw response
    async fn send_request(&self, request: Box<dyn ProviderRequest>) -> Result<reqwest::Response>;

    /// Convert provider-specific response to unified format
    fn parse_response(&self, response_text: &str) -> Result<LlmCompletionResponse>;

    /// Extract tool calls from response
    fn extract_tool_calls(&self, response: &LlmCompletionResponse) -> Vec<LlmToolCall>;

    /// Get provider name for logging/debugging
    fn provider_name(&self) -> &str;

    /// Get model name for logging/debugging
    fn model_name(&self) -> &str;
}
