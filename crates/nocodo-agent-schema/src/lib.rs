//! The Schema agent: turns a prompt into a typed `ChangeSet` plus assumptions.
//! It is the only source of schema changes outside developer commands, so it owns
//! `specs/schema.json` (the orchestrator writes that file on its behalf).
//!
//! Must not write SQL, touch `data.sqlite`, or write any other spec.

use nocodo_agent::{AgentContext, Result, run_extractor_with_id};
use nocodo_types::{
    change::ChangeSet,
    event::Finding,
    schema::{IMPLICIT_COLUMNS, SchemaSpec},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The role name used in `.agent-logs`.
pub const ROLE: &str = "schema";

/// The preamble template, kept in a file for easy review. It contains the
/// placeholder `{{implicit_columns}}`, filled by [`preamble`].
const PREAMBLE_TEMPLATE: &str = include_str!("preamble.md");

const IMPLICIT_COLUMNS_PLACEHOLDER: &str = "{{implicit_columns}}";

/// The section about the columns every table gets, rendered from
/// `IMPLICIT_COLUMNS` in `nocodo-types`.
fn implicit_columns_section() -> String {
    let mut out = String::from(
        "## Columns every table gets\n\nEvery table automatically has these columns. Never list them yourself, in `create_table` or anywhere else:\n\n",
    );
    for c in IMPLICIT_COLUMNS {
        let ty = serde_json::to_value(c.column_type)
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_default();
        out.push_str(&format!("- `{}` (`{ty}`): {}\n", c.name, c.description));
    }
    out.push_str(
        "\nA column named `<x>_id` that sets `references` to table `<x>` points at that table's `id`. `created_at` already records when an entry was added, so add a date column only for a real-world date (such as `paid_on`).",
    );
    out
}

/// The Schema agent's system prompt, with the generated sections filled in.
/// Deterministic: the same build always gives the same text.
pub fn preamble() -> String {
    PREAMBLE_TEMPLATE.replace(IMPLICIT_COLUMNS_PLACEHOLDER, &implicit_columns_section())
}

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
        run_extractor_with_id::<SchemaProposal>(ctx, &preamble(), &input).await
    }
}
