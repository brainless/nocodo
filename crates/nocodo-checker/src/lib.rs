//! Owns deterministic validation of specs and generated SQL (dry-run on a copy of the DB).
//!
//! Must not let an LLM decide pass or fail.
//!
//! Two layers, both returning `Vec<Finding>` with stable snake_case codes:
//! static rules on a change set ([`check_change_set`]) and a dry run of the
//! generated migration on a throwaway copy of the database ([`dry_run`]).

mod dry_run;
mod introspect;
mod keywords;
mod rules;

pub use dry_run::{CheckerError, NamedMigration, dry_run};
pub use rules::check_change_set;
