pub mod message_content;
pub mod sqlite;

use async_trait::async_trait;
use serde::Serialize;

use crate::error::AgentError;
pub use message_content::{MessageContent, QuestionKind, StructuredQuestion, StructuredResponse};

// ---------------------------------------------------------------------------
// Agent type registry
// ---------------------------------------------------------------------------

pub enum AgentType {
    DbEngineer,
    ProjectManager,
    UiDesigner,
    BackendEngineer,
    FrontendEngineer,
    ProductOwner,
    EngineeringManager,
}

impl AgentType {
    pub fn from_str(s: &str) -> Self {
        match s {
            "db_engineer" => AgentType::DbEngineer,
            "project_manager" => AgentType::ProjectManager,
            "ui_designer" => AgentType::UiDesigner,
            "backend_engineer" => AgentType::BackendEngineer,
            "frontend_engineer" => AgentType::FrontendEngineer,
            "product_owner" => AgentType::ProductOwner,
            "engineering_manager" => AgentType::EngineeringManager,
            _ => AgentType::ProjectManager,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            AgentType::DbEngineer => "db_engineer",
            AgentType::ProjectManager => "project_manager",
            AgentType::UiDesigner => "ui_designer",
            AgentType::BackendEngineer => "backend_engineer",
            AgentType::FrontendEngineer => "frontend_engineer",
            AgentType::ProductOwner => "product_owner",
            AgentType::EngineeringManager => "engineering_manager",
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
    Draft,
    NeedsTechnicalShaping,
    Ready,
    InProgress,
    Done,
    Blocked,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::NeedsTechnicalShaping => "needs_technical_shaping",
            Self::Ready => "ready",
            Self::InProgress => "in_progress",
            Self::Done => "done",
            Self::Blocked => "blocked",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "draft" => Self::Draft,
            "needs_technical_shaping" => Self::NeedsTechnicalShaping,
            "ready" => Self::Ready,
            "in_progress" => Self::InProgress,
            "done" => Self::Done,
            "blocked" => Self::Blocked,
            // Legacy mappings
            "open" => Self::Ready,
            "review" => Self::Done,
            _ => Self::Draft,
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
// User / User-chat / Comment row structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct UserRow {
    pub id: i64,
    pub display_name: String,
    pub email: Option<String>,
    pub is_guest: bool,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserChatSessionRow {
    pub id: i64,
    pub project_id: i64,
    pub created_by_user_id: i64,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub completed_at: Option<i64>,
    pub handoff_session_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserChatMessageRow {
    pub id: i64,
    pub session_id: i64,
    pub author_type: String,
    pub author_user_id: Option<i64>,
    pub agent_type: Option<String>,
    pub turn_id: Option<i64>,
    pub content_type: String,
    pub content: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct EpicCommentRow {
    pub id: i64,
    pub epic_id: i64,
    pub author_type: String,
    pub author_user_id: Option<i64>,
    pub agent_type: Option<String>,
    pub content: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskCommentRow {
    pub id: i64,
    pub task_id: i64,
    pub author_type: String,
    pub author_user_id: Option<i64>,
    pub agent_type: Option<String>,
    pub content: String,
    pub created_at: i64,
    pub updated_at: i64,
}

// ---------------------------------------------------------------------------
// AgentStorage trait — used by agents (session + message management)
// ---------------------------------------------------------------------------

#[async_trait]
pub trait AgentStorage: Send + Sync {
    /// Rename a project. Called by the PO agent during project naming.
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
    async fn list_pending_review_tasks(&self, project_id: i64) -> Result<Vec<Task>, AgentError>;

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
    async fn list_form_layouts(&self, project_id: i64)
        -> Result<Vec<(String, String)>, AgentError>;
}

// ---------------------------------------------------------------------------
// Stack notes — tech stack overview for the Engineering Manager agent
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum StackTag {
    Backend,
    Database,
    Frontend,
    Auth,
    ApiContract,
    Config,
    Tooling,
    Deployment,
    Testing,
}

impl StackTag {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Backend => "backend",
            Self::Database => "database",
            Self::Frontend => "frontend",
            Self::Auth => "auth",
            Self::ApiContract => "api_contract",
            Self::Config => "config",
            Self::Tooling => "tooling",
            Self::Deployment => "deployment",
            Self::Testing => "testing",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "backend" => Self::Backend,
            "database" => Self::Database,
            "frontend" => Self::Frontend,
            "auth" => Self::Auth,
            "api_contract" => Self::ApiContract,
            "config" => Self::Config,
            "tooling" => Self::Tooling,
            "deployment" => Self::Deployment,
            "testing" => Self::Testing,
            _ => Self::Backend,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct StackNoteRow {
    pub id: i64,
    pub project_id: i64,
    pub tag: String,
    pub note: String,
    pub file_path: Option<String>,
    pub line_number: Option<i64>,
    pub replaces_id: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
}

// ---------------------------------------------------------------------------
// ContextStorage trait — persists gathered project context
// ---------------------------------------------------------------------------

#[async_trait]
pub trait ContextStorage: Send + Sync {
    /// Upsert context for a (project_id, context_type) pair.
    async fn save_context(
        &self,
        project_id: i64,
        context_type: &str,
        context: &str,
    ) -> Result<(), AgentError>;

    /// Retrieve context for a (project_id, context_type) pair, if it exists.
    async fn get_context(
        &self,
        project_id: i64,
        context_type: &str,
    ) -> Result<Option<String>, AgentError>;
}

// ---------------------------------------------------------------------------
// UserStorage trait — guest user management
// ---------------------------------------------------------------------------

#[async_trait]
pub trait UserStorage: Send + Sync {
    async fn create_guest_user(&self, display_name: String) -> Result<i64, AgentError>;

    async fn get_user(&self, user_id: i64) -> Result<Option<UserRow>, AgentError>;

    async fn update_display_name(
        &self,
        user_id: i64,
        display_name: String,
    ) -> Result<(), AgentError>;
}

// ---------------------------------------------------------------------------
// UserChatStorage trait — user chat session + message management
// ---------------------------------------------------------------------------

#[async_trait]
pub trait UserChatStorage: Send + Sync {
    async fn create_session(&self, project_id: i64, user_id: i64) -> Result<i64, AgentError>;

    async fn get_session(&self, session_id: i64) -> Result<Option<UserChatSessionRow>, AgentError>;

    async fn append_message(
        &self,
        session_id: i64,
        author_type: &str,
        author_user_id: Option<i64>,
        agent_type: Option<AgentType>,
        turn_id: Option<i64>,
        content: MessageContent,
    ) -> Result<i64, AgentError>;

    async fn get_messages(&self, session_id: i64) -> Result<Vec<UserChatMessageRow>, AgentError>;

    async fn complete_session(&self, session_id: i64) -> Result<(), AgentError>;

    async fn set_handoff_session_id(
        &self,
        intake_id: i64,
        planning_id: i64,
    ) -> Result<(), AgentError>;
}

// ---------------------------------------------------------------------------
// StackNoteStorage trait — tech stack notes for the Engineering Manager
// ---------------------------------------------------------------------------

#[async_trait]
pub trait StackNoteStorage: Send + Sync {
    /// replaces_note: text of existing current note to supersede (None for new notes).
    /// Errors if replaces_note text not found, or if new note text already exists as a current note.
    async fn add_note(
        &self,
        project_id: i64,
        tag: StackTag,
        note: String,
        file_path: Option<String>,
        line_number: Option<i64>,
        replaces_note: Option<String>,
    ) -> Result<i64, AgentError>;

    async fn list_notes(&self, project_id: i64) -> Result<Vec<StackNoteRow>, AgentError>;

    /// Returns only notes not superseded by any other note (the "current view").
    async fn list_current_notes(&self, project_id: i64) -> Result<Vec<StackNoteRow>, AgentError>;

    async fn list_notes_by_tag(
        &self,
        project_id: i64,
        tag: StackTag,
    ) -> Result<Vec<StackNoteRow>, AgentError>;

    async fn delete_note(&self, note_id: i64) -> Result<(), AgentError>;
}

// ---------------------------------------------------------------------------
// Project notes — business-layer artifacts written by PO
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum ProjectNoteTopic {
    Goal,
    Constraint,
    Decision,
    Context,
    Assumption,
}

impl ProjectNoteTopic {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Goal => "goal",
            Self::Constraint => "constraint",
            Self::Decision => "decision",
            Self::Context => "context",
            Self::Assumption => "assumption",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "goal" => Self::Goal,
            "constraint" => Self::Constraint,
            "decision" => Self::Decision,
            "context" => Self::Context,
            "assumption" => Self::Assumption,
            _ => Self::Context,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectNoteRow {
    pub id: i64,
    pub project_id: i64,
    pub topic: String,
    pub note: String,
    pub source_session_id: Option<i64>,
    pub source_epic_comment_id: Option<i64>,
    pub source_task_comment_id: Option<i64>,
    pub replaces_id: Option<i64>,
    pub created_at: i64,
}

#[async_trait]
pub trait ProjectNoteStorage: Send + Sync {
    /// replaces_note: exact text of the current note this supersedes (None for new notes).
    /// Errors if replaces_note text not found, or if new note text already exists as a current note.
    async fn add_note(
        &self,
        project_id: i64,
        topic: ProjectNoteTopic,
        note: String,
        source_session_id: Option<i64>,
        replaces_note: Option<String>,
    ) -> Result<i64, AgentError>;

    /// Returns only notes not superseded by any other note (the "current view").
    async fn list_current_notes(
        &self,
        project_id: i64,
    ) -> Result<Vec<ProjectNoteRow>, AgentError>;

    async fn list_notes_by_topic(
        &self,
        project_id: i64,
        topic: ProjectNoteTopic,
    ) -> Result<Vec<ProjectNoteRow>, AgentError>;
}

// ---------------------------------------------------------------------------
// CommentStorage trait — epic & task comments
// ---------------------------------------------------------------------------

#[async_trait]
pub trait CommentStorage: Send + Sync {
    async fn add_epic_comment(
        &self,
        epic_id: i64,
        author_type: &str,
        author_user_id: Option<i64>,
        agent_type: Option<AgentType>,
        content: String,
    ) -> Result<i64, AgentError>;

    async fn get_epic_comments(&self, epic_id: i64) -> Result<Vec<EpicCommentRow>, AgentError>;

    async fn add_task_comment(
        &self,
        task_id: i64,
        author_type: &str,
        author_user_id: Option<i64>,
        agent_type: Option<AgentType>,
        content: String,
    ) -> Result<i64, AgentError>;

    async fn get_task_comments(&self, task_id: i64) -> Result<Vec<TaskCommentRow>, AgentError>;
}
