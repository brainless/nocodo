use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use nocodo_checker::{
    AnsweredQuestion, NamedMigration, check_change_set, check_db_matches_spec, check_questions,
    dry_run, static_rules_step, table_renames,
};
use nocodo_sqlgen::{METADATA_TOML, apply, generate};
use nocodo_types::apply::{AppliedMigration, ApplyReport, FailedMigration};
use nocodo_types::change::ChangeSet;
use nocodo_types::event::{Actor, CheckStep, Event, EventKind, Finding};
use nocodo_types::project::{ProjectBase, ProjectConfig};
use nocodo_types::question::{Answer, Question};
use nocodo_types::schema::{SPEC_VERSION, SchemaSpec};

use crate::error::OrchestratorError;
use crate::questions::{self, SCHEMA, SYSTEM};

type Result<T> = std::result::Result<T, OrchestratorError>;

/// Result of [`Project::propose`].
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

/// A proposer's whole reply: exactly one of `questions` (non-empty) and
/// `change_set` must be set; the orchestrator rejects other shapes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    /// Questions as the proposer wrote them; `id` and `asked_by` are
    /// overwritten by the orchestrator.
    pub questions: Vec<Question>,
    pub change_set: Option<ChangeSet>,
    /// Guesses the proposer made, as plain sentences.
    pub assumptions: Vec<String>,
}

impl Reply {
    pub fn change_set(change_set: ChangeSet, assumptions: Vec<String>) -> Reply {
        Reply {
            questions: Vec::new(),
            change_set: Some(change_set),
            assumptions,
        }
    }
}

/// What a [`Proposer`] returns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proposal {
    pub reply: Reply,
    /// Who made the proposal (recorded on the events it leads to).
    pub actor: Actor,
}

/// Why the previous attempt was rejected, passed back to the proposer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Feedback {
    /// The whole rejected reply, questions or change set.
    pub previous: Reply,
    pub findings: Vec<Finding>,
}

pub type ProposerError = Box<dyn std::error::Error + Send + Sync>;

/// A proposer error that knows which agent run failed. A [`Proposer`] may
/// return it (boxed) so the `prompt_failed` event can name the run and the
/// retries; any other error is recorded with its message only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposerFailure {
    pub message: String,
    /// The `.agent-logs/<run_id>.jsonl` file of the failed run.
    pub run_id: Option<String>,
    /// Waits spent retrying errors the provider marked retryable.
    pub provider_retries: u64,
}

impl std::fmt::Display for ProposerFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ProposerFailure {}

fn prompt_failed_detail(
    prompt_seq: u64,
    error: &ProposerError,
) -> (String, Option<String>, String) {
    let (message, run_id, retries) = match error.downcast_ref::<ProposerFailure>() {
        Some(f) => (f.message.clone(), f.run_id.clone(), f.provider_retries),
        None => (error.to_string(), None, 0),
    };
    let retried = match retries {
        0 => String::new(),
        n => format!(" (retried {n} times)"),
    };
    let log = match &run_id {
        Some(id) => format!("see .agent-logs/{id}.jsonl"),
        None => "no model run was started".to_string(),
    };
    let detail = format!(
        "The model call for prompt {prompt_seq} failed: {message}{retried}. Nothing was proposed; \
         {log}. Run `nocodo-api continue` to try again."
    );
    (message, run_id, detail)
}

/// Source of change sets for [`Project::run_prompt`]. Implemented outside the
/// orchestrator (the api adapts the Schema agent), so no agent dependency.
pub trait Proposer {
    fn propose(
        &self,
        current: &SchemaSpec,
        prompt: &str,
        answers: &[AnsweredQuestion],
        feedback: Option<&Feedback>,
    ) -> impl std::future::Future<Output = std::result::Result<Proposal, ProposerError>> + Send;
}

/// Result of [`Project::run_prompt`] and of resuming one.
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

/// Result of [`Project::answer`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnswerOutcome {
    /// Some questions are still open; the model was not called.
    StillWaiting { open: Vec<Question> },
    /// Every question is answered; call [`Project::continue_pending`].
    ReadyToContinue,
}

/// Result of [`Project::continue_pending`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContinueOutcome {
    NothingToContinue,
    /// The round is not fully answered yet.
    StillWaiting {
        open: Vec<Question>,
    },
    Continued(PromptOutcome),
}

pub type ApplierError = Box<dyn std::error::Error + Send + Sync>;

