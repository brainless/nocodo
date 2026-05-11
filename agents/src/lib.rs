pub mod backend_engineer;
pub mod config;
pub mod error;
pub mod frontend_engineer;
pub mod pm_agent;
pub mod schema_designer;
pub mod storage;
pub mod ui_designer;
pub mod utils;

pub use backend_engineer::{BackendEngineerAgent, BackendEngineerResponse};
pub use config::AgentConfig;
pub use error::AgentError;
pub use frontend_engineer::{FrontendEngineerAgent, FrontendEngineerResponse};
pub use pm_agent::{PmAgent, PmResponse};
pub use schema_designer::{AgentResponse, SchemaDesignerAgent, StopAgentParams};
pub use storage::sqlite::{
    SqliteAgentStorage, SqliteContextStorage, SqliteSchemaStorage, SqliteTaskStorage,
    SqliteUiFormStorage,
};
pub use storage::{
    AgentStorage, AgentType, ChatMessage, ContextStorage, Epic, EpicStatus, SchemaStorage, Session,
    Task, TaskStatus, TaskStorage, UiFormStorage,
};
pub use ui_designer::{
    agent::{UiDesignerAgent, UiDesignerResponse},
    FormField, FormFieldType, FormLayout, FormRow,
};

// ---------------------------------------------------------------------------
// Factory helpers
// ---------------------------------------------------------------------------

use std::sync::Arc;

fn make_llm_client(
    config: &AgentConfig,
) -> Result<Arc<dyn llm_sdk::client::LlmClient>, AgentError> {
    use llm_sdk::{claude::ClaudeClient, groq::GroqClient, openai::OpenAIClient};
    let client: Arc<dyn llm_sdk::client::LlmClient> = match config.provider.as_str() {
        config::PROVIDER_ANTHROPIC => Arc::new(
            ClaudeClient::new(config.api_key.clone())
                .map_err(|e| AgentError::Config(e.to_string()))?,
        ),
        config::PROVIDER_GROQ => Arc::new(
            GroqClient::new(config.api_key.clone())
                .map_err(|e| AgentError::Config(e.to_string()))?,
        ),
        _ => Arc::new(
            OpenAIClient::new(config.api_key.clone())
                .map_err(|e| AgentError::Config(e.to_string()))?,
        ),
    };
    Ok(client)
}

pub fn build_schema_designer(
    config: &AgentConfig,
    db_path: &str,
    project_id: i64,
) -> Result<SchemaDesignerAgent, AgentError> {
    let client = make_llm_client(config)?;
    let storage: Arc<dyn AgentStorage> = Arc::new(SqliteAgentStorage::open(db_path)?);
    let schema_storage: Arc<dyn SchemaStorage> = Arc::new(SqliteSchemaStorage::open(db_path)?);
    let task_storage: Arc<dyn TaskStorage> = Arc::new(SqliteTaskStorage::open(db_path)?);

    Ok(SchemaDesignerAgent::new(
        client,
        storage,
        schema_storage,
        task_storage,
        &config.model,
        project_id,
    ))
}

pub fn build_pm_agent(
    config: &AgentConfig,
    db_path: &str,
    project_id: i64,
) -> Result<PmAgent, AgentError> {
    let client = make_llm_client(config)?;
    let storage: Arc<dyn AgentStorage> = Arc::new(SqliteAgentStorage::open(db_path)?);
    let task_storage: Arc<dyn TaskStorage> = Arc::new(SqliteTaskStorage::open(db_path)?);

    Ok(PmAgent::new(client, storage, task_storage, &config.model, project_id))
}

pub fn build_ui_designer(
    config: &AgentConfig,
    db_path: &str,
    project_id: i64,
) -> Result<UiDesignerAgent, AgentError> {
    let client = make_llm_client(config)?;
    let storage: Arc<dyn AgentStorage> = Arc::new(SqliteAgentStorage::open(db_path)?);
    let form_storage: Arc<dyn UiFormStorage> = Arc::new(SqliteUiFormStorage::open(db_path)?);
    let task_storage: Arc<dyn TaskStorage> = Arc::new(SqliteTaskStorage::open(db_path)?);

    Ok(UiDesignerAgent::new(
        client,
        storage,
        form_storage,
        task_storage,
        &config.model,
        project_id,
    ))
}

pub fn build_pm_agent_with_task_storage(
    config: &AgentConfig,
    db_path: &str,
    project_id: i64,
    task_storage: Arc<dyn TaskStorage>,
) -> Result<PmAgent, AgentError> {
    let client = make_llm_client(config)?;
    let storage: Arc<dyn AgentStorage> = Arc::new(SqliteAgentStorage::open(db_path)?);

    Ok(PmAgent::new(client, storage, task_storage, &config.model, project_id))
}

pub fn build_backend_engineer(
    config: &AgentConfig,
    db_path: &str,
    project_id: i64,
    project_path: &str,
) -> Result<BackendEngineerAgent, AgentError> {
    let client = make_llm_client(config)?;
    let storage: Arc<dyn AgentStorage> = Arc::new(SqliteAgentStorage::open(db_path)?);
    let context_storage: Arc<dyn ContextStorage> = Arc::new(SqliteContextStorage::open(db_path)?);
    let task_storage: Arc<dyn TaskStorage> = Arc::new(SqliteTaskStorage::open(db_path)?);

    Ok(BackendEngineerAgent::new(
        client,
        storage,
        context_storage,
        task_storage,
        &config.model,
        project_id,
        project_path,
    ))
}

pub fn build_frontend_engineer(
    config: &AgentConfig,
    db_path: &str,
    project_id: i64,
    project_path: &str,
) -> Result<FrontendEngineerAgent, AgentError> {
    let client = make_llm_client(config)?;
    let storage: Arc<dyn AgentStorage> = Arc::new(SqliteAgentStorage::open(db_path)?);
    let context_storage: Arc<dyn ContextStorage> = Arc::new(SqliteContextStorage::open(db_path)?);
    let task_storage: Arc<dyn TaskStorage> = Arc::new(SqliteTaskStorage::open(db_path)?);

    Ok(FrontendEngineerAgent::new(
        client,
        storage,
        context_storage,
        task_storage,
        &config.model,
        project_id,
        project_path,
    ))
}
