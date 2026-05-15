use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
