use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Called at session start to surface tasks awaiting PM triage.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListPendingReviewTasksParams {}

/// Create a new Epic for a user initiative.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateEpicParams {
    /// Short title for the epic (≤ 100 chars).
    pub title: String,
    /// Longer description of the goal and scope.
    pub description: String,
}

/// Create a task and assign it to a focused agent.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateTaskParams {
    /// Short title (≤ 100 chars).
    pub title: String,
    /// Description of what the agent should produce.
    pub description: String,
    /// Verbatim user intent for this task — the focused agent reads this.
    pub source_prompt: String,
    /// Target agent. Must be one of: "schema_designer".
    pub assigned_to_agent: String,
    /// Epic this task belongs to (null for standalone tasks).
    pub epic_id: Option<i64>,
    /// Task that must complete before this one can start.
    pub depends_on_task_id: Option<i64>,
}

/// Update the status of any task the PM is managing.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PmUpdateTaskStatusParams {
    /// ID of the task to update.
    pub task_id: i64,
    /// New status. Must be one of: "in_progress", "review", "done", "blocked".
    pub status: String,
}
