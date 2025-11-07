use std::env;

/// Configuration for LLM provider testing
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct LlmTestConfig {
    pub enabled_providers: Vec<LlmProviderTestConfig>,
    pub default_provider: Option<String>,
    pub test_timeouts: LlmTestTimeouts,
}

/// Configuration for a specific LLM provider
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct LlmProviderTestConfig {
    pub name: String,        // "grok", "openai", "anthropic"
    pub models: Vec<String>, // ["grok-code-fast-1", "gpt-4", "claude-3"]
    pub api_key_env: String, // "GROK_API_KEY", "OPENAI_API_KEY"
    pub enabled: bool,       // Skip if API key not available
    pub test_prompts: LlmTestPrompts,
}

/// Test prompts for different LLM scenarios
#[derive(Debug, Clone)]
pub struct LlmTestPrompts {
    pub tech_stack_analysis: String,
    pub code_generation: String,
    pub file_analysis: String,
}

/// Timeout configurations for LLM tests
#[derive(Debug, Clone)]
pub struct LlmTestTimeouts {
    #[allow(dead_code)]
    pub request_timeout_secs: u64,
    #[allow(dead_code)]
    pub total_test_timeout_secs: u64,
}

impl Default for LlmTestTimeouts {
    fn default() -> Self {
        Self {
            request_timeout_secs: 30,
            total_test_timeout_secs: 120,
        }
    }
}

impl LlmTestConfig {
    /// Create configuration from environment variables
    #[allow(dead_code)]
    pub fn from_environment() -> Self {
        let mut providers = Vec::new();

        // Check for specific provider and model from environment (e.g., set by script)
        let forced_provider = env::var("PROVIDER").ok();
        let forced_model = env::var("MODEL").ok();

        // Auto-detect available API keys and validate against actual providers
        if env::var("GROK_API_KEY").is_ok() || env::var("XAI_API_KEY").is_ok() {
            let name = forced_provider.as_deref().unwrap_or("xai");
            if let Some(provider_config) =
                LlmProviderTestConfig::xai_with_validation(name, forced_model.as_deref())
            {
                providers.push(provider_config);
            }
        }
        if env::var("OPENAI_API_KEY").is_ok() {
            if let Some(provider_config) =
                LlmProviderTestConfig::openai_with_validation(forced_model.as_deref())
            {
                providers.push(provider_config);
            }
        }
        if env::var("ANTHROPIC_API_KEY").is_ok() {
            if let Some(provider_config) =
                LlmProviderTestConfig::anthropic_with_validation(forced_model.as_deref())
            {
                providers.push(provider_config);
            }
        }
        if env::var("ZAI_API_KEY").is_ok() {
            if let Some(provider_config) =
                LlmProviderTestConfig::zai_with_validation(forced_model.as_deref())
            {
                providers.push(provider_config);
            }
        }

        let default_provider = providers.first().map(|p| p.name.clone());

        Self {
            enabled_providers: providers,
            default_provider,
            test_timeouts: LlmTestTimeouts::default(),
        }
    }

    /// Check if any LLM providers are available
    #[allow(dead_code)]
    pub fn has_available_providers(&self) -> bool {
        !self.enabled_providers.is_empty()
    }

    /// Get the first available provider for simple tests
    #[allow(dead_code)]
    pub fn get_default_provider(&self) -> Option<&LlmProviderTestConfig> {
        self.enabled_providers.first()
    }
}

impl LlmProviderTestConfig {
    /// Create xAI provider configuration with validation
    pub fn xai_with_validation(name: &str, requested_model: Option<&str>) -> Option<Self> {
        // Check if API key is available
        if env::var("GROK_API_KEY").is_err() && env::var("XAI_API_KEY").is_err() {
            return None;
        }

        // Get available models from actual provider implementation
        let available_models = Self::get_available_xai_models();

        // If a specific model was requested, validate it exists
        if let Some(model) = requested_model {
            if !available_models.contains(&model.to_string()) {
                eprintln!("❌ Error: Model '{}' not available for xAI provider", model);
                eprintln!("   Available models: {:?}", available_models);
                return None;
            }
        }

        Some(Self {
            name: name.to_string(),
            models: available_models,
            api_key_env: "GROK_API_KEY".to_string(),
            enabled: true,
            test_prompts: LlmTestPrompts::default(),
        })
    }

