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
    /// Target agent. Must be one of: "db_engineer".
    pub assigned_to_agent: String,
    /// Epic this task belongs to (null for standalone tasks).
    pub epic_id: Option<i64>,
    /// Task that must complete before this one can start.
    pub depends_on_task_id: Option<i64>,
}

/// Set the human-readable name of the current project.
/// Use this during project init, after understanding the user's domain.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SetProjectNameParams {
    /// A concise, descriptive name for the project (≤ 60 chars). Derived from the user's domain,
    /// e.g. "CRM — Leads & Deals" or "Inventory Tracker".
    pub name: String,
}

/// Update the status of any task the PM is managing.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PmUpdateTaskStatusParams {
    /// ID of the task to update.
    pub task_id: i64,
    /// New status. Must be one of: "draft", "in_progress", "done", "blocked".
    pub status: String,
}

/// A single task definition within a finalize_session call.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FinalizeTaskDef {
    /// Short title for the task.
    pub title: String,
    /// Description of what the assigned agent should produce.
    pub description: String,
    /// Target agent type string, e.g. "db_engineer".
    pub assigned_to_agent: String,
}

/// Called by PM to atomically finalize a user chat session: emit a closing
/// message, create one epic, and create one or more tasks.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FinalizeSessionParams {
    /// PM's closing message to the user.
    pub final_message: String,
    /// Title for the epic.
    pub epic_title: String,
    /// Description for the epic.
    pub epic_description: String,
    /// Tasks to create under this epic.
    pub tasks: Vec<FinalizeTaskDef>,
}
