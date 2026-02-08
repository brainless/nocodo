use crate::types::{Message, Session, ToolCall};
use async_trait::async_trait;

mod memory;

pub use memory::InMemoryStorage;

#[async_trait]
pub trait AgentStorage: Send + Sync {
    async fn create_session(&self, session: Session) -> Result<i64, StorageError>;
    async fn get_session(&self, session_id: i64) -> Result<Option<Session>, StorageError>;
    async fn update_session(&self, session: Session) -> Result<(), StorageError>;

    async fn create_message(&self, message: Message) -> Result<i64, StorageError>;
    async fn get_messages(&self, session_id: i64) -> Result<Vec<Message>, StorageError>;

    async fn create_tool_call(&self, tool_call: ToolCall) -> Result<i64, StorageError>;
    async fn update_tool_call(&self, tool_call: ToolCall) -> Result<(), StorageError>;
    async fn get_tool_calls(&self, session_id: i64) -> Result<Vec<ToolCall>, StorageError>;
    async fn get_pending_tool_calls(&self, session_id: i64) -> Result<Vec<ToolCall>, StorageError>;
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Storage operation failed: {0}")]
    OperationFailed(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Other error: {0}")]
    Other(String),
}

impl From<anyhow::Error> for StorageError {
    fn from(err: anyhow::Error) -> Self {
        StorageError::Other(err.to_string())
    }
}
