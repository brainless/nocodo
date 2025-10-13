use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesktopConfig {
    pub ssh: SshConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshConfig {
    pub server: String,
    pub username: String,
    pub ssh_key_path: String,
    pub remote_port: u16,
}

impl Default for DesktopConfig {
    fn default() -> Self {
        Self {
            ssh: SshConfig {
                server: "dev-server.example.com".to_string(),
                username: "nocodo".to_string(),
                ssh_key_path: "~/.ssh/id_ed25519".to_string(),
                remote_port: 8081,
            },
        }
    }
}

impl DesktopConfig {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = Self::config_path();
        if config_path.exists() {
            let content = std::fs::read_to_string(config_path)?;
            let config: DesktopConfig = toml::from_str(&content)?;
            Ok(config)
        } else {
            Ok(Default::default())
        }
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = Self::config_path();
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }

    fn config_path() -> PathBuf {
        if let Some(home) = dirs::home_dir() {
            home.join(".config").join("nocodo").join("desktop.toml")
        } else {
            PathBuf::from("./nocodo-desktop.toml")
        }
    }
}