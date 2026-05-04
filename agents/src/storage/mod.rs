pub mod sqlite;

use async_trait::async_trait;

use crate::error::AgentError;

// ---------------------------------------------------------------------------
// Agent type registry
// ---------------------------------------------------------------------------

pub enum AgentType {
    SchemaDesigner,
    ProjectManager,
    UiDesigner,
}

impl AgentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentType::SchemaDesigner => "schema_designer",
            AgentType::ProjectManager => "project_manager",
            AgentType::UiDesigner => "ui_designer",
        }
    }
}

// ---------------------------------------------------------------------------
// Epic
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Epic {
    pub id: Option<i64>,
    pub project_id: i64,
    pub title: String,
    pub description: String,
    pub source_prompt: String,
    pub status: EpicStatus,
    pub created_by_agent: String,
    pub created_by_task_id: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EpicStatus {
    Open,
    InProgress,
    Done,
}

impl EpicStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::InProgress => "in_progress",
            Self::Done => "done",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "in_progress" => Self::InProgress,
            "done" => Self::Done,
            _ => Self::Open,
        }
    }
}

// ---------------------------------------------------------------------------
// Task
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Task {
    pub id: Option<i64>,
    pub project_id: i64,
    pub epic_id: Option<i64>,
    pub title: String,
    pub description: String,
    pub source_prompt: String,
    pub assigned_to_agent: String,
    pub status: TaskStatus,
    pub depends_on_task_id: Option<i64>,
    pub created_by_agent: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Open,
    InProgress,
    Review,
    Done,
    Blocked,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::InProgress => "in_progress",
            Self::Review => "review",
            Self::Done => "done",
            Self::Blocked => "blocked",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "in_progress" => Self::InProgress,
            "review" => Self::Review,
            "done" => Self::Done,
            "blocked" => Self::Blocked,
            _ => Self::Open,
        }
    }
}

// ---------------------------------------------------------------------------
// Session + ChatMessage
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Session {
    pub id: Option<i64>,
    pub project_id: i64,
    pub agent_type: String,
    pub task_id: i64,
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
// AgentStorage trait — used by agents (session + message management)
// ---------------------------------------------------------------------------

#[async_trait]
pub trait AgentStorage: Send + Sync {
    /// Rename a project. Used by the PM agent during project init.
    async fn rename_project(&self, project_id: i64, name: &str) -> Result<(), AgentError>;

    /// Create a new session for a task. One session per (task_id, agent_type).
    async fn create_task_session(
        &self,
        project_id: i64,
        task_id: i64,
        agent_type: &str,
    ) -> Result<Session, AgentError>;

    /// Find the session for a (task_id, agent_type) pair, if it exists.
    async fn get_session_by_task(
        &self,
        task_id: i64,
        agent_type: &str,
    ) -> Result<Option<Session>, AgentError>;

    /// Persist a single-row turn (user messages, nudges). Sets turn_id = id automatically.
    async fn create_message(&self, msg: ChatMessage) -> Result<i64, AgentError>;

    /// Persist all rows of one LLM response turn atomically.
    async fn create_turn(&self, messages: Vec<ChatMessage>) -> Result<i64, AgentError>;

    async fn get_messages(&self, session_id: i64) -> Result<Vec<ChatMessage>, AgentError>;
}

// ---------------------------------------------------------------------------
// TaskStorage trait — shared communication plane between agents
// ---------------------------------------------------------------------------

#[async_trait]
pub trait TaskStorage: Send + Sync {
    async fn create_task(&self, task: Task) -> Result<i64, AgentError>;
    async fn update_task_status(&self, task_id: i64, status: TaskStatus) -> Result<(), AgentError>;
    async fn get_task(&self, task_id: i64) -> Result<Option<Task>, AgentError>;
    async fn list_tasks_for_project(&self, project_id: i64) -> Result<Vec<Task>, AgentError>;
    async fn list_tasks_for_agent(
        &self,
        project_id: i64,
        agent_type: &str,
    ) -> Result<Vec<Task>, AgentError>;
    async fn list_pending_review_tasks(
        &self,
        project_id: i64,
    ) -> Result<Vec<Task>, AgentError>;

    /// All open tasks across every project that have no agent session yet and are
    /// not assigned to project_manager. Used by the startup reconciliation pass.
    async fn list_open_dispatchable_tasks(&self) -> Result<Vec<Task>, AgentError>;

    async fn create_epic(&self, epic: Epic) -> Result<i64, AgentError>;
    async fn update_epic_status(&self, epic_id: i64, status: EpicStatus) -> Result<(), AgentError>;
    async fn get_epic(&self, epic_id: i64) -> Result<Option<Epic>, AgentError>;
    async fn list_epics(&self, project_id: i64) -> Result<Vec<Epic>, AgentError>;
}

// ---------------------------------------------------------------------------
// SchemaStorage trait
// ---------------------------------------------------------------------------

#[async_trait]
pub trait SchemaStorage: Send + Sync {
    async fn save_schema(
        &self,
        project_id: i64,
        session_id: i64,
        schema_json: &str,
    ) -> Result<i64, AgentError>;

    async fn next_version(&self, project_id: i64) -> Result<i64, AgentError>;

    async fn get_schema_for_session(
        &self,
        session_id: i64,
        version: Option<i64>,
    ) -> Result<Option<(String, i64)>, AgentError>;

    /// Latest schema for the project regardless of session.
    async fn get_latest_schema_for_project(
        &self,
        project_id: i64,
    ) -> Result<Option<String>, AgentError>;
}

// ---------------------------------------------------------------------------
// UiFormStorage trait — persists generated form layouts
// ---------------------------------------------------------------------------

#[async_trait]
pub trait UiFormStorage: Send + Sync {
    /// Upsert the form layout JSON for a (project, entity) pair.
    async fn save_form_layout(
        &self,
        project_id: i64,
        entity_name: &str,
        layout_json: &str,
    ) -> Result<(), AgentError>;

    /// Retrieve the form layout JSON for a (project, entity) pair, if it exists.
    async fn get_form_layout(
        &self,
        project_id: i64,
        entity_name: &str,
    ) -> Result<Option<String>, AgentError>;

    /// List all (entity_name, layout_json) pairs for a project.
    async fn list_form_layouts(
        &self,
        project_id: i64,
    ) -> Result<Vec<(String, String)>, AgentError>;
}
