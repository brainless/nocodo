use config::{Config, ConfigError, File};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub socket: SocketConfig,
    pub api_keys: Option<ApiKeysConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ApiKeysConfig {
    pub xai_api_key: Option<String>,
    pub openai_api_key: Option<String>,
    pub anthropic_api_key: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub path: PathBuf,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SocketConfig {
    pub path: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8081,
            },
            database: DatabaseConfig {
                path: get_default_db_path(),
            },
            socket: SocketConfig {
                path: "/tmp/nocodo-manager.sock".to_string(),
            },
            api_keys: None,
        }
    }
}

impl AppConfig {
    pub fn load() -> Result<Self, ConfigError> {
        let config_path = get_config_path();

        // Create config directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                ConfigError::Message(format!("Failed to create config directory: {e}"))
            })?;
        }

        // Create default config file if it doesn't exist
        if !config_path.exists() {
            let default_config = r#"
[server]
host = "127.0.0.1"
port = 8081

[database]
path = "~/.local/share/nocodo/manager.db"

[socket]
path = "/tmp/nocodo-manager.sock"

[api_keys]
# xai_api_key = "your-xai-key"
# openai_api_key = "your-openai-key"
# anthropic_api_key = "your-anthropic-key"
"#;
            std::fs::write(&config_path, default_config).map_err(|e| {
                ConfigError::Message(format!("Failed to write default config: {e}"))
            })?;
        }

        let builder = Config::builder()
            .add_source(File::from(config_path))
            .build()?;

        let mut config: AppConfig = builder.try_deserialize()?;

        // Expand tilde in database path
        if config.database.path.starts_with("~") {
            if let Some(home) = home::home_dir() {
                let path_str = config.database.path.to_string_lossy();
                let expanded = path_str.replacen("~", &home.to_string_lossy(), 1);
                config.database.path = PathBuf::from(expanded);
            }
        }

        Ok(config)
    }

    pub fn load_from_file(config_path: &Path) -> Result<Self, ConfigError> {
        if !config_path.exists() {
            return Err(ConfigError::Message(format!(
                "Configuration file not found: {}",
                config_path.display()
            )));
        }

        let builder = Config::builder()
            .add_source(File::from(config_path.to_path_buf()))
            .build()?;

        let mut config: AppConfig = builder.try_deserialize()?;

        // Expand tilde in database path
        if config.database.path.starts_with("~") {
            if let Some(home) = home::home_dir() {
                let path_str = config.database.path.to_string_lossy();
                let expanded = path_str.replacen("~", &home.to_string_lossy(), 1);
                config.database.path = PathBuf::from(expanded);
            }
        }

        Ok(config)
    }
}

fn get_config_path() -> PathBuf {
    if let Some(home) = home::home_dir() {
        home.join(".config/nocodo/manager.toml")
    } else {
        PathBuf::from("manager.toml")
    }
}

fn get_default_db_path() -> PathBuf {
    if let Some(home) = home::home_dir() {
        home.join(".local/share/nocodo/manager.db")
    } else {
        PathBuf::from("manager.db")
    }
}
