use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// PO calls this to record a business-layer artifact (goal, constraint, decision, etc.)
/// discovered during intake. Can be called multiple times per session.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RecordProjectNoteParams {
    /// Topic category. Must be one of: goal, constraint, decision, context, assumption
    pub topic: String,
    /// Short descriptive title (under 80 characters).
    pub title: String,
    /// The note content. Be concise and factual.
    pub note: String,
    /// Exact text of an existing current note that this one supersedes. Omit for new facts.
    pub replaces_note: Option<String>,
}

/// PO calls this when all questions are answered and project notes are saved.
/// Signals the end of requirements gathering; backend will initiate project naming next.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CompleteRequirementsParams {
    /// Short, warm closing message to show the user before the team gets started.
    pub closing_message: String,
}

/// PO calls this (in project naming mode) to set a descriptive name for the project.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SetProjectNameParams {
    /// A concise, descriptive name for the project (≤ 60 chars).
    /// Derived from the user's domain, e.g. "CRM — Leads & Deals" or "Inventory Tracker".
    pub name: String,
}

/// PO calls this to transition a task out of draft.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ValidateTaskParams {
    pub task_id: i64,
    pub notes: Option<String>,
}

/// Comment on an epic or task.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PoCommentParams {
    pub epic_id: Option<i64>,
    pub task_id: Option<i64>,
    pub content: String,
}
