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
    /// NULL for human messages; agent identifier for agent-originated rows.
    pub agent_type: Option<String>,
    /// Plain text for user/tool rows; arguments JSON for assistant tool-call rows.
    pub content: String,
    /// Set on role="assistant" tool-call rows and their matching role="tool" result rows.
    pub tool_call_id: Option<String>,
    /// Set on role="assistant" (invocation) and role="tool" (result) rows.
    pub tool_name: Option<String>,
    /// id of the first row in this turn; equals id for single-row turns.
    /// Managed by the storage layer — callers always pass None.
    pub turn_id: Option<i64>,
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

    /// Persist a single-row turn (user messages, nudges). Sets turn_id = id automatically.
    async fn create_message(&self, msg: ChatMessage) -> Result<i64, AgentError>;

    /// Persist all rows of one LLM response turn atomically.
    /// The storage layer assigns turn_id = id of the first inserted row to every row.
    async fn create_turn(&self, messages: Vec<ChatMessage>) -> Result<i64, AgentError>;

    async fn get_messages(&self, session_id: i64) -> Result<Vec<ChatMessage>, AgentError>;
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

    /// Retrieve a schema for a session. If version is None, returns the latest.
    async fn get_schema_for_session(
        &self,
        session_id: i64,
        version: Option<i64>,
    ) -> Result<Option<(String, i64)>, AgentError>;
}
