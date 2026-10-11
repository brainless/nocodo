//! Owns shared, versioned, serde types: events, questions, `SchemaSpec`, `SchemaChange`, project config.
//!
//! Must not do I/O, use an LLM, or contain business logic.

pub mod apply;
pub mod change;
pub mod event;
pub mod naming;
pub mod project;
pub mod question;
pub mod schema;
