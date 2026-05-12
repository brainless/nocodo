use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// Re-export the canonical definition types from shared-types so agent code
// can import from one place.
pub use shared_types::{ColumnDef, DataType, ForeignKeyDef, SchemaDef, TableDef};

/// Argument type for the `stop_agent` tool.
///
/// The model calls this when the user's request cannot be expressed as a
/// relational database schema (e.g. a prose question or out-of-domain task).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StopAgentParams {
    /// Human-readable reply explaining why no schema was produced.
    pub reply: String,
}

/// Argument type for the `ask_user` tool.
///
/// The model calls this when it needs clarifying information from the user
/// before it can design a proper schema. The question may be plain text or
/// Markdown formatted.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AskUserParams {
    /// The question to ask the user. May be plain text or Markdown.
    pub question: String,
}

/// Argument type for the `update_task_status` tool.
///
/// The model calls this to record a status transition on the current task.
/// Valid status values: "in_progress", "review", "done", "blocked".
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UpdateTaskStatusParams {
    /// New task status. Must be one of: "in_progress", "review", "done", "blocked".
    pub status: String,
}
