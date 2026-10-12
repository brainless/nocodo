//! Typed results of the operations. Shared by the orchestrator, the engine and
//! every front end, so none of them prints or words anything the others own.

use std::collections::BTreeMap;

use crate::apply::ApplyReport;
use crate::change::ChangeSet;
use crate::event::{CheckStep, Finding};
use crate::question::Question;

/// Result of `Project::propose`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Migration folder name, e.g. `0002_add_family_member`.
    Accepted {
        migration: String,
        /// Every check step that ran, in order.
        checks: Vec<CheckStep>,
    },
    Rejected {
        findings: Vec<Finding>,
        /// Check steps up to and including the one that failed.
        checks: Vec<CheckStep>,
    },
    /// The change set removes data and passed every other check; the user's
    /// confirmation is waiting. Nothing was written to `migrations/` or `specs/`.
    Waiting { questions: Vec<Question> },
}

/// Result of `Project::run_prompt` and of resuming one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptOutcome {
    /// Questions are waiting: the agent's round, or the removal confirmation.
    Waiting { questions: Vec<Question> },
    /// The last attempt. `outcome` is never `Outcome::Waiting`.
    Done {
        outcome: Outcome,
        /// The last change set proposed; `None` when no valid change set came back.
        change_set: Option<ChangeSet>,
        assumptions: Vec<String>,
        attempts: u32,
    },
}

/// Result of `Project::answer`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnswerOutcome {
    /// Some questions are still open; the model was not called.
    StillWaiting { open: Vec<Question> },
    /// Every question is answered; call `Project::continue_pending`.
    ReadyToContinue,
}

/// Result of `Project::continue_pending`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContinueOutcome {
    NothingToContinue,
    /// The round is not fully answered yet.
    StillWaiting {
        open: Vec<Question>,
    },
    Continued(PromptOutcome),
}

/// Result of `Project::apply_pending`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyOutcome {
    pub report: ApplyReport,
    /// The post-apply comparison; `None` when nothing was applied or a migration failed.
    pub check: Option<CheckStep>,
    /// The `detail` of each `MigrationApplied` event, then of `MigrationFailed` if any.
    pub details: Vec<String>,
}

/// Result of the engine's `init`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitOutcome {
    pub name: String,
    /// Where the model settings came from (a file path or the built-in defaults).
    pub source: String,
    /// The Schema agent's model, as written in `project.json`.
    pub schema_model: String,
}

/// Result of the engine's `changes`: the verdict, and the apply that followed
/// when the change set was accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangesRun {
    pub outcome: Outcome,
    pub applied: Option<ApplyOutcome>,
}

/// Result of the engine's `prompt`, and of resuming one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptRun {
    pub outcome: PromptOutcome,
    /// Set when the change set was accepted and applied.
    pub applied: Option<ApplyOutcome>,
}

/// Result of the engine's `continue`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContinueRun {
    NothingToContinue,
    /// The round is not fully answered yet.
    StillWaiting {
        open: Vec<Question>,
    },
    Continued(Box<PromptRun>),
}

/// Result of the engine's `answer`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnswerRun {
    /// Some questions are still open; the model was not called.
    StillWaiting { open: Vec<Question> },
    /// Every question is answered; the run was resumed.
    Resumed(Box<ContinueRun>),
}

/// Result of the engine's `inspect`: what is stored, for the reviewer, and the
/// plain-language summary, for the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectOutcome {
    /// Row count per table (empty without a `data.sqlite`).
    pub tables: BTreeMap<String, u64>,
    pub applied: Vec<String>,
    pub pending: Vec<String>,
    pub backups: usize,
    pub summary: String,
}

/// Result of the engine's `sample_rows`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SampleRowsOutcome {
    /// Rows added per table.
    pub rows: BTreeMap<String, u64>,
    /// The sentence recorded in the event log.
    pub detail: String,
}

/// One project found under a projects folder (engine `list_projects`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ProjectInfo {
    /// The name in `project.json`; the folder name when that file is broken.
    pub name: String,
    pub path: String,
    /// Number of tables in the schema (none when the project is broken).
    pub tables: Option<usize>,
    /// When the latest event was recorded.
    pub last_event: Option<jiff::Timestamp>,
    /// Plain-language reason the project could not be read, if it could not.
    pub error: Option<String>,
}

/// One stored value, typed by its column.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum CellValue {
    Null,
    Bool(bool),
    Integer(i64),
    Real(f64),
    Text(String),
}

/// A column of a row page.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RowColumn {
    /// Column name as stored.
    pub name: String,
    /// The name in plain words, for headings.
    pub label: String,
    pub column_type: crate::schema::ColumnType,
}

/// One page of a table's rows (runtime `read_rows`).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RowPage {
    pub table: String,
    /// The implicit columns first, then the schema's, in table order.
    pub columns: Vec<RowColumn>,
    /// Each row has one value per column, ordered by `id`.
    pub rows: Vec<Vec<CellValue>>,
    /// Rows in the whole table.
    pub total: u64,
    pub offset: u64,
    pub limit: u64,
}
