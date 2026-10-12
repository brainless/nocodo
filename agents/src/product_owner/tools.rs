use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// PO calls this to record a business-layer artifact (goal, constraint, decision, etc.)
/// discovered during intake. Can be called multiple times per session.
///
/// When `content_type` is set (e.g. `"persona"`), the `note` field carries
/// JSON conforming to the schema for that type. This lets downstream agents
/// (PM, Praxis Writer) parse the note deterministically.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RecordProjectNoteParams {
    /// Topic category. Must be one of: goal, constraint, decision, context, assumption
    pub topic: String,
    /// The note content. Be concise and factual.
    /// For structured notes (when content_type is set), this is a JSON string.
    pub note: String,
    /// Exact text of an existing current note that this one supersedes. Omit for new facts.
    pub replaces_note: Option<String>,
    /// Id of an existing current project note that this one supersedes. Prefer this
    /// over `replaces_note` when the id is available.
    pub replaces_note_id: Option<i64>,
    /// Optional content type tag (e.g. "persona"). When set, the note is treated
    /// as structured data following the schema for that type.
    pub content_type: Option<String>,
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

/// PO calls this (in persona interview mode) when all personas have been
/// fully documented. Backend creates a planning session and fires PM.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CompletePersonaInterviewParams {
    /// Short message summarising what was covered, for the user.
    pub closing_message: String,
}

/// Comment on an epic or task.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PoCommentParams {
    pub epic_id: Option<i64>,
    pub task_id: Option<i64>,
    pub content: String,
}

/// PO calls this (in gap clarification mode) when all gaps have been addressed.
/// Backend re-runs Praxis Writer with the updated persona notes.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CompleteGapClarificationParams {
    /// Short message summarising what was filled, for the user.
    pub closing_message: String,
}
