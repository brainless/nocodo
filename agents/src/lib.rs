pub mod config;
pub mod error;
pub mod pm_agent;
pub mod schema_designer;
pub mod storage;

pub use config::AgentConfig;
pub use error::AgentError;
pub use pm_agent::{PmAgent, PmResponse};
pub use schema_designer::{AgentResponse, SchemaDesignerAgent, StopAgentParams};
pub use storage::sqlite::{SqliteAgentStorage, SqliteSchemaStorage, SqliteTaskStorage};
pub use storage::{
    AgentStorage, AgentType, ChatMessage, Epic, EpicStatus, SchemaStorage, Session, Task,
    TaskStatus, TaskStorage,
};

// ---------------------------------------------------------------------------
// Factory helper
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

/// Build a `SchemaDesignerAgent` from config + SQLite path.
/// The caller is responsible for creating the task and session before calling
/// `agent.chat_with_session(session_id, task_id, preview_mode)`.
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

/// Build a `PmAgent` from config + SQLite path.
/// The caller is responsible for creating the task and session before calling
/// `agent.chat_with_session(session_id, task_id)`.
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
