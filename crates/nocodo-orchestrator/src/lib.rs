//! Owns the project folder (`init`, `project.json`), the event log
//! (`events.jsonl`), and the pipeline that takes a proposed `ChangeSet`
//! through sqlgen and the checker, then writes `specs/schema.json` and the
//! migration. It writes artifacts only on behalf of their owner.
//!
//! Must not use an LLM. Routing is plain code.

mod error;
mod project;
mod questions;

pub use error::OrchestratorError;
pub use nocodo_checker::AnsweredQuestion;
pub use nocodo_types::outcome::{
    AnswerOutcome, ApplyOutcome, ContinueOutcome, Outcome, PromptOutcome,
};
pub use project::{
    Applier, ApplierError, Feedback, Project, Proposal, Proposer, ProposerError, ProposerFailure,
    Reply,
};
