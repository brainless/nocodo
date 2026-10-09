//! Owns the shared agent base: Rig wiring, per-role model config, `.agent-logs` JSONL logging, reading documents from allowed folders, and emitting `Question`s.
//!
//! Must not contain any role-specific prompt or logic.

mod error;
mod extract;
mod log;
mod model;

pub use error::{Error, Result};
pub use extract::{AgentContext, run_extractor, run_extractor_with_id};
pub use log::AgentLog;
pub use model::{BuiltModel, build_model};
