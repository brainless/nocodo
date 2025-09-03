use config::{Config as ConfigBuilder, ConfigError, Environment, File};
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub file: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub logging: LoggingConfig,
}

impl Config {
    pub fn load() -> Result<Self, ConfigError> {
        let settings = ConfigBuilder::builder()
            // Set defaults
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 8080)?
            .set_default("database.url", "postgresql://localhost/nocodo_services")?
            .set_default("database.max_connections", 10)?
            .set_default("logging.level", "info")?;

        let settings = {
            // Try to load from services.toml in current directory
            let settings = if let Ok(current_dir) = env::current_dir() {
                let config_path = current_dir.join("services.toml");
                if config_path.exists() {
                    settings.add_source(File::from(config_path))
                } else {
                    settings
                }
            } else {
                settings
            };

            // Try to load from ~/.config/nocodo/services.toml
            let settings = if let Ok(home_dir) = env::var("HOME") {
                let config_path = format!("{}/.config/nocodo/services.toml", home_dir);
                settings.add_source(File::with_name(&config_path).required(false))
            } else {
                settings
            };

            // Override with environment variables (with prefix NOCODO_SERVICES_)
            settings.add_source(Environment::with_prefix("NOCODO_SERVICES"))
        };

        settings.build()?.try_deserialize()
    }
}