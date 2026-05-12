use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListFilesParams {
    /// Directory path to list, relative to project root. Use "" for the project root.
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadFileParams {
    /// Path to the file to read, relative to the project root.
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UpdateTaskStatusParams {
    /// New task status. Must be one of: "in_progress", "done", "blocked".
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CommentaryParams {
    /// Optional assistant commentary text.
    pub text: Option<String>,
}
