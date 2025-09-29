use crate::llm_client::{LlmProvider, LlmModel, LlmClient, ProviderType, ModelCapabilities, ModelPricing, ProviderError, create_llm_client};
use crate::models::LlmProviderConfig;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;

/// Anthropic provider implementation
pub struct AnthropicProvider {
    config: LlmProviderConfig,
    models: HashMap<String, Box<dyn LlmModel>>,
}

impl AnthropicProvider {
    pub fn new(config: LlmProviderConfig) -> Result<Self> {
        let mut provider = Self {
            config,
            models: HashMap::new(),
        };
        provider.initialize_models();
        Ok(provider)
    }

    fn initialize_models(&mut self) {
        // Initialize common Anthropic models
        let models: Vec<Box<dyn LlmModel>> = vec![
            Box::new(Claude3OpusModel::new()),
            Box::new(Claude3SonnetModel::new()),
            Box::new(Claude3HaikuModel::new()),
        ];

        for model in models {
            self.models.insert(model.id().to_string(), model);
        }
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    fn id(&self) -> &str {
        &self.config.provider
    }

    fn name(&self) -> &str {
        "Anthropic"
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Anthropic
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

    async fn list_available_models(&self) -> Result<Vec<Box<dyn LlmModel>>, anyhow::Error> {
        // For now, return empty list to avoid cloning issues
        // TODO: Implement proper model listing without cloning
        Ok(Vec::new())
    }

    fn get_model(&self, _model_id: &str) -> Option<Box<dyn LlmModel>> {
        // For now, return None to avoid cloning issues
        // TODO: Implement proper model retrieval without cloning
        None
    }

    async fn test_connection(&self) -> Result<(), anyhow::Error> {
        // Test connection by making a simple API call
        let client = reqwest::Client::new();
        let response = client
            .get("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
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

/// Claude 3 Opus model implementation
pub struct Claude3OpusModel {
    capabilities: ModelCapabilities,
    pricing: Option<ModelPricing>,
}

impl Claude3OpusModel {
    pub fn new() -> Self {
        Self {
            capabilities: ModelCapabilities {
                supports_streaming: true,
                supports_tool_calling: true,
                supports_vision: true,
                supports_reasoning: false,
                supports_json_mode: true,
            },
            pricing: Some(ModelPricing {
                input_cost_per_million_tokens: 15.0,
                output_cost_per_million_tokens: 75.0,
                reasoning_cost_per_million_tokens: None,
            }),
        }
    }
}

impl LlmModel for Claude3OpusModel {
    fn id(&self) -> &str {
        "claude-3-opus-20240229"
    }

    fn name(&self) -> &str {
        "Claude 3 Opus"
    }

    fn provider_id(&self) -> &str {
        "anthropic"
    }

    fn context_length(&self) -> u32 {
        200000
    }

    fn max_output_tokens(&self) -> Option<u32> {
        Some(4096)
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
        Some(1000)
    }

    fn estimate_tokens(&self, text: &str) -> u32 {
        // Simple token estimation (rough approximation)
        text.len() as u32 / 4
    }
}

/// Claude 3 Sonnet model implementation
pub struct Claude3SonnetModel {
    capabilities: ModelCapabilities,
    pricing: Option<ModelPricing>,
}

impl Claude3SonnetModel {
    pub fn new() -> Self {
        Self {
            capabilities: ModelCapabilities {
                supports_streaming: true,
                supports_tool_calling: true,
                supports_vision: true,
                supports_reasoning: false,
                supports_json_mode: true,
            },
            pricing: Some(ModelPricing {
                input_cost_per_million_tokens: 3.0,
                output_cost_per_million_tokens: 15.0,
                reasoning_cost_per_million_tokens: None,
            }),
        }
    }
}

impl LlmModel for Claude3SonnetModel {
    fn id(&self) -> &str {
        "claude-3-sonnet-20240229"
    }

    fn name(&self) -> &str {
        "Claude 3 Sonnet"
    }

    fn provider_id(&self) -> &str {
        "anthropic"
    }

    fn context_length(&self) -> u32 {
        200000
    }

    fn max_output_tokens(&self) -> Option<u32> {
        Some(4096)
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
        Some(1000)
    }

    fn estimate_tokens(&self, text: &str) -> u32 {
        text.len() as u32 / 4
    }
}

/// Claude 3 Haiku model implementation
pub struct Claude3HaikuModel {
    capabilities: ModelCapabilities,
    pricing: Option<ModelPricing>,
}

impl Claude3HaikuModel {
    pub fn new() -> Self {
        Self {
            capabilities: ModelCapabilities {
                supports_streaming: true,
                supports_tool_calling: true,
                supports_vision: true,
                supports_reasoning: false,
                supports_json_mode: true,
            },
            pricing: Some(ModelPricing {
                input_cost_per_million_tokens: 0.25,
                output_cost_per_million_tokens: 1.25,
                reasoning_cost_per_million_tokens: None,
            }),
        }
    }
}

impl LlmModel for Claude3HaikuModel {
    fn id(&self) -> &str {
        "claude-3-haiku-20240307"
    }

    fn name(&self) -> &str {
        "Claude 3 Haiku"
    }

    fn provider_id(&self) -> &str {
        "anthropic"
    }

    fn context_length(&self) -> u32 {
        200000
    }

    fn max_output_tokens(&self) -> Option<u32> {
        Some(4096)
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
        Some(1000)
    }

    fn estimate_tokens(&self, text: &str) -> u32 {
        text.len() as u32 / 4
    }
}