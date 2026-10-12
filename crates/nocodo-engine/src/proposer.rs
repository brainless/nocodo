//! The `Applier` and `Proposer` implementations: the runtime and the Schema
//! agent behind the orchestrator's traits.

use nocodo_agent::AgentContext;
use nocodo_agent_schema::{AnswerSoFar, QuestionDraft, SchemaAgent, SchemaReply};
use nocodo_orchestrator::{
    Applier, ApplierError, Project, Proposal, Proposer, ProposerError, ProposerFailure, Reply,
};
use nocodo_types::event::Actor;
use nocodo_types::question::Question;
use nocodo_types::schema::SchemaSpec;

/// Adapts the runtime to the orchestrator's `Applier` trait.
pub(crate) struct RuntimeApplier;

impl Applier for RuntimeApplier {
    fn apply_pending(
        &self,
        project_dir: &std::path::Path,
    ) -> Result<nocodo_types::apply::ApplyReport, ApplierError> {
        Ok(nocodo_runtime::apply_pending(project_dir)?)
    }
}

/// Adapts the Schema agent to the orchestrator's `Proposer` trait.
pub struct SchemaProposer {
    ctx: AgentContext,
}

impl Proposer for SchemaProposer {
    async fn propose(
        &self,
        current: &SchemaSpec,
        prompt: &str,
        answers: &[nocodo_orchestrator::AnsweredQuestion],
        feedback: Option<&nocodo_orchestrator::Feedback>,
    ) -> Result<Proposal, ProposerError> {
        let answers: Vec<AnswerSoFar> = answers
            .iter()
            .map(|a| AnswerSoFar {
                question: a.question.text.clone(),
                chosen: a.question.choices[a.answer.choice - 1].clone(),
                by_default: a.answer.by_default,
            })
            .collect();
        let feedback = feedback.map(|f| nocodo_agent_schema::Feedback {
            previous: SchemaReply {
                questions: f.previous.questions.iter().map(to_draft).collect(),
                change_set: f.previous.change_set.clone(),
                assumptions: f.previous.assumptions.clone(),
            },
            findings: f.findings.clone(),
        });
        let (reply, run_id) = SchemaAgent::propose_with_run_id(
            &self.ctx,
            current,
            prompt,
            &answers,
            feedback.as_ref(),
        )
        .await
        .map_err(failure)?;
        Ok(Proposal {
            reply: Reply {
                // The orchestrator assigns the IDs and the asker.
                questions: reply
                    .questions
                    .into_iter()
                    .map(|d| d.into_question(String::new(), "schema"))
                    .collect(),
                change_set: reply.change_set,
                assumptions: reply.assumptions,
            },
            actor: Actor::SchemaAgent {
                run_id,
                model: format!("{}:{}", self.ctx.model.provider, self.ctx.model.model),
            },
        })
    }
}

fn to_draft(q: &Question) -> QuestionDraft {
    QuestionDraft {
        text: q.text.clone(),
        choices: q.choices.clone(),
        default: q.default,
        blocking: q.blocking,
        why: q.why.clone(),
    }
}

/// Hands the orchestrator the failed run's ID and retry count.
fn failure(error: nocodo_agent::Error) -> ProposerError {
    match error {
        nocodo_agent::Error::Extraction {
            message,
            run_id,
            provider_retries,
        } => Box::new(ProposerFailure {
            message,
            run_id: Some(run_id),
            provider_retries,
        }),
        other => Box::new(other),
    }
}

pub fn schema_proposer(project: &Project) -> SchemaProposer {
    SchemaProposer {
        ctx: AgentContext::new(
            project.dir(),
            nocodo_agent_schema::ROLE,
            project.config().models.schema.clone(),
        ),
    }
}
