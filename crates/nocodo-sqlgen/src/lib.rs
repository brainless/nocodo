//! Owns pure functions: apply `SchemaChange` to `SchemaSpec`, diff specs, generate `up.sql`/`down.sql` (including SQLite table rebuilds).
//!
//! Must not do I/O or use an LLM. This is the determinism core.

use nocodo_types as _;