/// Applies pending migrations to `data.sqlite`. Implemented outside the
/// orchestrator (the api adapts the runtime), so no runtime dependency.
pub trait Applier {
    fn apply_pending(&self, project_dir: &Path) -> std::result::Result<ApplyReport, ApplierError>;
}

/// Result of [`Project::apply_pending`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyOutcome {
    pub report: ApplyReport,
    /// The post-apply comparison; `None` when nothing was applied or a migration failed.
    pub check: Option<CheckStep>,
    /// The `detail` of each `MigrationApplied` event, then of `MigrationFailed` if any.
    pub details: Vec<String>,
}

fn rows_summary(before: &BTreeMap<String, u64>, after: &BTreeMap<String, u64>) -> String {
    if before.is_empty() {
        return "There were no existing lists yet.".to_string();
    }
    let b: u64 = before.values().sum();
    let a: u64 = before.keys().filter_map(|k| after.get(k)).sum();
    format!("Rows in existing lists: {b} before, {a} after.")
}

fn applied_detail(m: &AppliedMigration) -> String {
    let how = match &m.backup {
        Some(b) => format!("after backing it up to {b}"),
        None => "as a new file (there was nothing to back up)".to_string(),
    };
    format!(
        "Applied {} to data.sqlite {how}. {}",
        m.migration,
        rows_summary(&m.rows_before, &m.rows_after)
    )
}

fn failed_detail(f: &FailedMigration) -> String {
    let state = match (f.restored, &f.backup) {
        (true, Some(b)) => format!(
            "data.sqlite was restored from {b} and is exactly as it was before."
        ),
        (true, None) => "The new data.sqlite was removed, so it is exactly as it was before (it did not exist).".to_string(),
        (false, _) => "data.sqlite could NOT be restored automatically; check it against the backup before doing anything else.".to_string(),
    };
    format!(
        "{} failed: {}. {state} Later migrations were not attempted; `nocodo-api apply` retries.",
        f.migration, f.error
    )
}

#[derive(Debug, Clone)]
pub struct Project {
    dir: PathBuf,
    config: ProjectConfig,
}

fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    let mut text = serde_json::to_string_pretty(value)?;
    text.push('\n');
    fs::write(path, text)?;
    Ok(())
}

fn empty_spec() -> SchemaSpec {
    SchemaSpec {
        spec_version: SPEC_VERSION,
        tables: vec![],
    }
}

impl Project {
    /// Create a new project folder with the built-in default settings. Fails
    /// if `dir` exists and is not empty.
    pub fn init(dir: impl AsRef<Path>, name: &str) -> Result<Project> {
        Self::init_with_base(dir, name, &ProjectBase::default())
    }

    /// Like [`Project::init`], with settings copied from a `project-base.json`.
    pub fn init_with_base(
        dir: impl AsRef<Path>,
        name: &str,
        base: &ProjectBase,
    ) -> Result<Project> {
        let dir = dir.as_ref().to_path_buf();
        if dir.exists() && fs::read_dir(&dir)?.next().is_some() {
            return Err(OrchestratorError::DirectoryNotEmpty(dir));
        }
        fs::create_dir_all(dir.join("specs"))?;
        fs::create_dir_all(dir.join("migrations"))?;
        fs::create_dir_all(dir.join("inputs"))?;
        let config = ProjectConfig {
            spec_version: SPEC_VERSION,
            name: name.to_string(),
            models: base.models.clone(),
        };
        write_json(&dir.join("project.json"), &config)?;
        write_json(&dir.join("specs/schema.json"), &empty_spec())?;
        fs::write(dir.join("events.jsonl"), "")?;
        let project = Project { dir, config };
        project.append(
            Actor::System,
            EventKind::ProjectCreated {
                spec_version: SPEC_VERSION,
                name: name.to_string(),
            },
        )?;
        Ok(project)
    }

