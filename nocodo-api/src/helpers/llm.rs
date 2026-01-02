use anyhow::anyhow;
use std::env;
use std::sync::Arc;

pub fn create_llm_client() -> anyhow::Result<Arc<dyn nocodo_llm_sdk::client::LlmClient>> {
    let provider = env::var("LLM_PROVIDER").unwrap_or_else(|_| "anthropic".to_string());
    let api_key = env::var("ANTHROPIC_API_KEY")
        .or_else(|_| env::var("OPENAI_API_KEY"))
        .or_else(|_| env::var("XAI_API_KEY"))
        .map_err(|_| anyhow!("No API key found in environment variables. Set ANTHROPIC_API_KEY, OPENAI_API_KEY, or XAI_API_KEY."))?;

    let client: Arc<dyn nocodo_llm_sdk::client::LlmClient> = match provider.as_str() {
        "anthropic" => Arc::new(nocodo_llm_sdk::claude::ClaudeClient::new(api_key)?),
        "openai" => Arc::new(nocodo_llm_sdk::openai::OpenAIClient::new(api_key)?),
        "xai" => Arc::new(nocodo_llm_sdk::grok::xai::XaiGrokClient::new(api_key)?),
        _ => Arc::new(nocodo_llm_sdk::claude::ClaudeClient::new(api_key)?),
    };

    Ok(client)
}
