//! Events: the append-only log (`events.jsonl`).
//!
//! `seq` starts at 1 and increases by one per event. `proposal_seq` points at
//! the `ChangeSetProposed` event.

use serde::{Deserialize, Serialize};

use crate::change::ChangeSet;

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
    SchemaAgent { run_id: String },
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
    },
    ChangeSetAccepted {
        proposal_seq: u64,
        /// e.g. "0002_add_family_member"
        migration: String,
    },
}

/// A checker result (produced in epic 003).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    pub code: String,
    pub message: String,
}