    /// Create xAI provider configuration (legacy method)
    pub fn xai(name: &str) -> Self {
        Self::xai_with_validation(name, None).unwrap_or_else(|| Self {
            name: name.to_string(),
            models: vec!["grok-code-fast-1".to_string()],
            api_key_env: "GROK_API_KEY".to_string(),
            enabled: false,
            test_prompts: LlmTestPrompts::default(),
        })
    }

    /// Create Grok provider configuration (deprecated, use xai)
    pub fn grok() -> Self {
        Self::xai("grok")
    }

    /// Create OpenAI provider configuration with validation
    pub fn openai_with_validation(requested_model: Option<&str>) -> Option<Self> {
        // Check if API key is available
        if env::var("OPENAI_API_KEY").is_err() {
            return None;
        }

        // Get available models from actual provider implementation
        let available_models = Self::get_available_openai_models();

        // If a specific model was requested, validate it exists
        if let Some(model) = requested_model {
            if !available_models.contains(&model.to_string()) {
                eprintln!(
                    "❌ Error: Model '{}' not available for OpenAI provider",
                    model
                );
                eprintln!("   Available models: {:?}", available_models);
                return None;
            }
        }

        Some(Self {
            name: "openai".to_string(),
            models: available_models,
            api_key_env: "OPENAI_API_KEY".to_string(),
            enabled: true,
            test_prompts: LlmTestPrompts::default(),
        })
    }

    /// Create OpenAI provider configuration (legacy method)
    pub fn openai() -> Self {
        Self::openai_with_validation(None).unwrap_or_else(|| Self {
            name: "openai".to_string(),
            models: vec!["gpt-4".to_string(), "gpt-4-turbo".to_string()],
            api_key_env: "OPENAI_API_KEY".to_string(),
            enabled: false,
            test_prompts: LlmTestPrompts::default(),
        })
    }

    /// Create Anthropic provider configuration with validation
    pub fn anthropic_with_validation(requested_model: Option<&str>) -> Option<Self> {
        // Check if API key is available
        if env::var("ANTHROPIC_API_KEY").is_err() {
            return None;
        }

        // Get available models from actual provider implementation
        let available_models = Self::get_available_anthropic_models();

        // If a specific model was requested, validate it exists
        if let Some(model) = requested_model {
            if !available_models.contains(&model.to_string()) {
                eprintln!(
                    "❌ Error: Model '{}' not available for Anthropic provider",
                    model
                );
                eprintln!("   Available models: {:?}", available_models);
                return None;
            }
        }

        Some(Self {
            name: "anthropic".to_string(),
            models: available_models,
            api_key_env: "ANTHROPIC_API_KEY".to_string(),
            enabled: true,
            test_prompts: LlmTestPrompts::default(),
        })
    }

    /// Create Anthropic provider configuration (legacy method)
    pub fn anthropic() -> Self {
        Self::anthropic_with_validation(None).unwrap_or_else(|| Self {
            name: "anthropic".to_string(),
            models: vec![
                "claude-sonnet-4-5-20250929".to_string(),
                "claude-haiku-4-5-20251001".to_string(),
                "claude-opus-4-1-20250805".to_string(),
            ],
            api_key_env: "ANTHROPIC_API_KEY".to_string(),
            enabled: false,
            test_prompts: LlmTestPrompts::default(),
        })
    }

    /// Create zAI provider configuration with validation
    pub fn zai_with_validation(requested_model: Option<&str>) -> Option<Self> {
        // Check if API key is available
        if env::var("ZAI_API_KEY").is_err() {
            return None;
        }

        // Get available models from actual provider implementation
        let available_models = Self::get_available_zai_models();

        // If a specific model was requested, validate it exists
        if let Some(model) = requested_model {
            if !available_models.contains(&model.to_string()) {
                eprintln!("❌ Error: Model '{}' not available for zAI provider", model);
                eprintln!("   Available models: {:?}", available_models);
                return None;
            }
        }

        Some(Self {
            name: "zai".to_string(),
            models: available_models,
            api_key_env: "ZAI_API_KEY".to_string(),
            enabled: true,
            test_prompts: LlmTestPrompts::default(),
        })
    }

    /// Get available zAI models from the actual provider implementation
    fn get_available_zai_models() -> Vec<String> {
        // This would ideally query the actual provider, but for now we'll hardcode
        // the models that are actually implemented in the provider
        vec!["glm-4.6".to_string()]
    }

