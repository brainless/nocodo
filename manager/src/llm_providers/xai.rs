use crate::llm_client::{
    create_llm_client, LlmClient, LlmModel, LlmProvider, ModelCapabilities, ModelPricing,
    ProviderError, ProviderType,
};
use crate::models::LlmProviderConfig;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// xAI provider implementation
pub struct XaiProvider {
    #[allow(dead_code)]
    config: LlmProviderConfig,
    models: HashMap<String, Arc<dyn LlmModel>>,
}

impl XaiProvider {
    pub fn new(config: LlmProviderConfig) -> Result<Self> {
        let mut provider = Self {
            config,
            models: HashMap::new(),
        };
        provider.initialize_models();
        Ok(provider)
    }

    fn initialize_models(&mut self) {
        // Initialize xAI models
        let models: Vec<Arc<dyn LlmModel>> = vec![Arc::new(GrokCodeFast1Model::new())];

        for model in models {
            self.models.insert(model.id().to_string(), model);
        }
    }
}

#[async_trait]
impl LlmProvider for XaiProvider {
    fn id(&self) -> &str {
        &self.config.provider
    }

    fn name(&self) -> &str {
        "xAI"
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Custom
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn supports_tool_calling(&self) -> bool {
        true
    }

    fn supports_vision(&self) -> bool {
        false
    }

    async fn list_available_models(&self) -> Result<Vec<Arc<dyn LlmModel>>, anyhow::Error> {
        Ok(self.models.values().cloned().collect())
    }

    fn get_model(&self, model_id: &str) -> Option<Arc<dyn LlmModel>> {
        self.models.get(model_id).cloned()
    }

    async fn test_connection(&self) -> Result<(), anyhow::Error> {
        // Test connection by making a simple API call
        let client = reqwest::Client::new();
        let response = client
            .get("https://api.x.ai/v1/models")
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(ProviderError::Authentication("Invalid API key".to_string()).into())
        }
    }

    fn create_client(&self, model_id: &str) -> Result<Box<dyn LlmClient>, anyhow::Error> {
        let mut config = self.config.clone();
        config.model = model_id.to_string();
        create_llm_client(config)
    }
}

/// Grok Code Fast 1 model implementation
pub struct GrokCodeFast1Model {
    capabilities: ModelCapabilities,
    pricing: Option<ModelPricing>,
}

impl GrokCodeFast1Model {
    pub fn new() -> Self {
        Self {
            capabilities: ModelCapabilities {
                supports_streaming: true,
                supports_tool_calling: true,
                supports_vision: false,
                supports_reasoning: true,
                supports_json_mode: true,
            },
            pricing: Some(ModelPricing {
                input_cost_per_million_tokens: 2.0,
                output_cost_per_million_tokens: 10.0,
                reasoning_cost_per_million_tokens: Some(5.0),
            }),
        }
    }
}

impl Default for GrokCodeFast1Model {
    fn default() -> Self {
        Self::new()
    }
}

impl LlmModel for GrokCodeFast1Model {
    fn id(&self) -> &str {
        "grok-code-fast-1"
    }

    fn name(&self) -> &str {
        "Grok Code Fast 1"
    }

    fn provider_id(&self) -> &str {
        "xai"
    }

    fn context_length(&self) -> u32 {
        131072
    }

    fn max_output_tokens(&self) -> Option<u32> {
        Some(8192)
    }

    fn supports_streaming(&self) -> bool {
        self.capabilities.supports_streaming
    }

    fn supports_tool_calling(&self) -> bool {
        self.capabilities.supports_tool_calling
    }

    fn supports_vision(&self) -> bool {
        self.capabilities.supports_vision
    }

    fn supports_reasoning(&self) -> bool {
        self.capabilities.supports_reasoning
    }

    fn input_cost_per_token(&self) -> Option<f64> {
        self.pricing
            .as_ref()
            .map(|p| p.input_cost_per_million_tokens / 1_000_000.0)
    }

    fn output_cost_per_token(&self) -> Option<f64> {
        self.pricing
            .as_ref()
            .map(|p| p.output_cost_per_million_tokens / 1_000_000.0)
    }

    fn default_temperature(&self) -> Option<f32> {
        Some(0.7)
    }

    fn default_max_tokens(&self) -> Option<u32> {
        Some(2000)
    }

    fn estimate_tokens(&self, text: &str) -> u32 {
        text.len() as u32 / 4
    }
}
