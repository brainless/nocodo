use crate::{
    error::LlmError,
    types::{CompletionRequest, CompletionResponse},
};

/// Core trait for LLM clients
#[allow(async_fn_in_trait)]
pub trait LlmClient {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError>;
    fn provider_name(&self) -> &str;
    fn model_name(&self) -> &str;
}