    pub fn open(dir: impl AsRef<Path>) -> Result<Project> {
        let dir = dir.as_ref().to_path_buf();
        let path = dir.join("project.json");
        if !path.is_file() {
            return Err(OrchestratorError::NotAProject(dir));
        }
        let config = serde_json::from_str(&fs::read_to_string(path)?)?;
        Ok(Project { dir, config })
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn config(&self) -> &ProjectConfig {
        &self.config
    }

    fn events_path(&self) -> PathBuf {
        self.dir.join("events.jsonl")
    }

    fn schema_path(&self) -> PathBuf {
        self.dir.join("specs/schema.json")
    }

    fn migrations_dir(&self) -> PathBuf {
        self.dir.join("migrations")
    }

    /// All events, in order.
    pub fn events(&self) -> Result<Vec<Event>> {
        let text = fs::read_to_string(self.events_path())?;
        text.lines()
            .filter(|l| !l.trim().is_empty())
            .map(|l| serde_json::from_str(l).map_err(Into::into))
            .collect()
    }

    /// Append one event, assigning `seq` and `at`.
    pub fn append(&self, actor: Actor, kind: EventKind) -> Result<Event> {
        let seq = self.events()?.last().map_or(0, |e| e.seq) + 1;
        let event = Event {
            seq,
            at: jiff::Timestamp::now(),
            actor,
            kind,
        };
        let mut line = serde_json::to_string(&event)?;
        line.push('\n');
        let mut file = OpenOptions::new().append(true).open(self.events_path())?;
        file.write_all(line.as_bytes())?;
        Ok(event)
    }

    /// The schema as produced by the accepted change sets. Errors if
    /// `specs/schema.json` on disk differs (it was edited by hand).
    pub fn current_schema(&self) -> Result<SchemaSpec> {
        let events = self.events()?;
        let mut spec = empty_spec();
        for event in &events {
            if let EventKind::ChangeSetAccepted { proposal_seq, .. } = &event.kind {
                let proposed = events.iter().find_map(|e| match &e.kind {
                    EventKind::ChangeSetProposed { change_set, .. } if e.seq == *proposal_seq => {
                        Some(change_set)
                    }
                    _ => None,
                });
                let Some(cs) = proposed else {
                    return Err(OrchestratorError::CorruptEventLog(format!(
                        "event {} accepts proposal {proposal_seq}, which does not exist",
                        event.seq
                    )));
                };
                spec = apply(&spec, &cs.changes)?;
            }
        }
        let on_disk: SchemaSpec = serde_json::from_str(&fs::read_to_string(self.schema_path())?)?;
        if on_disk != spec {
            return Err(OrchestratorError::SchemaEditedByHand);
        }
        Ok(spec)
    }

    /// Apply pending migrations through `applier` and record the result:
    /// one `MigrationApplied` per applied migration, `MigrationFailed` if one
    /// failed, and `DatabaseChecked` when at least one was applied and none
    /// failed. Appends nothing when nothing was pending.
    pub fn apply_pending<A: Applier>(&self, applier: &A) -> Result<ApplyOutcome> {
        let report = applier
            .apply_pending(&self.dir)
            .map_err(OrchestratorError::Applier)?;
        let mut details = Vec::new();
        for m in &report.applied {
            let detail = applied_detail(m);
            self.append(
                Actor::System,
                EventKind::MigrationApplied {
                    migration: m.migration.clone(),
                    backup: m.backup.clone(),
                    rows_before: m.rows_before.clone(),
                    rows_after: m.rows_after.clone(),
                    detail: detail.clone(),
                },
            )?;
            details.push(detail);
        }
        if let Some(f) = &report.failed {
            let detail = failed_detail(f);
            self.append(
                Actor::System,
                EventKind::MigrationFailed {
                    migration: f.migration.clone(),
                    backup: f.backup.clone(),
                    error: f.error.clone(),
                    restored: f.restored,
                    detail: detail.clone(),
                },
            )?;
            details.push(detail);
        }
        let check = if !report.applied.is_empty() && report.failed.is_none() {
            let spec = self.current_schema()?;
            let step = check_db_matches_spec(&self.dir.join("data.sqlite"), &spec);
            self.append(
                Actor::System,
                EventKind::DatabaseChecked {
                    check: step.clone(),
                },
            )?;
            Some(step)
        } else {
            None
        };
        Ok(ApplyOutcome {
            report,
            check,
            details,
        })
    }

    fn next_migration_name(&self, cs_name: &str) -> Result<String> {
        let mut max = 0u32;
        for entry in fs::read_dir(self.migrations_dir())? {
            let name = entry?.file_name().to_string_lossy().into_owned();
            if let Some(n) = name.split('_').next().and_then(|p| p.parse::<u32>().ok()) {
                max = max.max(n);
            }
        }
        Ok(format!("{:04}_{cs_name}", max + 1))
    }

    /// Errors while questions are waiting; `prompt` and `changes` start nothing then.
    fn refuse_while_waiting(&self) -> Result<()> {
        let open = questions::open_questions(&self.events()?);
        if open.is_empty() {
            return Ok(());
        }
        let ids = open
            .iter()
            .map(|q| q.id.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let (n, verb) = (open.len(), if open.len() == 1 { "is" } else { "are" });
        let noun = if n == 1 { "question" } else { "questions" };
        Err(OrchestratorError::QuestionsWaiting(format!(
            "{n} {noun} {verb} waiting ({ids}). Answer them with `nocodo-api answer`, \
             or see them with `nocodo-api questions`."
        )))
    }

    /// The questions still waiting for an answer, derived from the log.
    pub fn open_questions(&self) -> Result<Vec<Question>> {
        Ok(questions::open_questions(&self.events()?))
    }

    /// The default answer for every open non-blocking question that `given`
    /// does not answer. Blocking questions are skipped.
    pub fn default_answers(&self, given: &[Answer]) -> Result<Vec<Answer>> {
        Ok(self
            .open_questions()?
            .into_iter()
            .filter(|q| !q.blocking && !given.iter().any(|a| a.question_id == q.id))
            .filter_map(|q| {
                q.default.map(|d| Answer {
                    question_id: q.id,
                    choice: d,
                    by_default: true,
                })
            })
            .collect())
    }

    /// Records answers (choices numbered from 1). Every answer is validated
    /// first, so a refused call records nothing. Never calls the model.
    pub fn answer(&self, answers: &[Answer]) -> Result<AnswerOutcome> {
        let events = self.events()?;
        let open = questions::open_questions(&events);
        let mut seen: Vec<&str> = Vec::new();
        for a in answers {
            let id = a.question_id.as_str();
            let refuse = |m: String| Err(OrchestratorError::InvalidAnswer(m));
            if seen.contains(&id) {
                return refuse(format!("{id} is answered twice in this command."));
            }
            seen.push(id);
            let Some(q) = open.iter().find(|q| q.id == id) else {
                let known = questions::asked(&events).iter().any(|(_, q)| q.id == id);
                return refuse(if known {
                    format!("{id} is already answered; an answer cannot be changed.")
                } else {
                    format!("There is no question {id}.")
                });
            };
            if a.choice < 1 || a.choice > q.choices.len() {
                return refuse(format!(
                    "{id} has {} choices, numbered from 1; {} is not one of them.",
                    q.choices.len(),
                    a.choice
                ));
            }
        }
        for a in answers {
            let q = open
                .iter()
                .find(|q| q.id == a.question_id)
                .expect("checked");
            let how = if a.by_default { " by the default" } else { "" };
            let detail = format!(
                "{} answered{how}: \"{}\" (choice {} of {}).",
                q.id,
                q.choices[a.choice - 1],
                a.choice,
                q.choices.len()
            );
            self.append(
                Actor::User,
                EventKind::QuestionAnswered {
                    answer: a.clone(),
                    detail,
                },
            )?;
        }
        let open = self.open_questions()?;
        Ok(if open.is_empty() {
            AnswerOutcome::ReadyToContinue
        } else {
            AnswerOutcome::StillWaiting { open }
        })
    }

    /// Resumes what the latest prompt run is waiting on or failed at.
    /// See the epic: a fully answered agent round, a fully answered removal
    /// confirmation, or a failed model call. Records nothing twice.
    pub async fn continue_pending<P: Proposer>(
        &self,
        proposer: &P,
        confirm_destructive: bool,
        max_attempts: u32,
    ) -> Result<ContinueOutcome> {
        let events = self.events()?;
        let last = events.iter().rev().find(|e| {
            matches!(
                e.kind,
                EventKind::UserPrompt { .. }
                    | EventKind::PromptFailed { .. }
                    | EventKind::QuestionsAsked { .. }
                    | EventKind::QuestionsRejected { .. }
                    | EventKind::ChangeSetProposed { .. }
                    | EventKind::ChangeSetAccepted { .. }
                    | EventKind::ChangeSetRejected { .. }
            )
        });
        let Some(last) = last else {
            return Ok(ContinueOutcome::NothingToContinue);
        };
        let prompt_seq = match &last.kind {
            EventKind::PromptFailed { prompt_seq, .. } => *prompt_seq,
            EventKind::QuestionsAsked {
                prompt_seq,
                proposal_seq,
                ..
            } => {
                let open = questions::open_questions(&events);
                if !open.is_empty() {
                    return Ok(ContinueOutcome::StillWaiting { open });
                }
                match (prompt_seq, proposal_seq) {
                    (Some(p), _) => *p,
                    (None, Some(ps)) => {
                        return Ok(ContinueOutcome::Continued(
                            self.resolve_confirmation(&events, last, *ps)?,
                        ));
                    }
                    (None, None) => return Ok(ContinueOutcome::NothingToContinue),
                }
            }
            _ => return Ok(ContinueOutcome::NothingToContinue),
        };
        let text = events.iter().find_map(|e| match &e.kind {
            EventKind::UserPrompt { text } if e.seq == prompt_seq => Some(text.clone()),
            _ => None,
        });
        let Some(text) = text else {
            return Err(OrchestratorError::CorruptEventLog(format!(
                "event {} refers to prompt {prompt_seq}, which does not exist",
                last.seq
            )));
        };
        let offset = questions::attempts_used(&events, prompt_seq);
        Ok(ContinueOutcome::Continued(
            self.drive_prompt(
                prompt_seq,
                &text,
                proposer,
                confirm_destructive,
                max_attempts,
                offset,
            )
            .await?,
        ))
    }

    /// A fully answered removal confirmation: yes continues the same proposal,
    /// no rejects it with `user_declined`. No second proposal is recorded.
    fn resolve_confirmation(
        &self,
        events: &[Event],
        asked: &Event,
        proposal_seq: u64,
    ) -> Result<PromptOutcome> {
        let EventKind::QuestionsAsked { questions, .. } = &asked.kind else {
            return Err(OrchestratorError::CorruptEventLog(
                "not a question round".into(),
            ));
        };
        let answers = questions::answers(events);
        let yes = questions
            .first()
            .and_then(|q| answers.get(&q.id))
            .is_some_and(|a| a.choice == 1);
        let proposed = events.iter().find_map(|e| match &e.kind {
            EventKind::ChangeSetProposed {
                change_set,
                attempt,
                assumptions,
                ..
            } if e.seq == proposal_seq => Some((change_set.clone(), *attempt, assumptions.clone())),
            _ => None,
        });
        let Some((cs, attempts, assumptions)) = proposed else {
            return Err(OrchestratorError::CorruptEventLog(format!(
                "event {} asks about proposal {proposal_seq}, which does not exist",
                asked.seq
            )));
        };
        let outcome = if yes {
            self.finish_proposal(proposal_seq, &cs, false)?
        } else {
            let findings = vec![Finding {
                code: "user_declined".into(),
                message: "The user chose to keep the data, so nothing was removed.".into(),
            }];
            self.append(
                Actor::System,
                EventKind::ChangeSetRejected {
                    proposal_seq,
                    findings: findings.clone(),
                    checks: Vec::new(),
                },
            )?;
            Outcome::Rejected {
                findings,
                checks: Vec::new(),
            }
        };
        Ok(PromptOutcome::Done {
            outcome,
            change_set: Some(cs),
            assumptions,
            attempts,
        })
    }

    /// Run `cs` through checker and sqlgen. On success writes the migration and
    /// `specs/schema.json`; on rejection only the event log changes. A change
    /// set that removes data and passes every check waits for the user's
    /// confirmation unless `confirm_destructive` is set.
    pub fn propose(
        &self,
        cs: ChangeSet,
        actor: Actor,
        confirm_destructive: bool,
    ) -> Result<Outcome> {
        self.refuse_while_waiting()?;
        self.propose_attempt(cs, actor, confirm_destructive, None, 1, Vec::new())
    }

    /// Prompt run: record the prompt, then ask `proposer` for questions or a
    /// change set. Questions are checked, numbered and recorded, and the run
    /// waits. A change set goes through [`Project::propose`]'s pipeline; on
    /// rejection the proposer is asked again with the findings, up to
    /// `max_attempts` times. Each attempt has its own events, linked by
    /// `prompt_seq` and `attempt`.
    pub async fn run_prompt<P: Proposer>(
        &self,
        prompt: &str,
        proposer: &P,
        confirm_destructive: bool,
        max_attempts: u32,
    ) -> Result<PromptOutcome> {
        self.refuse_while_waiting()?;
        let prompt_seq = self
            .append(
                Actor::User,
                EventKind::UserPrompt {
                    text: prompt.to_string(),
                },
            )?
            .seq;
        self.drive_prompt(
            prompt_seq,
            prompt,
            proposer,
            confirm_destructive,
            max_attempts,
            0,
        )
        .await
    }

    /// The model-call loop of a prompt. `offset` is the number of attempts
    /// already in the log; `max_attempts` counts only the attempts of this call.
    async fn drive_prompt<P: Proposer>(
        &self,
        prompt_seq: u64,
        prompt: &str,
        proposer: &P,
        confirm_destructive: bool,
        max_attempts: u32,
        offset: u32,
    ) -> Result<PromptOutcome> {
        let mut feedback: Option<Feedback> = None;
        let mut tries = 0;
        loop {
            tries += 1;
            let attempt = offset + tries;
            let events = self.events()?;
            let answers = questions::answered_questions(&events, prompt_seq);
            let current = self.current_schema()?;
            let Proposal { reply, actor } = match proposer
                .propose(&current, prompt, &answers, feedback.as_ref())
                .await
            {
                Ok(proposal) => proposal,
                Err(error) => {
                    let (message, run_id, detail) = prompt_failed_detail(prompt_seq, &error);
                    self.append(
                        Actor::System,
                        EventKind::PromptFailed {
                            prompt_seq,
                            error: message,
                            run_id,
                            detail,
                        },
                    )?;
                    return Err(OrchestratorError::Proposer(error));
                }
            };

            let reject_round = |findings: Vec<Finding>| -> Result<Outcome> {
                self.append(
                    Actor::System,
                    EventKind::QuestionsRejected {
                        prompt_seq,
                        findings: findings.clone(),
                    },
                )?;
                Ok(Outcome::Rejected {
                    findings,
                    checks: Vec::new(),
                })
            };
            let outcome = match (&reply.change_set, reply.questions.is_empty()) {
                (Some(_), false) | (None, true) => reject_round(vec![Finding {
                    code: "reply_shape".into(),
                    message: "Return either questions or a change set: exactly one of them, \
                              never both and never neither."
                        .into(),
                }])?,
                (Some(cs), true) => self.propose_attempt(
                    cs.clone(),
                    actor.clone(),
                    confirm_destructive,
                    Some(prompt_seq),
                    attempt,
                    reply.assumptions.clone(),
                )?,
                (None, false) => {
                    let findings = check_questions(&reply.questions, &answers);
                    if findings.is_empty() {
                        let questions =
                            self.ask_round(&events, prompt_seq, reply.questions.clone(), &actor)?;
                        return Ok(PromptOutcome::Waiting { questions });
                    }
                    reject_round(findings)?
                }
            };
            match outcome {
                Outcome::Waiting { questions } => {
                    return Ok(PromptOutcome::Waiting { questions });
                }
                Outcome::Rejected { findings, .. } if tries < max_attempts => {
                    feedback = Some(Feedback {
                        previous: reply,
                        findings,
                    });
                }
                outcome => {
                    return Ok(PromptOutcome::Done {
                        outcome,
                        change_set: reply.change_set,
                        assumptions: reply.assumptions,
                        attempts: attempt,
                    });
                }
            }
        }
    }

    /// Orders (blocking first, then the agent's order), numbers, de-duplicates
    /// and records a round of agent questions that passed the checks.
    fn ask_round(
        &self,
        events: &[Event],
        prompt_seq: u64,
        drafts: Vec<Question>,
        actor: &Actor,
    ) -> Result<Vec<Question>> {
        let (blocking, rest): (Vec<_>, Vec<_>) = drafts.into_iter().partition(|q| q.blocking);
        let first = questions::next_number(events);
        let mut kept: Vec<Question> = Vec::new();
        let mut dropped: Vec<String> = Vec::new();
        for (i, mut q) in blocking.into_iter().chain(rest).enumerate() {
            q.id = format!("q{}", first + i as u64);
            q.asked_by = SCHEMA.to_string();
            let text = questions::normalize(&q.text);
            if kept.iter().any(|k| questions::normalize(&k.text) == text) {
                dropped.push(q.id);
            } else {
                kept.push(q);
            }
        }
        if !dropped.is_empty() {
            let detail = format!(
                "{} dropped: the same question was asked twice in one round.",
                dropped.join(", ")
            );
            self.append(
                Actor::System,
                EventKind::QuestionsDropped {
                    question_ids: dropped,
                    detail,
                },
            )?;
        }
        let ids = kept
            .iter()
            .map(|q| q.id.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let needing = kept.iter().filter(|q| q.blocking).count();
        let detail = format!(
            "The Schema agent asked {} question(s) for prompt {prompt_seq} ({ids}); \
             {needing} need an explicit answer. The run waits for the answers.",
            kept.len()
        );
        self.append(
            actor.clone(),
            EventKind::QuestionsAsked {
                prompt_seq: Some(prompt_seq),
                proposal_seq: None,
                questions: kept.clone(),
                detail,
            },
        )?;
        Ok(kept)
    }

    fn propose_attempt(
        &self,
        cs: ChangeSet,
        actor: Actor,
        confirm_destructive: bool,
        prompt_seq: Option<u64>,
        attempt: u32,
        assumptions: Vec<String>,
    ) -> Result<Outcome> {
        let proposal = self.append(
            actor,
            EventKind::ChangeSetProposed {
                change_set: cs.clone(),
                prompt_seq,
                attempt,
                assumptions,
            },
        )?;
        let ask = cs.is_destructive() && !confirm_destructive;
        self.finish_proposal(proposal.seq, &cs, ask)
    }

    /// Checks and dry-runs the recorded proposal `proposal_seq` as if any
    /// removal were confirmed. With `ask_confirmation`, a proposal that passes
    /// everything waits for the user instead of being written.
    fn finish_proposal(
        &self,
        proposal_seq: u64,
        cs: &ChangeSet,
        ask_confirmation: bool,
    ) -> Result<Outcome> {
        let current = self.current_schema()?;
        let reject = |findings: Vec<Finding>, checks: Vec<CheckStep>| -> Result<Outcome> {
            self.append(
                Actor::System,
                EventKind::ChangeSetRejected {
                    proposal_seq,
                    findings: findings.clone(),
                    checks: checks.clone(),
                },
            )?;
            Ok(Outcome::Rejected { findings, checks })
        };

        let findings = check_change_set(&current, cs, true);
        let mut checks = vec![static_rules_step(&findings)];
        if !findings.is_empty() {
            return reject(findings, checks);
        }
        let migration = generate(&current, cs)?;
        let after = apply(&current, &cs.changes)?;
        let name = self.next_migration_name(&cs.name)?;
        let named = NamedMigration {
            name: name.clone(),
            up: migration.up.clone(),
            down: migration.down.clone(),
            table_renames: table_renames(&cs.changes),
        };
        let report = dry_run(
            Some(&self.dir.join("data.sqlite")),
            &self.migrations_dir(),
            &named,
            &current,
            &after,
        )?;
        checks.extend(report.steps);
        if !report.findings.is_empty() {
            return reject(report.findings, checks);
        }

        if ask_confirmation {
            let events = self.events()?;
            let n = questions::next_number(&events);
            let question = Question {
                id: format!("q{n}"),
                asked_by: SYSTEM.to_string(),
                text: questions::confirmation_text(&current, cs),
                choices: vec!["Yes, forget it".into(), "No, keep it".into()],
                default: None,
                blocking: true,
                why: format!(
                    "Proposal {proposal_seq} ({}) removes data, so the user must confirm it.",
                    cs.name
                ),
            };
            let detail = format!(
                "Proposal {proposal_seq} passed every check but removes data. \
                 Asking the user to confirm ({}); nothing is written until the answer is yes.",
                question.id
            );
            self.append(
                Actor::System,
                EventKind::QuestionsAsked {
                    prompt_seq: None,
                    proposal_seq: Some(proposal_seq),
                    questions: vec![question.clone()],
                    detail,
                },
            )?;
            return Ok(Outcome::Waiting {
                questions: vec![question],
            });
        }

        // Write the migration folder under a temp name, then rename it.
        let tmp = tempfile::Builder::new()
            .prefix(".migration-")
            .tempdir_in(&self.dir)?;
        fs::write(tmp.path().join("up.sql"), &migration.up)?;
        fs::write(tmp.path().join("down.sql"), &migration.down)?;
        fs::write(tmp.path().join("metadata.toml"), METADATA_TOML)?;
        fs::rename(tmp.keep(), self.migrations_dir().join(&name))?;

        write_json(&self.schema_path(), &after)?;
        self.append(
            Actor::System,
            EventKind::ChangeSetAccepted {
                proposal_seq,
                migration: name.clone(),
                checks: checks.clone(),
            },
        )?;
        Ok(Outcome::Accepted {
            migration: name,
            checks,
        })
    }
}
