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

/// Who caused an event. `SchemaAgent { run_id }` and `User` come in epic 004.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Actor {
    Developer,
    System,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventKind {
    ProjectCreated {
        spec_version: u32,
        name: String,
    },
    ChangeSetProposed {
        change_set: ChangeSet,
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
