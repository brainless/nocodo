use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// PO calls this when requirements are gathered — closes intake and hands off to PM.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HandOffToPmParams {
    /// Short closing message to show the user (e.g. "Great, I have everything I need!").
    pub final_message: String,
    /// Structured requirements brief for the PM: business context, workflow, key data entities,
    /// scope decisions, and anything else PM needs to create the epic and tasks.
    pub summary: String,
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
