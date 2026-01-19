use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub mod agent;
pub mod project_management;
pub mod session;
pub mod typescript_gen;
pub mod user_interaction;

pub use typescript_gen::generate_typescript_definitions;

pub use agent::{
    AgentConfig, AgentExecutionRequest, AgentInfo, AgentSettingsSchema, AgentsResponse,
    CodebaseAnalysisAgentConfig, SettingDefinition, SettingType, SettingsManagementAgentConfig,
    SqliteAgentConfig, StructuredJsonAgentConfig, TesseractAgentConfig,
};
pub use project_management::{
    Project, SaveWorkflowRequest, Workflow, WorkflowStep, WorkflowStepData, WorkflowWithSteps,
};
pub use session::{
    AgentExecutionResponse, SessionListItem, SessionListResponse, SessionMessage, SessionResponse,
    SessionToolCall,
};
pub use user_interaction::{
    AskUserRequest, AskUserResponse, QuestionType, UserQuestion, UserQuestionResponse,
};

// Shared models for nocodo API and desktop-app

/// API key configuration for the settings page
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ApiKeyConfig {
    pub name: String,
    pub key: Option<String>, // Will be masked for security
    pub is_configured: bool,
}

/// Settings response containing API keys and configuration info
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SettingsResponse {
    pub config_file_path: String,
    pub api_keys: Vec<ApiKeyConfig>,
    pub projects_default_path: Option<String>,
}

/// Request for updating API keys
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct UpdateApiKeysRequest {
    pub xai_api_key: Option<String>,
    pub openai_api_key: Option<String>,
    pub anthropic_api_key: Option<String>,
    pub zai_api_key: Option<String>,
    pub zai_coding_plan: Option<bool>,
}
