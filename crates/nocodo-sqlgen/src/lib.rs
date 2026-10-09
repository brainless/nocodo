//! Owns pure functions: apply `SchemaChange`s to a `SchemaSpec`, and generate `up.sql`/`down.sql` from a spec plus a change list (including SQLite table rebuilds).
//!
//! Must not do I/O or use an LLM. This is the determinism core.
//!
//! Identifiers are emitted unquoted. Names must be validated first by the
//! checker (`^[a-z][a-z0-9_]{0,62}$`, no SQLite keywords).

mod apply;
mod generate;
mod naming;

pub use apply::{ApplyError, apply, apply_one};
pub use generate::{METADATA_TOML, Migration, generate};
pub use naming::{IndexDef, column_sql_type, index_name, table_indexes};
