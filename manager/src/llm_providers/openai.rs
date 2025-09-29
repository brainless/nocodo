use crate::llm_client::{LlmProvider, LlmModel, LlmClient, ProviderType, ModelCapabilities, ModelPricing, ProviderError, create_llm_client};
use crate::models::LlmProviderConfig;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// OpenAI provider implementation
pub struct OpenAiProvider {
    #[allow(dead_code)]
    config: LlmProviderConfig,
    models: HashMap<String, Arc<dyn LlmModel>>,
}

impl OpenAiProvider {
    pub fn new(config: LlmProviderConfig) -> Result<Self> {
        let mut provider = Self {
            config,
            models: HashMap::new(),
        };
        provider.initialize_models();
        Ok(provider)
    }

    fn initialize_models(&mut self) {
        // Initialize OpenAI models
        let models: Vec<Arc<dyn LlmModel>> = vec![
            Arc::new(Gpt5Model::new()),
            Arc::new(Gpt5CodexModel::new()),
        ];

        for model in models {
            self.models.insert(model.id().to_string(), model);
        }
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    fn id(&self) -> &str {
        &self.config.provider
    }

    fn name(&self) -> &str {
        "OpenAI"
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::OpenAI
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn supports_tool_calling(&self) -> bool {
        true
    }

    fn supports_vision(&self) -> bool {
        true
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
            .get("https://api.openai.com/v1/models")
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

/// GPT-5 model implementation
pub struct Gpt5Model {
    capabilities: ModelCapabilities,
    pricing: Option<ModelPricing>,
}

impl Gpt5Model {
    pub fn new() -> Self {
        Self {
            capabilities: ModelCapabilities {
                supports_streaming: true,
                supports_tool_calling: true,
                supports_vision: true,
                supports_reasoning: true,
                supports_json_mode: true,
            },
            pricing: Some(ModelPricing {
                input_cost_per_million_tokens: 2.0,
                output_cost_per_million_tokens: 8.0,
                reasoning_cost_per_million_tokens: Some(4.0),
            }),
        }
    }
}

impl LlmModel for Gpt5Model {
    fn id(&self) -> &str {
        "gpt-5"
    }

    fn name(&self) -> &str {
        "GPT-5"
    }

    fn provider_id(&self) -> &str {
        "openai"
    }

    fn context_length(&self) -> u32 {
        262144
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
        self.pricing.as_ref().map(|p| p.input_cost_per_million_tokens / 1_000_000.0)
    }

    fn output_cost_per_token(&self) -> Option<f64> {
        self.pricing.as_ref().map(|p| p.output_cost_per_million_tokens / 1_000_000.0)
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

/// GPT-5 Codex model implementation
pub struct Gpt5CodexModel {
    capabilities: ModelCapabilities,
    pricing: Option<ModelPricing>,
}

impl Gpt5CodexModel {
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
                input_cost_per_million_tokens: 1.5,
                output_cost_per_million_tokens: 6.0,
                reasoning_cost_per_million_tokens: Some(3.0),
            }),
        }
    }
}

impl LlmModel for Gpt5CodexModel {
    fn id(&self) -> &str {
        "gpt-5-codex"
    }

    fn name(&self) -> &str {
        "GPT-5 Codex"
    }

    fn provider_id(&self) -> &str {
        "openai"
    }

    fn context_length(&self) -> u32 {
        262144
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
        self.pricing.as_ref().map(|p| p.input_cost_per_million_tokens / 1_000_000.0)
    }

    fn output_cost_per_token(&self) -> Option<f64> {
        self.pricing.as_ref().map(|p| p.output_cost_per_million_tokens / 1_000_000.0)
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