    /// Create zAI provider configuration (legacy method)
    pub fn zai() -> Self {
        Self::zai_with_validation(None).unwrap_or_else(|| Self {
            name: "zai".to_string(),
            models: vec!["glm-4.6".to_string()],
            api_key_env: "ZAI_API_KEY".to_string(),
            enabled: false,
            test_prompts: LlmTestPrompts::default(),
        })
    }

    /// Get available xAI models from the actual provider implementation
    fn get_available_xai_models() -> Vec<String> {
        // This would ideally query the actual provider, but for now we'll hardcode
        // the models that are actually implemented in the provider
        vec!["grok-code-fast-1".to_string()]
    }

    /// Get available OpenAI models from the actual provider implementation
    fn get_available_openai_models() -> Vec<String> {
        // This would ideally query the actual provider, but for now we'll hardcode
        // the models that are actually implemented in the provider
        vec!["gpt-5".to_string(), "gpt-5-codex".to_string()]
    }

    /// Get available Anthropic models from the actual provider implementation
    fn get_available_anthropic_models() -> Vec<String> {
        // Claude 4.5/4.1 models - current generation (ONLY these are supported)
        vec![
            // Full model IDs
            "claude-sonnet-4-5-20250929".to_string(),
            "claude-haiku-4-5-20251001".to_string(),
            "claude-opus-4-1-20250805".to_string(),
            // Aliases (for convenience)
            "claude-sonnet-4-5".to_string(),
            "claude-haiku-4-5".to_string(),
            "claude-opus-4-1".to_string(),
        ]
    }

    /// Get the default model for this provider
    #[allow(dead_code)]
    pub fn default_model(&self) -> &str {
        self.models.first().map(|s| s.as_str()).unwrap_or("unknown")
    }

    /// Convert to app config format
    pub fn to_app_config(&self) -> nocodo_manager::config::AppConfig {
        use nocodo_manager::config::{
            ApiKeysConfig, AppConfig, DatabaseConfig, ServerConfig, SocketConfig,
        };
        use std::path::PathBuf;

        // Get API key from environment
        let api_key = env::var(&self.api_key_env).ok();

        AppConfig {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 0,
            },
            database: DatabaseConfig {
                path: PathBuf::from("/tmp/test.db"),
            },
            socket: SocketConfig {
                path: "/tmp/test.sock".to_string(),
            },
            auth: Some(nocodo_manager::config::AuthConfig {
                jwt_secret: Some("test-jwt-secret-for-llm-tests".to_string()),
            }),
            api_keys: Some(ApiKeysConfig {
                xai_api_key: if self.name == "xai" {
                    api_key.clone()
                } else {
                    None
                },
                openai_api_key: if self.name == "openai" {
                    api_key.clone()
                } else {
                    None
                },
                anthropic_api_key: if self.name == "anthropic" {
                    api_key.clone()
                } else {
                    None
                },
                zai_api_key: if self.name == "zai" {
                    api_key.clone()
                } else {
                    None
                },
            }),
            projects: None,
        }
    }
}

impl Default for LlmTestPrompts {
    fn default() -> Self {
        Self {
            tech_stack_analysis: "Analyze the tech stack of this project. What technologies and frameworks are being used?".to_string(),
            code_generation: "Write a simple function in the main language of this project that calculates factorial.".to_string(),
            file_analysis: "Examine the project files and provide a summary of the project structure and purpose.".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_configurations() {
        let grok = LlmProviderTestConfig::grok();
        assert_eq!(grok.name, "grok");
        assert_eq!(grok.api_key_env, "GROK_API_KEY");
        assert!(grok.models.contains(&"grok-code-fast-1".to_string()));

        let openai = LlmProviderTestConfig::openai();
        assert_eq!(openai.name, "openai");
        assert_eq!(openai.api_key_env, "OPENAI_API_KEY");
        assert!(openai.models.contains(&"gpt-4".to_string()));

        let anthropic = LlmProviderTestConfig::anthropic();
        assert_eq!(anthropic.name, "anthropic");
        assert_eq!(anthropic.api_key_env, "ANTHROPIC_API_KEY");
        assert!(anthropic
            .models
            .contains(&"claude-sonnet-4-5-20250929".to_string()));
    }

    #[test]
    fn test_default_prompts() {
        let prompts = LlmTestPrompts::default();
        assert!(prompts.tech_stack_analysis.contains("tech stack"));
        assert!(prompts.code_generation.contains("function"));
        assert!(prompts.file_analysis.contains("project"));
    }
}
