pub mod sqlite;

use async_trait::async_trait;

use crate::error::AgentError;

// ---------------------------------------------------------------------------
// Agent type registry
// ---------------------------------------------------------------------------

pub enum AgentType {
    SchemaDesigner,
}

impl AgentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentType::SchemaDesigner => "schema_designer",
        }
    }
}

// ---------------------------------------------------------------------------
// Domain types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Session {
    pub id: Option<i64>,
    pub project_id: i64,
    pub agent_type: String,
    pub created_at: i64,
}

#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub id: Option<i64>,
    pub session_id: i64,
    /// "user" | "assistant" | "tool"
    pub role: String,
    pub content: String,
    /// Set for role="tool": the LLM-assigned call_id this result belongs to.
    pub tool_call_id: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone)]
pub struct ToolCallRecord {
    pub id: Option<i64>,
    pub message_id: i64,
    /// LLM-assigned call identifier (used to correlate tool results).
    pub call_id: String,
    pub tool_name: String,
    /// JSON-serialised arguments (raw LLM tool call input).
    pub arguments: String,
    pub created_at: i64,
}

// ---------------------------------------------------------------------------
// Storage trait
// ---------------------------------------------------------------------------

#[async_trait]
pub trait AgentStorage: Send + Sync {
    /// Return the existing session for (project_id, agent_type) or create one.
    async fn get_or_create_session(
        &self,
        project_id: i64,
        agent_type: &str,
    ) -> Result<Session, AgentError>;

    async fn create_message(&self, msg: ChatMessage) -> Result<i64, AgentError>;

    async fn get_messages(&self, session_id: i64) -> Result<Vec<ChatMessage>, AgentError>;

    async fn create_tool_call(&self, record: ToolCallRecord) -> Result<i64, AgentError>;
}

// ---------------------------------------------------------------------------
// Schema storage trait
// ---------------------------------------------------------------------------

#[async_trait]
pub trait SchemaStorage: Send + Sync {
    /// Persist a new schema version.  Returns the new row id.
    async fn save_schema(
        &self,
        project_id: i64,
        session_id: i64,
        schema_json: &str,
    ) -> Result<i64, AgentError>;

    /// Next version number for a given project (latest + 1, or 1 if none).
    async fn next_version(&self, project_id: i64) -> Result<i64, AgentError>;
}
