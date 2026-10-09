use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use nocodo_checker::{NamedMigration, check_change_set, dry_run};
use nocodo_sqlgen::{METADATA_TOML, apply, generate};
use nocodo_types::change::ChangeSet;
use nocodo_types::event::{Actor, Event, EventKind, Finding};
use nocodo_types::project::ProjectConfig;
use nocodo_types::schema::{SPEC_VERSION, SchemaSpec};

use crate::error::OrchestratorError;

type Result<T> = std::result::Result<T, OrchestratorError>;

/// Result of [`Project::propose`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Migration folder name, e.g. `0002_add_family_member`.
    Accepted {
        migration: String,
    },
    Rejected {
        findings: Vec<Finding>,
    },
}

/// What a [`Proposer`] returns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proposal {
    pub change_set: ChangeSet,
    /// Guesses the proposer made, as plain sentences.
    pub assumptions: Vec<String>,
    /// Who made the proposal (recorded on the `ChangeSetProposed` event).
    pub actor: Actor,
}

/// Why the previous attempt was rejected, passed back to the proposer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Feedback {
    pub previous: ChangeSet,
    pub findings: Vec<Finding>,
}

pub type ProposerError = Box<dyn std::error::Error + Send + Sync>;

/// Source of change sets for [`Project::run_prompt`]. Implemented outside the
/// orchestrator (the api adapts the Schema agent), so no agent dependency.
pub trait Proposer {
    fn propose(
        &self,
        current: &SchemaSpec,
        prompt: &str,
        feedback: Option<&Feedback>,
    ) -> impl std::future::Future<Output = std::result::Result<Proposal, ProposerError>> + Send;
}

/// Result of [`Project::run_prompt`]: the last attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptOutcome {
    pub outcome: Outcome,
    pub change_set: ChangeSet,
    pub assumptions: Vec<String>,
    pub attempts: u32,
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
    /// Create a new project folder. Fails if `dir` exists and is not empty.
    pub fn init(dir: impl AsRef<Path>, name: &str) -> Result<Project> {
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
            models: Default::default(),
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

    /// Run `cs` through checker and sqlgen. On success writes the migration and
    /// `specs/schema.json`; on rejection only the event log changes.
    pub fn propose(
        &self,
        cs: ChangeSet,
        actor: Actor,
        confirm_destructive: bool,
    ) -> Result<Outcome> {
        self.propose_attempt(cs, actor, confirm_destructive, None, 1, Vec::new())
    }

    /// Prompt run: record the prompt, then ask `proposer` for a change set,
    /// run it through [`Project::propose`]'s pipeline, and on rejection ask
    /// again with the findings, up to `max_attempts` times. Each attempt has
    /// its own proposed/rejected/accepted events, linked by `prompt_seq` and
    /// `attempt`. Returns the last attempt's outcome.
    pub async fn run_prompt<P: Proposer>(
        &self,
        prompt: &str,
        proposer: &P,
        confirm_destructive: bool,
        max_attempts: u32,
    ) -> Result<PromptOutcome> {
        let prompt_seq = self
            .append(
                Actor::User,
                EventKind::UserPrompt {
                    text: prompt.to_string(),
                },
            )?
            .seq;
        let mut feedback: Option<Feedback> = None;
        let mut attempt = 0;
        loop {
            attempt += 1;
            let current = self.current_schema()?;
            let proposal = proposer
                .propose(&current, prompt, feedback.as_ref())
                .await
                .map_err(OrchestratorError::Proposer)?;
            let outcome = self.propose_attempt(
                proposal.change_set.clone(),
                proposal.actor,
                confirm_destructive,
                Some(prompt_seq),
                attempt,
                proposal.assumptions.clone(),
            )?;
            match outcome {
                Outcome::Rejected { findings } if attempt < max_attempts => {
                    feedback = Some(Feedback {
                        previous: proposal.change_set,
                        findings,
                    });
                }
                outcome => {
                    return Ok(PromptOutcome {
                        outcome,
                        change_set: proposal.change_set,
                        assumptions: proposal.assumptions,
                        attempts: attempt,
                    });
                }
            }
        }
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
        let current = self.current_schema()?;
        let proposal = self.append(
            actor.clone(),
            EventKind::ChangeSetProposed {
                change_set: cs.clone(),
                prompt_seq,
                attempt,
                assumptions,
            },
        )?;

        let reject = |findings: Vec<Finding>| -> Result<Outcome> {
            self.append(
                Actor::System,
                EventKind::ChangeSetRejected {
                    proposal_seq: proposal.seq,
                    findings: findings.clone(),
                },
            )?;
            Ok(Outcome::Rejected { findings })
        };

        let findings = check_change_set(&current, &cs, confirm_destructive);
        if !findings.is_empty() {
            return reject(findings);
        }
        let migration = generate(&current, &cs)?;
        let after = apply(&current, &cs.changes)?;
        let name = self.next_migration_name(&cs.name)?;
        let named = NamedMigration {
            name: name.clone(),
            up: migration.up.clone(),
            down: migration.down.clone(),
        };
        let findings = dry_run(
            Some(&self.dir.join("data.sqlite")),
            &self.migrations_dir(),
            &named,
            &current,
            &after,
        )?;
        if !findings.is_empty() {
            return reject(findings);
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
                proposal_seq: proposal.seq,
                migration: name.clone(),
            },
        )?;
        Ok(Outcome::Accepted { migration: name })
    }
}
