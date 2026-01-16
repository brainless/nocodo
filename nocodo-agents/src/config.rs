use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Deserialize)]
pub struct Config {
    #[serde(default)]
    pub api_keys: HashMap<String, toml::Value>,
}

pub struct ZaiConfig {
    pub api_key: String,
    pub coding_plan: bool,
}

pub fn load_config(path: &PathBuf) -> anyhow::Result<Config> {
    let content = std::fs::read_to_string(path)?;
    let config: Config = toml::from_str(&content)?;
    Ok(config)
}

pub fn get_zai_config(config: &Config) -> anyhow::Result<ZaiConfig> {
    let api_key = config
        .api_keys
        .get("zai_api_key")
        .and_then(|v| v.as_str())
        .or_else(|| config.api_keys.get("ZAI_API_KEY").and_then(|v| v.as_str()))
        .map(|s| s.to_string())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "ZAI API key not found in config. Expected 'zai_api_key' or 'ZAI_API_KEY'"
            )
        })?;

    let coding_plan = config
        .api_keys
        .get("zai_coding_plan")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    Ok(ZaiConfig {
        api_key,
        coding_plan,
    })
}
