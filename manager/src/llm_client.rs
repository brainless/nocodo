use crate::models::LlmProviderConfig;

use anyhow::Result;

// Re-export SDK types for backward compatibility and convenience
#[allow(unused_imports)] // Some types may be used in the future
pub use nocodo_llm_sdk::client::LlmClient;
pub use nocodo_llm_sdk::types::{CompletionRequest, ContentBlock, Message, Role};

// Re-export model constants for convenience
pub use nocodo_llm_sdk::claude::SONNET_4_5 as CLAUDE_SONNET_4_5_MODEL_ID;

/// Create an LLM client using SDK directly
pub fn create_llm_client(config: LlmProviderConfig) -> Result<Box<dyn LlmClient>> {
    match config.provider.to_lowercase().as_str() {
        "openai" => {
            let mut client = nocodo_llm_sdk::openai::client::OpenAIClient::new(&config.api_key)?;
            if let Some(base_url) = &config.base_url {
                client = client.with_base_url(base_url);
            }
            Ok(Box::new(client))
        }
        "anthropic" | "claude" => {
            let client = nocodo_llm_sdk::claude::client::ClaudeClient::new(&config.api_key)?;
            Ok(Box::new(client))
        }
        "grok" | "xai" => {
            let client = nocodo_llm_sdk::grok::xai::XaiGrokClient::new(&config.api_key)?;
            Ok(Box::new(client))
        }
        "cerebras" | "zai" | "glm" => {
            let client = nocodo_llm_sdk::glm::cerebras::CerebrasGlmClient::new(&config.api_key)?;
            Ok(Box::new(client))
        }
        "zen-grok" | "zengrok" => {
            let client = if config.api_key.is_empty() {
                nocodo_llm_sdk::grok::zen::ZenGrokClient::new()?
            } else {
                nocodo_llm_sdk::grok::zen::ZenGrokClient::with_api_key(&config.api_key)?
            };
            Ok(Box::new(client))
        }
        "zen-glm" | "zenglm" | "zen" => {
            let client = if config.api_key.is_empty() {
                nocodo_llm_sdk::glm::zen::ZenGlmClient::new()?
            } else {
                nocodo_llm_sdk::glm::zen::ZenGlmClient::with_api_key(&config.api_key)?
            };
            Ok(Box::new(client))
        }
        _ => anyhow::bail!("Unsupported provider: {}", config.provider),
    }
}

/// Factory function to create LLM clients with model information
#[allow(dead_code)]
pub fn create_llm_client_with_model(config: LlmProviderConfig) -> Result<Box<dyn LlmClient>> {
    // For now, just use regular create_llm_client
    // TODO: Implement proper model-aware client creation
    create_llm_client(config)
}
