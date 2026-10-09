//! Owns the project folder (`init`, `project.json`), the event log
//! (`events.jsonl`), and the pipeline that takes a proposed `ChangeSet`
//! through sqlgen and the checker, then writes `specs/schema.json` and the
//! migration. It writes artifacts only on behalf of their owner.
//!
//! Must not use an LLM. Routing is plain code.

mod error;
mod project;

pub use error::OrchestratorError;
pub use project::{Outcome, Project};
