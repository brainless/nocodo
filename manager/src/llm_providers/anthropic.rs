use crate::llm_client::{
    create_llm_client, LlmClient, LlmModel, LlmProvider, ModelCapabilities, ModelPricing,
    ProviderError, ProviderType,
};
use crate::models::LlmProviderConfig;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

// Model ID constants
pub const CLAUDE_OPUS_4_1_MODEL_ID: &str = "claude-opus-4-1-20250805";
pub const CLAUDE_SONNET_4_5_MODEL_ID: &str = "claude-sonnet-4-5-20250929";
pub const CLAUDE_HAIKU_4_5_MODEL_ID: &str = "claude-haiku-4-5-20251001";

/// Anthropic provider implementation
pub struct AnthropicProvider {
    #[allow(dead_code)]
    config: LlmProviderConfig,
    models: HashMap<String, Arc<dyn LlmModel>>,
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
        // Initialize latest Claude models
        let models: Vec<Arc<dyn LlmModel>> = vec![
            Arc::new(ClaudeOpus41Model::new()),
            Arc::new(ClaudeSonnet45Model::new()),
            Arc::new(ClaudeHaiku45Model::new()),
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

/// Claude Opus 4.1 model implementation
pub struct ClaudeOpus41Model {
    capabilities: ModelCapabilities,
    pricing: Option<ModelPricing>,
}

impl ClaudeOpus41Model {
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
                input_cost_per_million_tokens: 15.0,
                output_cost_per_million_tokens: 75.0,
                reasoning_cost_per_million_tokens: None,
            }),
        }
    }
}

impl Default for ClaudeOpus41Model {
    fn default() -> Self {
        Self::new()
    }
}

impl LlmModel for ClaudeOpus41Model {
    fn id(&self) -> &str {
        CLAUDE_OPUS_4_1_MODEL_ID
    }

    fn name(&self) -> &str {
        "Claude Opus 4.1"
    }

    fn provider_id(&self) -> &str {
        "anthropic"
    }

    fn context_length(&self) -> u32 {
        200000 // 200K tokens
    }

    fn max_output_tokens(&self) -> Option<u32> {
        Some(32000) // 32K max output tokens
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
        Some(4000)
    }

    fn estimate_tokens(&self, text: &str) -> u32 {
        text.len() as u32 / 4
    }
}

/// Claude Sonnet 4.5 model implementation
pub struct ClaudeSonnet45Model {
    capabilities: ModelCapabilities,
    pricing: Option<ModelPricing>,
}

impl ClaudeSonnet45Model {
    pub fn new() -> Self {
        Self {
            capabilities: ModelCapabilities {
                supports_streaming: true,
                supports_tool_calling: true,
                supports_vision: true,
                supports_reasoning: true, // Claude Sonnet 4.5 supports extended thinking
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

impl Default for ClaudeSonnet45Model {
    fn default() -> Self {
        Self::new()
    }
}

impl LlmModel for ClaudeSonnet45Model {
    fn id(&self) -> &str {
        CLAUDE_SONNET_4_5_MODEL_ID
    }

    fn name(&self) -> &str {
        "Claude Sonnet 4.5"
    }

    fn provider_id(&self) -> &str {
        "anthropic"
    }

    fn context_length(&self) -> u32 {
        200000 // 200K tokens, with 1M beta available
    }

    fn max_output_tokens(&self) -> Option<u32> {
        Some(64000) // 64K max output tokens
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
        Some(4000)
    }

    fn estimate_tokens(&self, text: &str) -> u32 {
        text.len() as u32 / 4
    }
}

/// Claude Haiku 4.5 model implementation
pub struct ClaudeHaiku45Model {
    capabilities: ModelCapabilities,
    pricing: Option<ModelPricing>,
}

impl ClaudeHaiku45Model {
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
                input_cost_per_million_tokens: 1.0,
                output_cost_per_million_tokens: 5.0,
                reasoning_cost_per_million_tokens: None,
            }),
        }
    }
}

impl Default for ClaudeHaiku45Model {
    fn default() -> Self {
        Self::new()
    }
}

impl LlmModel for ClaudeHaiku45Model {
    fn id(&self) -> &str {
        CLAUDE_HAIKU_4_5_MODEL_ID
    }

    fn name(&self) -> &str {
        "Claude Haiku 4.5"
    }

    fn provider_id(&self) -> &str {
        "anthropic"
    }

    fn context_length(&self) -> u32 {
        200000 // 200K tokens
    }

    fn max_output_tokens(&self) -> Option<u32> {
        Some(64000) // 64K max output tokens
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
        Some(4000)
    }

    fn estimate_tokens(&self, text: &str) -> u32 {
        text.len() as u32 / 4
    }
}
