//! Project configuration (`project.json`).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectConfig {
    pub spec_version: u32,
    pub name: String,
    /// Absent in epic 003 projects; defaults to the default models.
    #[serde(default)]
    pub models: ModelsConfig,
}

/// Settings every new project starts from, read from `project-base.json`.
/// `init` copies them into the new project's `project.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectBase {
    #[serde(default)]
    pub models: ModelsConfig,
}

/// Per-role model configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelsConfig {
    #[serde(default)]
    pub schema: ModelConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelConfig {
    pub provider: Provider,
    pub model: String,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            provider: Provider::Anthropic,
            model: DEFAULT_MODEL.to_string(),
        }
    }
}

pub const DEFAULT_MODEL: &str = "claude-sonnet-5-5";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    Anthropic,
}
