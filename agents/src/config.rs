use std::{fs, path::Path, path::PathBuf};

use crate::error::AgentError;

/// Provider identifier constants — mirror llm_sdk::providers
pub const PROVIDER_OPENAI: &str = "openai";
pub const PROVIDER_ANTHROPIC: &str = "anthropic";
pub const PROVIDER_GROQ: &str = "groq";

/// Runtime configuration for an agent: which LLM to use and how to authenticate.
#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub provider: String,
    pub model: String,
    pub api_key: String,
}

impl AgentConfig {
    /// Load default agent config (AGENT_PROVIDER / AGENT_MODEL).
    pub fn load() -> Result<Self, AgentError> {
        Self::load_from_prefix("AGENT")
    }

    /// Load schema-designer config: SCHEMA_AGENT_* with fallback to AGENT_*.
    pub fn load_schema_designer() -> Result<Self, AgentError> {
        Self::load_with_agent_fallback("SCHEMA_AGENT")
    }

    /// Load UI-designer config: UI_AGENT_* with fallback to AGENT_*.
    pub fn load_ui_designer() -> Result<Self, AgentError> {
        Self::load_with_agent_fallback("UI_AGENT")
    }

    /// Load PM agent config: PM_AGENT_* with fallback to AGENT_*.
    pub fn load_pm() -> Result<Self, AgentError> {
        Self::load_with_agent_fallback("PM_AGENT")
    }

    /// Try `{prefix}_PROVIDER` / `{prefix}_MODEL` first; fall back to AGENT_* defaults.
    fn load_with_agent_fallback(prefix: &str) -> Result<Self, AgentError> {
        let provider_key = format!("{}_PROVIDER", prefix);
        let model_key = format!("{}_MODEL", prefix);

        let provider = std::env::var(&provider_key)
            .ok()
            .or_else(|| read_project_conf(&provider_key))
            .or_else(|| std::env::var("AGENT_PROVIDER").ok())
            .or_else(|| read_project_conf("AGENT_PROVIDER"))
            .unwrap_or_else(|| PROVIDER_OPENAI.to_string());

        let default_model = match provider.as_str() {
            PROVIDER_ANTHROPIC => llm_sdk::models::claude::SONNET_4_5_ID.to_string(),
            PROVIDER_GROQ => llm_sdk::models::groq::GPT_OSS_120B_ID.to_string(),
            _ => llm_sdk::models::openai::GPT_5_MINI_ID.to_string(),
        };

        let model = std::env::var(&model_key)
            .ok()
            .or_else(|| read_project_conf(&model_key))
            .or_else(|| std::env::var("AGENT_MODEL").ok())
            .or_else(|| read_project_conf("AGENT_MODEL"))
            .unwrap_or(default_model);

        let key_name = match provider.as_str() {
            PROVIDER_ANTHROPIC => "ANTHROPIC_API_KEY",
            PROVIDER_GROQ => "GROQ_API_KEY",
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

        Ok(AgentConfig { provider, model, api_key })
    }

    fn load_from_prefix(prefix: &str) -> Result<Self, AgentError> {
        let provider_key = format!("{}_PROVIDER", prefix);
        let model_key = format!("{}_MODEL", prefix);

        let provider = std::env::var(&provider_key)
            .ok()
            .or_else(|| read_project_conf(&provider_key))
            .unwrap_or_else(|| PROVIDER_OPENAI.to_string());

        let default_model = match provider.as_str() {
            PROVIDER_ANTHROPIC => llm_sdk::models::claude::SONNET_4_5_ID.to_string(),
            PROVIDER_GROQ => llm_sdk::models::groq::GPT_OSS_120B_ID.to_string(),
            _ => llm_sdk::models::openai::GPT_5_MINI_ID.to_string(),
        };

        let model = std::env::var(&model_key)
            .ok()
            .or_else(|| read_project_conf(&model_key))
            .unwrap_or(default_model);

        let key_name = match provider.as_str() {
            PROVIDER_ANTHROPIC => "ANTHROPIC_API_KEY",
            PROVIDER_GROQ => "GROQ_API_KEY",
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

        Ok(AgentConfig { provider, model, api_key })
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
        let value = value.trim();
        let value = value.split_once(" #").map(|(v, _)| v).unwrap_or(value).trim();
        let value = value.trim_matches('"').trim_matches('\'');
        if !value.is_empty() {
            return Some(value.to_string());
        }
    }
    None
}

fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()?
        .parent()
        .map(|p| p.to_path_buf())
}

fn read_project_conf(key: &str) -> Option<String> {
    let mut candidates = vec![
        PathBuf::from("project.conf"),
        PathBuf::from("../project.conf"),
        PathBuf::from("../../project.conf"),
    ];

    // Check relative to executable (for Tauri apps)
    if let Some(dir) = exe_dir() {
        candidates.push(dir.join("server.env"));
        candidates.push(dir.join("project.conf"));
        candidates.push(dir.parent()?.join("project.conf"));
        candidates.push(dir.parent()?.parent()?.join("project.conf"));
    }

    // Allow PROJECT_ROOT env var to specify where project.conf lives
    if let Ok(project_root) = std::env::var("PROJECT_ROOT") {
        candidates.push(PathBuf::from(&project_root).join("project.conf"));
    }

    candidates.iter().find_map(|p| read_conf_file(p, key))
}
