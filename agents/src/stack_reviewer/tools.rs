use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EmitNoteParams {
    /// Tag category. Must be one of: backend, database, frontend, auth, api_contract, config, tooling, deployment, testing
    pub tag: String,
    /// Short key point (under 120 characters).
    pub note: String,
    /// Relative file path where this note applies (optional).
    pub file_path: Option<String>,
    /// Line number within file_path (optional).
    pub line_number: Option<i64>,
    /// Exact text of an existing current note that this one supersedes. Omit for brand-new facts.
    pub replaces_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FinishReviewParams {
    /// Brief summary of changes made: new notes added, stale notes replaced.
    pub summary: String,
}
