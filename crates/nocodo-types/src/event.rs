//! Events: the append-only log (`events.jsonl`).
//!
//! `seq` starts at 1 and increases by one per event. `proposal_seq` points at
//! the `ChangeSetProposed` event.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::change::ChangeSet;
use crate::question::{Answer, Question};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub seq: u64,
    pub at: jiff::Timestamp,
    pub actor: Actor,
    pub kind: EventKind,
}

/// Who caused an event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Actor {
    Developer,
    System,
    User,
    SchemaAgent {
        run_id: String,
        /// `provider:model` that proposed the change set.
        #[serde(default)]
        model: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventKind {
    ProjectCreated {
        spec_version: u32,
        name: String,
    },
    UserPrompt {
        text: String,
    },
    /// The model call for a prompt failed and nothing was proposed (actor `System`).
    /// The prompt stays in the log; `nocodo-api continue` repeats the call.
    PromptFailed {
        /// Seq of the `UserPrompt` event whose run failed.
        prompt_seq: u64,
        /// The error, verbatim.
        error: String,
        /// The `.agent-logs/<run_id>.jsonl` file of the failed run, when a run started.
        #[serde(default)]
        run_id: Option<String>,
        detail: String,
    },
    ChangeSetProposed {
        change_set: ChangeSet,
        /// Seq of the `UserPrompt` event that triggered this proposal.
        #[serde(default)]
        prompt_seq: Option<u64>,
        /// 1-based attempt number within a prompt run.
        #[serde(default)]
        attempt: u32,
        /// Guesses the proposer made, as plain sentences.
        #[serde(default)]
        assumptions: Vec<String>,
    },
    ChangeSetRejected {
        proposal_seq: u64,
        findings: Vec<Finding>,
        /// Check steps run, up to and including the one that failed.
        #[serde(default)]
        checks: Vec<CheckStep>,
    },
    ChangeSetAccepted {
        proposal_seq: u64,
        /// e.g. "0002_add_family_member"
        migration: String,
        /// Every check step that ran, in order (all passed).
        #[serde(default)]
        checks: Vec<CheckStep>,
    },
    /// A round of questions waiting for the user. Actor `SchemaAgent` (points at a
    /// prompt) or `System` (destructive confirmation, points at a proposal).
    QuestionsAsked {
        #[serde(default)]
        prompt_seq: Option<u64>,
        #[serde(default)]
        proposal_seq: Option<u64>,
        questions: Vec<Question>,
        detail: String,
    },
    /// One answer from the user (actor `User`).
    QuestionAnswered {
        answer: Answer,
        detail: String,
    },
    /// A round that failed the question checks and was not shown to the user.
    QuestionsRejected {
        prompt_seq: u64,
        findings: Vec<Finding>,
    },
    /// Duplicate questions removed by the presenter.
    QuestionsDropped {
        question_ids: Vec<String>,
        detail: String,
    },
    /// A migration was applied to `data.sqlite` (actor `System`).
    MigrationApplied {
        migration: String,
        /// Backup path relative to the project; `None` when `data.sqlite` did not exist yet.
        backup: Option<String>,
        rows_before: BTreeMap<String, u64>,
        rows_after: BTreeMap<String, u64>,
        detail: String,
    },
    /// A migration failed and `data.sqlite` was put back (actor `System`).
    MigrationFailed {
        migration: String,
        backup: Option<String>,
        /// The SQLite or I/O error, verbatim.
        error: String,
        restored: bool,
        detail: String,
    },
    /// The comparison of `data.sqlite` with the spec after an apply run (actor `System`).
    DatabaseChecked {
        check: CheckStep,
    },
    /// Fake rows added for testing (actor `Developer`).
    SampleRowsAdded {
        rows: BTreeMap<String, u64>,
        detail: String,
    },
}

/// A checker result (produced in epic 003).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    pub code: String,
    pub message: String,
}

/// One step the checker ran on a proposal.
///
/// `check` is a stable snake_case code: `static_rules`,
/// `earlier_migrations_applied`, `db_matches_spec_before`, `up`,
/// `db_matches_spec_after`, `foreign_key_check`, `down`,
/// `db_matches_spec_after_down`, `up_again`, `db_matches_spec_after_up_again`,
/// `rows_kept` (dry run: existing rows survive `up`) and
/// `db_matches_spec_after_apply` (real `data.sqlite` after an apply run).
/// `detail` is a plain sentence a reviewer can read alone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckStep {
    pub check: String,
    pub passed: bool,
    pub detail: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_events_round_trip() {
        let kinds = vec![
            EventKind::MigrationApplied {
                migration: "0002_x".into(),
                backup: Some("backups/before_0002_x.sqlite".into()),
                rows_before: BTreeMap::from([("a".into(), 1)]),
                rows_after: BTreeMap::from([("a".into(), 1)]),
                detail: "d".into(),
            },
            EventKind::MigrationFailed {
                migration: "0003_y".into(),
                backup: None,
                error: "boom".into(),
                restored: true,
                detail: "d".into(),
            },
            EventKind::DatabaseChecked {
                check: CheckStep {
                    check: "db_matches_spec_after_apply".into(),
                    passed: true,
                    detail: "ok".into(),
                },
            },
            EventKind::SampleRowsAdded {
                rows: BTreeMap::from([("a".into(), 3)]),
                detail: "d".into(),
            },
            EventKind::PromptFailed {
                prompt_seq: 2,
                error: "429".into(),
                run_id: Some("r1".into()),
                detail: "d".into(),
            },
            EventKind::QuestionsAsked {
                prompt_seq: Some(2),
                proposal_seq: None,
                questions: vec![Question {
                    id: "q1".into(),
                    asked_by: "schema".into(),
                    text: "t".into(),
                    choices: vec!["a".into(), "b".into()],
                    default: Some(1),
                    blocking: false,
                    why: "w".into(),
                }],
                detail: "d".into(),
            },
            EventKind::QuestionAnswered {
                answer: Answer {
                    question_id: "q1".into(),
                    choice: 2,
                    by_default: false,
                },
                detail: "d".into(),
            },
            EventKind::QuestionsRejected {
                prompt_seq: 2,
                findings: vec![Finding {
                    code: "technical_word".into(),
                    message: "m".into(),
                }],
            },
            EventKind::QuestionsDropped {
                question_ids: vec!["q2".into()],
                detail: "d".into(),
            },
        ];
        for kind in kinds {
            let json = serde_json::to_string(&kind).unwrap();
            assert_eq!(serde_json::from_str::<EventKind>(&json).unwrap(), kind);
        }
    }

    #[test]
    fn old_logs_load() {
        let line = r#"{"seq":1,"at":"2026-10-01T00:00:00Z","actor":{"type":"user"},"kind":{"type":"user_prompt","text":"hi"}}"#;
        let e: Event = serde_json::from_str(line).unwrap();
        assert_eq!(e.kind, EventKind::UserPrompt { text: "hi".into() });
    }
}
