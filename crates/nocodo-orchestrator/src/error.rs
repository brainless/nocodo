use std::io;
use std::path::PathBuf;

use nocodo_sqlgen::ApplyError;

#[derive(Debug)]
pub enum OrchestratorError {
    Io(io::Error),
    Json(serde_json::Error),
    Checker(nocodo_checker::CheckerError),
    Apply(ApplyError),
    /// `init` target exists and is not empty.
    DirectoryNotEmpty(PathBuf),
    /// `open` target has no `project.json`.
    NotAProject(PathBuf),
    /// `specs/schema.json` differs from what `events.jsonl` produces.
    SchemaEditedByHand,
    /// `events.jsonl` is inconsistent (bad line, missing proposal, ...).
    CorruptEventLog(String),
    /// The proposer (e.g. the Schema agent) failed.
    Proposer(Box<dyn std::error::Error + Send + Sync>),
}

impl std::fmt::Display for OrchestratorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "file error: {e}"),
            Self::Json(e) => write!(f, "invalid JSON: {e}"),
            Self::Checker(e) => write!(f, "{e}"),
            Self::Apply(e) => write!(f, "could not apply the accepted changes: {e}"),
            Self::DirectoryNotEmpty(p) => write!(
                f,
                "{} already exists and is not empty; choose a new folder",
                p.display()
            ),
            Self::NotAProject(p) => write!(
                f,
                "{} is not a nocodo project (no project.json); run init first",
                p.display()
            ),
            Self::SchemaEditedByHand => write!(
                f,
                "specs/schema.json does not match the accepted change sets in events.jsonl; \
                 it was edited by hand. Restore it, or propose a change set instead"
            ),
            Self::CorruptEventLog(m) => write!(f, "events.jsonl is inconsistent: {m}"),
            Self::Proposer(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for OrchestratorError {}

impl From<io::Error> for OrchestratorError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<serde_json::Error> for OrchestratorError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}
impl From<nocodo_checker::CheckerError> for OrchestratorError {
    fn from(e: nocodo_checker::CheckerError) -> Self {
        Self::Checker(e)
    }
}
impl From<ApplyError> for OrchestratorError {
    fn from(e: ApplyError) -> Self {
        Self::Apply(e)
    }
}
