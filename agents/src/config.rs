use std::{fs, path::Path, path::PathBuf};

use crate::error::AgentError;

/// Provider identifier constants — mirror llm_sdk::providers
pub const PROVIDER_OPENAI: &str = "openai";
pub const PROVIDER_ANTHROPIC: &str = "anthropic";

/// Runtime configuration for an agent: which LLM to use and how to authenticate.
#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub provider: String,
    pub model: String,
    pub api_key: String,
}

impl AgentConfig {
    /// Load from environment variables, falling back to project.conf.
    ///
    /// Keys read:
    ///   AGENT_PROVIDER  (default: "openai")
    ///   AGENT_MODEL     (default: llm_sdk::models::openai::GPT_5_MINI_ID)
    ///   OPENAI_API_KEY  — required when provider is "openai"
    ///   ANTHROPIC_API_KEY — required when provider is "anthropic"
    pub fn load() -> Result<Self, AgentError> {
        let provider = std::env::var("AGENT_PROVIDER")
            .ok()
            .or_else(|| read_project_conf("AGENT_PROVIDER"))
            .unwrap_or_else(|| PROVIDER_OPENAI.to_string());

        let default_model = match provider.as_str() {
            PROVIDER_ANTHROPIC => llm_sdk::models::claude::SONNET_4_5_ID.to_string(),
            _ => llm_sdk::models::openai::GPT_5_MINI_ID.to_string(),
        };

        let model = std::env::var("AGENT_MODEL")
            .ok()
            .or_else(|| read_project_conf("AGENT_MODEL"))
            .unwrap_or(default_model);

        let key_name = match provider.as_str() {
            PROVIDER_ANTHROPIC => "ANTHROPIC_API_KEY",
            _ => "OPENAI_API_KEY",
        };

        let api_key = std::env::var(key_name)
            .ok()
            .or_else(|| read_project_conf(key_name))
            .ok_or_else(|| {
                AgentError::Config(format!(
                    "{} not set — add it to environment or project.conf",
                    key_name
                ))
            })?;

        Ok(AgentConfig {
            provider,
            model,
            api_key,
        })
    }
}

// ---------------------------------------------------------------------------
// Internal config file reader — same pattern as backend/src/config.rs
// ---------------------------------------------------------------------------

fn read_conf_file(path: &Path, key: &str) -> Option<String> {
    let contents = fs::read_to_string(path).ok()?;
    for line in contents.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, value)) = line.split_once('=') else {
            continue;
        };
        if k.trim() != key {
            continue;
        }
        let value = value.trim().trim_matches('"').trim_matches('\'');
        if !value.is_empty() {
            return Some(value.to_string());
        }
    }
    None
}

fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe().ok()?.parent().map(|p| p.to_path_buf())
}

fn read_project_conf(key: &str) -> Option<String> {
    let mut candidates = vec![
        PathBuf::from("project.conf"),
        PathBuf::from("../project.conf"),
    ];
    if let Some(dir) = exe_dir() {
        candidates.push(dir.join("server.env"));
    }
    candidates.iter().find_map(|p| read_conf_file(p, key))
}
