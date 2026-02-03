use config::{Config, ConfigError, File};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApiConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub api_keys: Option<ApiKeysConfig>,
    pub llm: Option<LlmConfig>,
    pub cors: Option<CorsConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LlmConfig {
    pub provider: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApiKeysConfig {
    pub xai_api_key: Option<String>,
    pub openai_api_key: Option<String>,
    pub anthropic_api_key: Option<String>,
    pub cerebras_api_key: Option<String>,
    pub zai_api_key: Option<String>,
    pub zai_coding_plan: Option<bool>,
    pub zen_api_key: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DatabaseConfig {
    pub path: PathBuf,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
            },
            database: DatabaseConfig {
                path: get_default_db_path(),
            },
            api_keys: None,
            llm: None,
            cors: Some(CorsConfig {
                allowed_origins: vec!["http://localhost:3000".to_string()],
            }),
        }
    }
}

impl ApiConfig {
    pub fn load() -> Result<(Self, PathBuf), ConfigError> {
        let config_path = get_config_path();

        // Create config directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                ConfigError::Message(format!("Failed to create config directory: {e}"))
            })?;
        }

        // Create default config file if it doesn't exist
        if !config_path.exists() {
            let default_db_path = get_default_db_path();
            let default_config = format!(
                r#"
[server]
host = "127.0.0.1"
port = 8080

[database]
path = "{}"

[cors]
allowed_origins = ["http://localhost:3000"]

[llm]
# provider = "anthropic"  # Options: anthropic, openai, xai, zai

[api_keys]
# xai_api_key = "your-xai-key"
# openai_api_key = "your-openai-key"
# anthropic_api_key = "your-anthropic-key"
# cerebras_api_key = "your-cerebras-key"
# zai_api_key = "your-zai-key"
# zai_coding_plan = true
# zen_api_key = "your-zen-key"
"#,
                default_db_path.display()
            );
            std::fs::write(&config_path, default_config).map_err(|e| {
                ConfigError::Message(format!("Failed to write default config: {e}"))
            })?;
        }

        let builder = Config::builder()
            .add_source(File::from(config_path.clone()))
            .build()?;

        let mut config: ApiConfig = builder.try_deserialize()?;

        // Expand tilde in database path
        if config.database.path.starts_with("~") {
            if let Some(home) = home::home_dir() {
                let path_str = config.database.path.to_string_lossy();
                let expanded = path_str.replacen("~", &home.to_string_lossy(), 1);
                config.database.path = PathBuf::from(expanded);
            }
        }

        Ok((config, config_path))
    }
}

fn get_config_path() -> PathBuf {
    if let Some(config_dir) = dirs::config_dir() {
        config_dir.join("nocodo/api.toml")
    } else {
        PathBuf::from("api.toml")
    }
}

fn get_default_db_path() -> PathBuf {
    if let Some(data_dir) = dirs::data_local_dir() {
        data_dir.join("nocodo/api.db")
    } else {
        PathBuf::from("api.db")
    }
}
