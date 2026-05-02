pub mod config;
pub mod error;
pub mod schema_designer;
pub mod storage;

pub use config::AgentConfig;
pub use error::AgentError;
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

/// Build a `SchemaDesignerAgent` from config + SQLite path.
/// The caller is responsible for creating the task and session before calling
/// `agent.chat_with_session(session_id, preview_mode)`.
pub fn build_schema_designer(
    config: &AgentConfig,
    db_path: &str,
    project_id: i64,
) -> Result<SchemaDesignerAgent, AgentError> {
    use llm_sdk::{claude::ClaudeClient, openai::OpenAIClient};

    let client: Arc<dyn llm_sdk::client::LlmClient> = match config.provider.as_str() {
        config::PROVIDER_ANTHROPIC => Arc::new(
            ClaudeClient::new(config.api_key.clone())
                .map_err(|e| AgentError::Config(e.to_string()))?,
        ),
        _ => Arc::new(
            OpenAIClient::new(config.api_key.clone())
                .map_err(|e| AgentError::Config(e.to_string()))?,
        ),
    };

    let storage: Arc<dyn AgentStorage> = Arc::new(SqliteAgentStorage::open(db_path)?);
    let schema_storage: Arc<dyn SchemaStorage> = Arc::new(SqliteSchemaStorage::open(db_path)?);

    Ok(SchemaDesignerAgent::new(
        client,
        storage,
        schema_storage,
        &config.model,
        project_id,
    ))
}
