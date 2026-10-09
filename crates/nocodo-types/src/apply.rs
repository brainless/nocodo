//! Apply report: produced by the runtime, recorded by the orchestrator.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// A migration applied to `data.sqlite`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppliedMigration {
    pub migration: String,
    /// Backup path relative to the project (e.g. `backups/before_0002_x.sqlite`),
    /// or `None` when `data.sqlite` did not exist yet.
    pub backup: Option<String>,
    pub rows_before: BTreeMap<String, u64>,
    pub rows_after: BTreeMap<String, u64>,
}

/// A migration that failed; `data.sqlite` was restored when `restored` is true.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FailedMigration {
    pub migration: String,
    pub backup: Option<String>,
    /// The SQLite or I/O error, verbatim.
    pub error: String,
    pub restored: bool,
}

/// Result of an apply run. Migrations after a failure are not attempted.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApplyReport {
    pub applied: Vec<AppliedMigration>,
    pub failed: Option<FailedMigration>,
}
