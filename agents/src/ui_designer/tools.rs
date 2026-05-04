use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UpdateTaskStatusParams {
    /// New task status. Must be one of: "in_progress", "done", "blocked".
    pub status: String,
}
