//! Owns shared, versioned, serde types: events, questions, `SchemaSpec`, `SchemaChange`, project config.
//!
//! Must not do I/O, use an LLM, or contain business logic.

pub mod change;
pub mod event;
pub mod project;
pub mod schema;
