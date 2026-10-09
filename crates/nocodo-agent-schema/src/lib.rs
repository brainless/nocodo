//! The Schema agent: turns a prompt into a typed `ChangeSet` plus assumptions.
//! It is the only source of schema changes outside developer commands, so it owns
//! `specs/schema.json` (the orchestrator writes that file on its behalf).
//!
//! Must not write SQL, touch `data.sqlite`, or write any other spec.

use nocodo_agent::{AgentContext, Result, run_extractor_with_id};
use nocodo_types::{change::ChangeSet, event::Finding, schema::SchemaSpec};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The role name used in `.agent-logs`.
pub const ROLE: &str = "schema";

/// The Schema agent's system prompt, kept in a file for easy review.
pub const PREAMBLE: &str = include_str!("preamble.md");

/// What the agent returns: the change set and every guess it made.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SchemaProposal {
    pub change_set: ChangeSet,
    /// Short plain sentences: what the agent would have asked the user.
    pub assumptions: Vec<String>,
}

/// Why a previous attempt was rejected, passed back verbatim on repair attempts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Feedback {
    pub previous: ChangeSet,
    pub findings: Vec<Finding>,
}

/// Builds the user message deterministically.
pub fn build_input(current: &SchemaSpec, prompt: &str, feedback: Option<&Feedback>) -> String {
    let schema = serde_json::to_string_pretty(current).unwrap_or_else(|_| "{}".to_string());
    let mut out = format!(
        "## Current schema\n\n```json\n{schema}\n```\n\n## User request\n\n{}\n",
        prompt.trim()
    );
    if let Some(fb) = feedback {
        let previous =
            serde_json::to_string_pretty(&fb.previous).unwrap_or_else(|_| "{}".to_string());
        let findings =
            serde_json::to_string_pretty(&fb.findings).unwrap_or_else(|_| "[]".to_string());
        out.push_str(&format!(
            "\n## Previous attempt (rejected)\n\n```json\n{previous}\n```\n\n## Checker findings\n\n```json\n{findings}\n```\n\nReturn a corrected change set that fixes every finding.\n"
        ));
    }
    out
}

pub struct SchemaAgent;

impl SchemaAgent {
    /// Runs one proposal against the current schema, optionally with feedback
    /// from a rejected earlier attempt.
    pub async fn propose(
        ctx: &AgentContext,
        current: &SchemaSpec,
        prompt: &str,
        feedback: Option<&Feedback>,
    ) -> Result<SchemaProposal> {
        Self::propose_with_run_id(ctx, current, prompt, feedback)
            .await
            .map(|(proposal, _)| proposal)
    }

    /// Like [`SchemaAgent::propose`], but also returns the run ID of the
    /// `.agent-logs` file for this call.
    pub async fn propose_with_run_id(
        ctx: &AgentContext,
        current: &SchemaSpec,
        prompt: &str,
        feedback: Option<&Feedback>,
    ) -> Result<(SchemaProposal, String)> {
        let input = build_input(current, prompt, feedback);
        run_extractor_with_id::<SchemaProposal>(ctx, PREAMBLE, &input).await
    }
}
