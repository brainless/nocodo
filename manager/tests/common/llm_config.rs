use std::env;

/// Configuration for LLM provider testing
#[derive(Debug, Clone)]
pub struct LlmTestConfig {
    pub enabled_providers: Vec<LlmProviderTestConfig>,
    pub default_provider: Option<String>,
    pub test_timeouts: LlmTestTimeouts,
}

/// Configuration for a specific LLM provider
#[derive(Debug, Clone)]
pub struct LlmProviderTestConfig {
    pub name: String,           // "grok", "openai", "anthropic"
    pub models: Vec<String>,    // ["grok-code-fast-1", "gpt-4", "claude-3"]
    pub api_key_env: String,    // "GROK_API_KEY", "OPENAI_API_KEY"
    pub enabled: bool,          // Skip if API key not available
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
    pub request_timeout_secs: u64,
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
    pub fn from_environment() -> Self {
        let mut providers = Vec::new();

        // Auto-detect available API keys
        if env::var("GROK_API_KEY").is_ok() {
            providers.push(LlmProviderTestConfig::grok());
        }
        if env::var("OPENAI_API_KEY").is_ok() {
            providers.push(LlmProviderTestConfig::openai());
        }
        if env::var("ANTHROPIC_API_KEY").is_ok() {
            providers.push(LlmProviderTestConfig::anthropic());
        }

        let default_provider = providers.get(0).map(|p| p.name.clone());

        Self {
            enabled_providers: providers,
            default_provider,
            test_timeouts: LlmTestTimeouts::default(),
        }
    }

    /// Check if any LLM providers are available
    pub fn has_available_providers(&self) -> bool {
        !self.enabled_providers.is_empty()
    }

    /// Get the first available provider for simple tests
    pub fn get_default_provider(&self) -> Option<&LlmProviderTestConfig> {
        self.enabled_providers.get(0)
    }
}

impl LlmProviderTestConfig {
    /// Create Grok provider configuration
    pub fn grok() -> Self {
        Self {
            name: "grok".to_string(),
            models: vec!["grok-code-fast-1".to_string()],
            api_key_env: "GROK_API_KEY".to_string(),
            enabled: env::var("GROK_API_KEY").is_ok(),
            test_prompts: LlmTestPrompts::default(),
        }
    }

    /// Create OpenAI provider configuration
    pub fn openai() -> Self {
        Self {
            name: "openai".to_string(),
            models: vec!["gpt-4".to_string(), "gpt-4-turbo".to_string()],
            api_key_env: "OPENAI_API_KEY".to_string(),
            enabled: env::var("OPENAI_API_KEY").is_ok(),
            test_prompts: LlmTestPrompts::default(),
        }
    }

    /// Create Anthropic provider configuration
    pub fn anthropic() -> Self {
        Self {
            name: "anthropic".to_string(),
            models: vec!["claude-3-sonnet-20240229".to_string(), "claude-3-opus-20240229".to_string()],
            api_key_env: "ANTHROPIC_API_KEY".to_string(),
            enabled: env::var("ANTHROPIC_API_KEY").is_ok(),
            test_prompts: LlmTestPrompts::default(),
        }
    }

    /// Get the default model for this provider
    pub fn default_model(&self) -> &str {
        self.models.get(0).map(|s| s.as_str()).unwrap_or("unknown")
    }

    /// Convert to app config format
    pub fn to_app_config(&self) -> nocodo_manager::config::AppConfig {
        use nocodo_manager::config::{AppConfig, ApiKeysConfig, DatabaseConfig, ServerConfig, SocketConfig};
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
            api_keys: Some(ApiKeysConfig {
                grok_api_key: if self.name == "grok" { api_key.clone() } else { None },
                openai_api_key: if self.name == "openai" { api_key.clone() } else { None },
                anthropic_api_key: if self.name == "anthropic" { api_key.clone() } else { None },
            }),
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
        assert!(anthropic.models.contains(&"claude-3-sonnet-20240229".to_string()));
    }

    #[test]
    fn test_default_prompts() {
        let prompts = LlmTestPrompts::default();
        assert!(prompts.tech_stack_analysis.contains("tech stack"));
        assert!(prompts.code_generation.contains("function"));
        assert!(prompts.file_analysis.contains("project"));
    }
}