//! The one home of operations: `init`, `changes`, `prompt`, `questions`,
//! `answer`, `continue_run`, `apply`, `inspect` and `sample_rows`. Each takes
//! a project folder and returns a typed result from `nocodo-types`. Front ends
//! (CLI, API) parse input and present results; they add no logic of their own.
//!
//! Must not print. Calls are synchronous: one call, one result.

mod error;
mod lock;
mod proposer;

use std::path::{Path, PathBuf};

use nocodo_types::change::ChangeSet;
use nocodo_types::event::{Actor, EventKind};
use nocodo_types::outcome::{
    AnswerRun, ApplyOutcome, ChangesRun, ContinueRun, InitOutcome, InspectOutcome, Outcome,
    ProjectInfo, PromptOutcome, PromptRun, RowPage, SampleRowsOutcome,
};
use nocodo_types::project::ProjectBase;
use nocodo_types::question::{Answer, Question};

pub use error::EngineError;
pub use lock::{LOCK_FILE, WriteLock};
pub use nocodo_orchestrator::{OrchestratorError, Project, Proposer};
use proposer::RuntimeApplier;
pub use proposer::{SchemaProposer, schema_proposer};

type Result<T> = std::result::Result<T, EngineError>;

/// Maximum attempts (initial proposal plus repairs) for one prompt.
const MAX_ATTEMPTS: u32 = 3;

/// Settings file `init` copies into new projects when no base file is given.
pub const PROJECT_BASE_FILE: &str = "project-base.json";

fn block_on<F: std::future::Future>(f: F) -> Result<F::Output> {
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| EngineError::Message(format!("starting async runtime: {e}")))?;
    Ok(rt.block_on(f))
}

/// Create a new project folder. `name` defaults to the folder's name; `base`
/// to `project-base.json` in the current folder, if present.
pub fn init(dir: &Path, name: Option<String>, base: Option<PathBuf>) -> Result<InitOutcome> {
    let name = match name {
        Some(n) => n,
        None => dir
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| {
                EngineError::Message(
                    "cannot derive a project name from the folder; pass --name".to_string(),
                )
            })?
            .to_string(),
    };
    let base_path = base.or_else(|| {
        let default = PathBuf::from(PROJECT_BASE_FILE);
        default.is_file().then_some(default)
    });
    let (base, source) = match &base_path {
        Some(path) => {
            let text = std::fs::read_to_string(path)
                .map_err(|e| EngineError::Message(format!("reading {}: {e}", path.display())))?;
            let base: ProjectBase = serde_json::from_str(&text)
                .map_err(|e| EngineError::Message(format!("parsing {}: {e}", path.display())))?;
            (base, path.display().to_string())
        }
        None => (
            ProjectBase::default(),
            format!("built-in defaults (no {PROJECT_BASE_FILE} found)"),
        ),
    };
    Project::init_with_base(dir, &name, &base)?;
    Ok(InitOutcome {
        name,
        source,
        schema_model: base.models.schema.model,
    })
}

/// Apply everything pending, after a backup.
pub fn apply(dir: &Path) -> Result<ApplyOutcome> {
    let _lock = WriteLock::acquire(dir, "apply")?;
    apply_project(&Project::open(dir)?)
}

fn apply_project(project: &Project) -> Result<ApplyOutcome> {
    Ok(project.apply_pending(&RuntimeApplier)?)
}

/// Propose a hand-written change set. An accepted one is applied at once.
pub fn changes(dir: &Path, change_set: ChangeSet, confirm_destructive: bool) -> Result<ChangesRun> {
    let _lock = WriteLock::acquire(dir, "changes")?;
    let project = Project::open(dir)?;
    let outcome = project.propose(change_set, Actor::Developer, confirm_destructive)?;
    let applied = match outcome {
        Outcome::Accepted { .. } => Some(apply_project(&project)?),
        _ => None,
    };
    Ok(ChangesRun { outcome, applied })
}

/// The questions waiting for an answer.
pub fn questions(dir: &Path) -> Result<Vec<Question>> {
    Ok(Project::open(dir)?.open_questions()?)
}

/// Turn a plain-language prompt into a schema change via the Schema agent.
pub fn prompt(dir: &Path, text: &str, confirm_destructive: bool) -> Result<PromptRun> {
    prompt_with(dir, text, confirm_destructive, schema_proposer)
}

