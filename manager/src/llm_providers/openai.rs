use crate::llm_client::{LlmProvider, LlmModel, LlmClient, ProviderType, ModelCapabilities, ModelPricing, ProviderError, create_llm_client};
use crate::models::LlmProviderConfig;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;

/// OpenAI provider implementation
pub struct OpenAiProvider {
    config: LlmProviderConfig,
    models: HashMap<String, Box<dyn LlmModel>>,
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
        // Initialize common OpenAI models
        let models: Vec<Box<dyn LlmModel>> = vec![
            Box::new(Gpt4Model::new()),
            Box::new(Gpt4TurboModel::new()),
            Box::new(Gpt35TurboModel::new()),
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

/// GPT-4 model implementation
pub struct Gpt4Model {
    capabilities: ModelCapabilities,
    pricing: Option<ModelPricing>,
}

impl Gpt4Model {
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
                input_cost_per_million_tokens: 30.0,
                output_cost_per_million_tokens: 60.0,
                reasoning_cost_per_million_tokens: None,
            }),
        }
    }
}

impl LlmModel for Gpt4Model {
    fn id(&self) -> &str {
        "gpt-4"
    }

    fn name(&self) -> &str {
        "GPT-4"
    }

    fn provider_id(&self) -> &str {
        "openai"
    }

    fn context_length(&self) -> u32 {
        8192
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

/// GPT-4 Turbo model implementation
pub struct Gpt4TurboModel {
    capabilities: ModelCapabilities,
    pricing: Option<ModelPricing>,
}

impl Gpt4TurboModel {
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
                input_cost_per_million_tokens: 10.0,
                output_cost_per_million_tokens: 30.0,
                reasoning_cost_per_million_tokens: None,
            }),
        }
    }
}

impl LlmModel for Gpt4TurboModel {
    fn id(&self) -> &str {
        "gpt-4-turbo"
    }

    fn name(&self) -> &str {
        "GPT-4 Turbo"
    }

    fn provider_id(&self) -> &str {
        "openai"
    }

    fn context_length(&self) -> u32 {
        128000
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

/// GPT-3.5 Turbo model implementation
pub struct Gpt35TurboModel {
    capabilities: ModelCapabilities,
    pricing: Option<ModelPricing>,
}

impl Gpt35TurboModel {
    pub fn new() -> Self {
        Self {
            capabilities: ModelCapabilities {
                supports_streaming: true,
                supports_tool_calling: true,
                supports_vision: false,
                supports_reasoning: false,
                supports_json_mode: true,
            },
            pricing: Some(ModelPricing {
                input_cost_per_million_tokens: 1.5,
                output_cost_per_million_tokens: 2.0,
                reasoning_cost_per_million_tokens: None,
            }),
        }
    }
}

impl LlmModel for Gpt35TurboModel {
    fn id(&self) -> &str {
        "gpt-3.5-turbo"
    }

    fn name(&self) -> &str {
        "GPT-3.5 Turbo"
    }

    fn provider_id(&self) -> &str {
        "openai"
    }

    fn context_length(&self) -> u32 {
        16385
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