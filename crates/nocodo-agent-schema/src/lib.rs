//! The Schema agent: turns a prompt into questions for the user or a typed `ChangeSet`, plus assumptions.
//! It is the only source of schema changes outside developer commands, so it owns
//! `specs/schema.json` (the orchestrator writes that file on its behalf).
//!
//! Must not write SQL, touch `data.sqlite`, or write any other spec.

use nocodo_agent::{AgentContext, Result, run_extractor_with_id};
use nocodo_types::{
    change::ChangeSet,
    event::Finding,
    question::Question,
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

/// A question as the agent writes it: a [`nocodo_types::question::Question`]
/// without `id` and `asked_by`, which the orchestrator assigns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QuestionDraft {
    /// The question, in the user's own words.
    pub text: String,
    /// 2 to 5 short choices.
    pub choices: Vec<String>,
    /// Number (from 1) of the choice used when the user accepts the defaults.
    /// Null only for a blocking question.
    pub default: Option<usize>,
    /// True when no sensible default exists, so the user must answer.
    pub blocking: bool,
    /// For the reviewer, not shown to the user: what depends on the answer.
    pub why: String,
}

impl QuestionDraft {
    /// The full question once the orchestrator has assigned an ID.
    pub fn into_question(self, id: String, asked_by: &str) -> Question {
        Question {
            id,
            asked_by: asked_by.to_string(),
            text: self.text,
            choices: self.choices,
            default: self.default,
            blocking: self.blocking,
            why: self.why,
        }
    }
}

/// What the agent returns: either questions or a change set (exactly one of
/// `questions` non-empty and `change_set` set; the orchestrator rejects other
/// shapes), plus the minor guesses it made. One struct rather than a tagged
/// enum, so strict tool schemas keep working.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SchemaReply {
    /// Everything the agent needs to ask, in one round. Empty when it returns a change set.
    pub questions: Vec<QuestionDraft>,
    /// The proposed change set. Null when it asks questions.
    pub change_set: Option<ChangeSet>,
    /// Short plain sentences: minor guesses that were not worth a question.
    pub assumptions: Vec<String>,
}

/// An answered question, as shown to the agent in "Answers so far".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnswerSoFar {
    pub question: String,
    /// The text of the chosen option.
    pub chosen: String,
    /// True when the user accepted the default instead of choosing.
    pub by_default: bool,
}

/// Why a previous attempt was rejected, passed back verbatim on repair attempts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Feedback {
    /// The whole rejected reply, questions or change set.
    pub previous: SchemaReply,
    pub findings: Vec<Finding>,
}

/// Builds the user message deterministically.
pub fn build_input(
    current: &SchemaSpec,
    prompt: &str,
    answers: &[AnswerSoFar],
    feedback: Option<&Feedback>,
) -> String {
    let schema = serde_json::to_string_pretty(current).unwrap_or_else(|_| "{}".to_string());
    let mut out = format!(
        "## Current schema\n\n```json\n{schema}\n```\n\n## User request\n\n{}\n",
        prompt.trim()
    );
    if !answers.is_empty() {
        out.push_str(
            "\n## Answers so far\n\nThe user already answered these. Do not ask them again.\n\n",
        );
        for a in answers {
            let how = if a.by_default {
                "accepted the default"
            } else {
                "chosen"
            };
            out.push_str(&format!(
                "- Q: {}\n  A: {} ({how})\n",
                a.question.trim(),
                a.chosen.trim()
            ));
        }
    }
    if let Some(fb) = feedback {
        let previous =
            serde_json::to_string_pretty(&fb.previous).unwrap_or_else(|_| "{}".to_string());
        let findings =
            serde_json::to_string_pretty(&fb.findings).unwrap_or_else(|_| "[]".to_string());
        out.push_str(&format!(
            "\n## Previous attempt (rejected)\n\n```json\n{previous}\n```\n\n## Checker findings\n\n```json\n{findings}\n```\n\nReturn a corrected reply that fixes every finding.\n"
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
        answers: &[AnswerSoFar],
        feedback: Option<&Feedback>,
    ) -> Result<SchemaReply> {
        Self::propose_with_run_id(ctx, current, prompt, answers, feedback)
            .await
            .map(|(proposal, _)| proposal)
    }

    /// Like [`SchemaAgent::propose`], but also returns the run ID of the
    /// `.agent-logs` file for this call.
    pub async fn propose_with_run_id(
        ctx: &AgentContext,
        current: &SchemaSpec,
        prompt: &str,
        answers: &[AnswerSoFar],
        feedback: Option<&Feedback>,
    ) -> Result<(SchemaReply, String)> {
        let input = build_input(current, prompt, answers, feedback);
        run_extractor_with_id::<SchemaReply>(ctx, &preamble(), &input).await
    }
}
