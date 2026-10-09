//! Owns applying migrations (`FileBasedMigrations::from_path`), backups, `inspect` and schema summaries.
//!
//! Must not write specs.

mod apply;
mod inspect;
mod sample;
mod summary;

pub use apply::{RuntimeError, apply_pending};
pub use inspect::{Inspection, inspect};
pub use sample::add_sample_rows;
pub use summary::summary;

use libsqlite3_sys as _;
