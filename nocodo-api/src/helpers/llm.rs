use crate::config::ApiConfig;
use anyhow::anyhow;
use std::sync::Arc;

pub fn create_llm_client(
    config: &ApiConfig,
) -> anyhow::Result<Arc<dyn nocodo_llm_sdk::client::LlmClient>> {
    let api_keys = config.api_keys.as_ref().ok_or_else(|| {
        anyhow!("No API keys configured. Please configure API keys in the settings.")
    })?;

    let provider = config
        .llm
        .as_ref()
        .and_then(|llm| llm.provider.as_ref())
        .map(|s| s.as_str());

    let use_zai = api_keys.zai_api_key.is_some();

    let client: Arc<dyn nocodo_llm_sdk::client::LlmClient> = if use_zai {
        let api_key = api_keys
            .zai_api_key
            .as_ref()
            .ok_or_else(|| anyhow!("zai_api_key is required for zai provider"))?;

        let coding_plan = api_keys.zai_coding_plan.unwrap_or(true);

        Arc::new(nocodo_llm_sdk::glm::zai::ZaiGlmClient::with_coding_plan(
            api_key,
            coding_plan,
        )?)
    } else {
        let provider = provider.unwrap_or("anthropic");

        match provider {
            "anthropic" => {
                let api_key = api_keys.anthropic_api_key.as_ref().ok_or_else(|| {
                    anyhow!("anthropic_api_key is required for anthropic provider")
                })?;

                Arc::new(nocodo_llm_sdk::claude::ClaudeClient::new(api_key)?)
            }
            "openai" => {
                let api_key = api_keys
                    .openai_api_key
                    .as_ref()
                    .ok_or_else(|| anyhow!("openai_api_key is required for openai provider"))?;

                Arc::new(nocodo_llm_sdk::openai::OpenAIClient::new(api_key)?)
            }
            "xai" => {
                let api_key = api_keys
                    .xai_api_key
                    .as_ref()
                    .ok_or_else(|| anyhow!("xai_api_key is required for xai provider"))?;

                Arc::new(nocodo_llm_sdk::grok::xai::XaiGrokClient::new(api_key)?)
            }
            _ => Err(anyhow!(
                "Unsupported LLM provider: {}. Supported providers: anthropic, openai, xai, zai",
                provider
            ))?,
        }
    };

    Ok(client)
}