/// `prompt` with the proposer built by `make` (tests inject a scripted one).
pub fn prompt_with<P: Proposer>(
    dir: &Path,
    text: &str,
    confirm_destructive: bool,
    make: impl FnOnce(&Project) -> P,
) -> Result<PromptRun> {
    let _lock = WriteLock::acquire(dir, "prompt")?;
    let project = Project::open(dir)?;
    let proposer = make(&project);
    let outcome =
        block_on(project.run_prompt(text, &proposer, confirm_destructive, MAX_ATTEMPTS))??;
    finish_prompt(&project, outcome)
}

/// Record answers; with `defaults`, also accept the default for every open
/// non-blocking question not answered here. Resumes the run once every
/// question is answered.
pub fn answer(dir: &Path, given: Vec<Answer>, defaults: bool) -> Result<AnswerRun> {
    answer_with(dir, given, defaults, schema_proposer)
}

/// `answer` with the proposer built by `make`.
pub fn answer_with<P: Proposer>(
    dir: &Path,
    mut given: Vec<Answer>,
    defaults: bool,
    make: impl FnOnce(&Project) -> P,
) -> Result<AnswerRun> {
    let _lock = WriteLock::acquire(dir, "answer")?;
    let project = Project::open(dir)?;
    if defaults {
        let more = project.default_answers(&given)?;
        given.extend(more);
    }
    match project.answer(&given)? {
        nocodo_types::outcome::AnswerOutcome::StillWaiting { open } => {
            Ok(AnswerRun::StillWaiting { open })
        }
        nocodo_types::outcome::AnswerOutcome::ReadyToContinue => Ok(AnswerRun::Resumed(Box::new(
            resume(&project, &make(&project))?,
        ))),
    }
}

/// Retry whatever failed last, or resume an answered round.
pub fn continue_run(dir: &Path) -> Result<ContinueRun> {
    continue_with(dir, schema_proposer)
}

/// `continue_run` with the proposer built by `make`.
pub fn continue_with<P: Proposer>(
    dir: &Path,
    make: impl FnOnce(&Project) -> P,
) -> Result<ContinueRun> {
    let _lock = WriteLock::acquire(dir, "continue")?;
    let project = Project::open(dir)?;
    resume(&project, &make(&project))
}

fn resume<P: Proposer>(project: &Project, proposer: &P) -> Result<ContinueRun> {
    match block_on(project.continue_pending(proposer, false, MAX_ATTEMPTS))?? {
        nocodo_types::outcome::ContinueOutcome::NothingToContinue => {
            Ok(ContinueRun::NothingToContinue)
        }
        nocodo_types::outcome::ContinueOutcome::StillWaiting { open } => {
            Ok(ContinueRun::StillWaiting { open })
        }
        nocodo_types::outcome::ContinueOutcome::Continued(outcome) => Ok(ContinueRun::Continued(
            Box::new(finish_prompt(project, outcome)?),
        )),
    }
}

/// An accepted prompt run is applied at once.
fn finish_prompt(project: &Project, outcome: PromptOutcome) -> Result<PromptRun> {
    let accepted = matches!(
        &outcome,
        PromptOutcome::Done {
            outcome: Outcome::Accepted { .. },
            ..
        }
    );
    let applied = if accepted {
        Some(apply_project(project)?)
    } else {
        None
    };
    Ok(PromptRun { outcome, applied })
}

/// What is stored (for the reviewer) and a plain-language summary (for the user).
pub fn inspect(dir: &Path) -> Result<InspectOutcome> {
    let project = Project::open(dir)?;
    let spec = project.current_schema()?;
    let info = nocodo_runtime::inspect(project.dir())?;
    let summary = nocodo_runtime::summary(&spec, &info.tables);
    Ok(InspectOutcome {
        tables: info.tables,
        applied: info.applied,
        pending: info.pending,
        backups: info.backups,
        summary,
    })
}

/// Fill every list with a few obviously fake rows (a test aid).
pub fn sample_rows(dir: &Path, per_table: u32) -> Result<SampleRowsOutcome> {
    let _lock = WriteLock::acquire(dir, "sample-rows")?;
    let project = Project::open(dir)?;
    let spec = project.current_schema()?;
    let rows = nocodo_runtime::add_sample_rows(project.dir(), &spec, per_table)?;
    let total: u64 = rows.values().sum();
    let list = rows
        .iter()
        .map(|(t, n)| format!("{t}: {n}"))
        .collect::<Vec<_>>()
        .join(", ");
    let detail = format!(
        "Added {total} sample rows to data.sqlite ({per_table} per table; {list}). \
         The values are fake and generated from the schema."
    );
    project.append(
        Actor::Developer,
        EventKind::SampleRowsAdded {
            rows: rows.clone(),
            detail: detail.clone(),
        },
    )?;
    Ok(SampleRowsOutcome { rows, detail })
}

