use config::{Config, ConfigError, File};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use rand::Rng;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AuthConfig {
    pub jwt_secret: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub socket: SocketConfig,
    pub auth: Option<AuthConfig>,
    pub api_keys: Option<ApiKeysConfig>,
    pub projects: Option<ProjectsConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ApiKeysConfig {
    pub xai_api_key: Option<String>,
    pub openai_api_key: Option<String>,
    pub anthropic_api_key: Option<String>,
    pub zai_api_key: Option<String>,   // NEW: zAI API key
    pub zai_coding_plan: Option<bool>, // NEW: Use zAI Coding Plan endpoint
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ProjectsConfig {
    pub default_path: Option<String>,
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

#[derive(Debug, Deserialize, Serialize, Clone)]
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
            auth: None,
            api_keys: None,
            projects: None,
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

[auth]
# JWT secret for authentication tokens (IMPORTANT: Change this in production!)
# jwt_secret = "change-this-to-a-secure-random-string-in-production"

[api_keys]
# xai_api_key = "your-xai-key"
# openai_api_key = "your-openai-key"
# anthropic_api_key = "your-anthropic-key"
# zai_api_key = "your-zai-key"
# zai_coding_plan = true  # Set to true if using GLM Coding Plan subscription

[projects]
# default_path = "~/projects"
"#;
            std::fs::write(&config_path, default_config).map_err(|e| {
                ConfigError::Message(format!("Failed to write default config: {e}"))
            })?;
        }

        let builder = Config::builder()
            .add_source(File::from(config_path.clone()))
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

        // Check if JWT secret is missing and generate one if needed
        let jwt_secret_missing = config
            .auth
            .as_ref()
            .and_then(|a| a.jwt_secret.as_ref())
            .is_none();

        if jwt_secret_missing {
            let new_secret = generate_jwt_secret();
            tracing::info!("Generated new JWT secret for authentication");

            // Initialize auth config if it doesn't exist
            if config.auth.is_none() {
                config.auth = Some(AuthConfig {
                    jwt_secret: Some(new_secret.clone()),
                });
            } else if let Some(ref mut auth) = config.auth {
                auth.jwt_secret = Some(new_secret.clone());
            }

            // Update the config file with the new JWT secret
            if let Err(e) = update_config_file_with_jwt_secret(&config_path, &new_secret) {
                tracing::warn!("Failed to save JWT secret to config file: {e}");
                tracing::warn!("The JWT secret will be regenerated on next restart");
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

/// Generates a cryptographically secure random JWT secret
/// Equivalent to `openssl rand -base64 48`
fn generate_jwt_secret() -> String {
    let mut rng = rand::rng();
    let random_bytes: Vec<u8> = (0..48).map(|_| rng.random()).collect();
    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &random_bytes)
}

/// Updates the config file with a newly generated JWT secret
fn update_config_file_with_jwt_secret(
    config_path: &Path,
    jwt_secret: &str,
) -> Result<(), std::io::Error> {
    let content = std::fs::read_to_string(config_path)?;
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

    let mut in_auth_section = false;
    let mut secret_updated = false;

    for i in 0..lines.len() {
        let line = lines[i].trim();

        // Check if we're entering the [auth] section
        if line == "[auth]" {
            in_auth_section = true;
            continue;
        }

        // Check if we're leaving the [auth] section
        if in_auth_section && line.starts_with('[') && line.ends_with(']') {
            // If we didn't find jwt_secret in the auth section, add it before the next section
            if !secret_updated {
                lines.insert(i, format!("jwt_secret = \"{}\"", jwt_secret));
                secret_updated = true;
            }
            break;
        }

        // If we're in the auth section and found a jwt_secret line (commented or not)
        if in_auth_section && (line.starts_with("jwt_secret") || line.starts_with("# jwt_secret")) {
            lines[i] = format!("jwt_secret = \"{}\"", jwt_secret);
            secret_updated = true;
            break;
        }
    }

    // If we're still in auth section at the end of file and haven't updated the secret
    if in_auth_section && !secret_updated {
        lines.push(format!("jwt_secret = \"{}\"", jwt_secret));
    }

    // Write the updated content back to the file
    let updated_content = lines.join("\n") + "\n";
    std::fs::write(config_path, updated_content)?;

    Ok(())
}
