//! # nocodo_praxis
//!
//! A provenance-aware vocabulary crate that encodes the structural primitives of
//! business logic as Rust types. The spec *is* the runtime — no separate IR,
//! no sync problem.
//!
//! ## Module composition
//!
//! Modules layer from foundational to domain-specific:
//!
//! | Module | Purpose |
//! |---|---|
//! | [`primitives`] | `AtLeastOne<T>`, `Unresolved<T>` — structural building blocks used by all other modules |
//! | [`provenance`] | `Provenance`, `PrdValue<T>` — where every requirement came from |
//! | [`auth`] | Roles, permissions, user personas — identity and access |
//! | [`statemachine`] | States, transitions, conditions — lifecycle modeling |
//! | [`entity`] | Entities, fields, invariants — domain objects |
//!
//! ## Design principles
//!
//! 1. **Encode business constraints as types, not conventions.** An action with
//!    no permitted role is structurally unrepresentable (`AtLeastOne`).
//! 2. **Incompleteness is first-class.** `Unresolved::Pending` blocks codegen
//!    the way a compile error blocks a build.
//! 3. **Inference is visible.** `Provenance::Inferred` marks every LLM assumption.
//! 4. **Spec is runtime.** Types used in spec definitions are the same types
//!    used by permission checkers, state machine enforcement, and middleware.
//! 5. **Grow from evidence.** Every type in this crate was motivated by a real
//!    project that needed it.

pub mod auth;
pub mod entity;
pub mod primitives;
pub mod provenance;
pub mod statemachine;
