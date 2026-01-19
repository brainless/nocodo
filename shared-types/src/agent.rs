use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Agent information for the agents list
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
}

/// Configuration for SQLite analysis agent
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SqliteAgentConfig {
    pub db_path: String,
}

/// Configuration for codebase analysis agent
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CodebaseAnalysisAgentConfig {
    pub path: String,
    pub max_depth: Option<usize>,
}

/// Configuration for Tesseract OCR agent
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TesseractAgentConfig {
    pub image_path: String,
}

/// Configuration for Structured JSON agent
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct StructuredJsonAgentConfig {
    pub type_names: Vec<String>,
    pub domain_description: String,
}

/// Configuration for Requirements Gathering agent
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RequirementsGatheringAgentConfig {
    // No configuration needed - agent just needs user prompt
}

/// Configuration for Settings Management agent
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SettingsManagementAgentConfig {
    pub settings_file_path: String,
    pub agent_schemas: Vec<AgentSettingsSchema>,
}

/// Type of setting value
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum SettingType {
    Text,
    Password,
    FilePath,
    Email,
    Url,
    Boolean,
}

/// Definition of a single setting that an agent needs
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SettingDefinition {
    pub name: String,
    pub label: String,
    pub description: String,
    pub setting_type: SettingType,
    pub required: bool,
    pub default_value: Option<String>,
}

/// Schema describing all settings an agent needs
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AgentSettingsSchema {
    pub agent_name: String,
    pub section_name: String,
    pub settings: Vec<SettingDefinition>,
}

/// Variant-specific agent configurations
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(tag = "type")]
pub enum AgentConfig {
    #[serde(rename = "sqlite")]
    Sqlite(SqliteAgentConfig),
    #[serde(rename = "codebase-analysis")]
    CodebaseAnalysis(CodebaseAnalysisAgentConfig),
    #[serde(rename = "tesseract")]
    Tesseract(TesseractAgentConfig),
    #[serde(rename = "structured-json")]
    StructuredJson(StructuredJsonAgentConfig),
    #[serde(rename = "requirements-gathering")]
    RequirementsGathering(RequirementsGatheringAgentConfig),
    #[serde(rename = "settings-management")]
    SettingsManagement(SettingsManagementAgentConfig),
}

/// Generic agent execution request with type-safe config
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AgentExecutionRequest {
    pub user_prompt: String,
    pub config: AgentConfig,
}

/// Response containing list of available agents
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AgentsResponse {
    pub agents: Vec<AgentInfo>,
}