/// The projects under `root`: each folder with a `project.json`, by folder
/// name. Folders without one are skipped; a project that cannot be read is
/// listed with the reason. A missing `root` has no projects.
pub fn list_projects(root: &Path) -> Result<Vec<ProjectInfo>> {
    let entries = match std::fs::read_dir(root) {
        Ok(rd) => rd,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => {
            return Err(EngineError::Message(format!(
                "reading projects folder {}: {e}",
                root.display()
            )));
        }
    };
    let mut dirs: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.join("project.json").is_file())
        .collect();
    dirs.sort();
    Ok(dirs.into_iter().map(|dir| project_info(&dir)).collect())
}

fn project_info(dir: &Path) -> ProjectInfo {
    let folder = dir
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let read = || -> Result<(String, usize, Option<jiff::Timestamp>)> {
        let project = Project::open(dir)?;
        let tables = project.current_schema()?.tables.len();
        let last = project.events()?.last().map(|e| e.at);
        Ok((project.config().name.clone(), tables, last))
    };
    match read() {
        Ok((name, tables, last_event)) => ProjectInfo {
            name,
            path: dir.display().to_string(),
            tables: Some(tables),
            last_event,
            error: None,
        },
        Err(e) => ProjectInfo {
            name: folder,
            path: dir.display().to_string(),
            tables: None,
            last_event: None,
            error: Some(e.to_string()),
        },
    }
}

/// One page of a table's rows, read-only. Works while a writer is running.
pub fn read_rows(dir: &Path, table: &str, offset: u64, limit: u64) -> Result<RowPage> {
    let project = Project::open(dir)?;
    let spec = project.current_schema()?;
    let found = spec.tables.iter().any(|t| t.name == table);
    match nocodo_runtime::read_rows(project.dir(), &spec, table, offset, limit) {
        Err(nocodo_runtime::RuntimeError::Refused(m)) if !found => Err(EngineError::NotFound(m)),
        other => Ok(other?),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nocodo-engine-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn lists_projects_skips_folders_and_shows_broken_ones() {
        let root = temp("list");
        assert!(list_projects(&root).unwrap().is_empty());
        init(&root.join("b-second"), Some("Second".into()), None).unwrap();
        init(&root.join("a-first"), None, None).unwrap();
        std::fs::create_dir_all(root.join("not-a-project")).unwrap();
        std::fs::write(root.join("a-file"), "x").unwrap();
        std::fs::create_dir_all(root.join("c-broken")).unwrap();
        std::fs::write(root.join("c-broken/project.json"), "{ nope").unwrap();
        let list = list_projects(&root).unwrap();
        let names: Vec<_> = list.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["a-first", "Second", "c-broken"]);
        assert_eq!(list[0].tables, Some(0));
        assert!(list[0].last_event.is_some());
        assert!(list[0].error.is_none());
        assert!(list[2].error.is_some());
        assert_eq!(list[2].tables, None);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn read_rows_on_empty_and_missing_projects() {
        let dir = temp("rows");
        init(&dir, Some("rows".into()), None).unwrap();
        let err = read_rows(&dir, "expenses", 0, 10).unwrap_err();
        assert!(err.to_string().contains("no list called"), "{err}");
        let err = read_rows(&dir.join("gone"), "expenses", 0, 10).unwrap_err();
        assert!(err.to_string().contains("not a nocodo project"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn writers_are_refused_while_locked_but_reads_work() {
        let dir = std::env::temp_dir().join(format!("nocodo-engine-lock-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(&dir, Some("locked".to_string()), None).unwrap();
        let held = WriteLock::acquire(&dir, "prompt").unwrap();
        assert!(matches!(
            sample_rows(&dir, 2),
            Err(EngineError::Busy { .. })
        ));
        assert!(matches!(apply(&dir), Err(EngineError::Busy { .. })));
        assert!(matches!(continue_run(&dir), Err(EngineError::Busy { .. })));
        assert!(questions(&dir).unwrap().is_empty());
        assert!(inspect(&dir).is_ok());
        drop(held);
        assert!(apply(&dir).is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
