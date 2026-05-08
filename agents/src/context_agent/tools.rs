use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListFilesParams {
    /// Absolute or relative directory path to list. Use "" for the project root.
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadFileParams {
    /// Path to the file to read, relative to the project root.
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WriteContextParams {
    /// The gathered context as a structured summary of the codebase.
    pub context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UpdateTaskStatusParams {
    /// New task status. Must be one of: "in_progress", "done", "blocked".
    pub status: String,
}