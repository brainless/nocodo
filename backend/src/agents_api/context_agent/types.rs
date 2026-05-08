use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct GatherContextRequest {
    pub project_id: i64,
    pub context_type: String,
}

#[derive(Debug, Serialize)]
pub struct ContextResponse {
    pub context_type: String,
    pub context: String,
}

#[derive(Debug, Serialize)]
pub struct GatherContextQueued {
    pub task_id: i64,
}