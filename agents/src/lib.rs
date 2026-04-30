pub mod config;
pub mod error;
pub mod schema_designer;
pub mod storage;

pub use config::AgentConfig;
pub use error::AgentError;
pub use schema_designer::{AgentResponse, SchemaDesignerAgent, StopAgentParams};
pub use storage::sqlite::{SqliteAgentStorage, SqliteSchemaStorage};
pub use storage::{AgentStorage, AgentType, ChatMessage, SchemaStorage, Session};

// ---------------------------------------------------------------------------
// Factory helpers
// ---------------------------------------------------------------------------

use std::sync::Arc;

/// Build a `SchemaDesignerAgent` from config + SQLite connection paths.
///
/// `db_path` should be the same SQLite file used by the backend (contains the
/// migrated tables).  The same file can be opened concurrently; each storage
/// wrapper holds its own connection.
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
