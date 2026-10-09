//! Thin Axum layer over orchestrator, agents and runtime.
//!
//! Must not contain business logic.

use nocodo_agent as _;
use nocodo_agent_pm as _;
use nocodo_agent_schema as _;
use nocodo_checker as _;
use nocodo_orchestrator as _;
use nocodo_runtime as _;
use nocodo_sqlgen as _;
use nocodo_types as _;

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use nocodo_agent::AgentContext;
use nocodo_agent_schema::SchemaAgent;
use nocodo_orchestrator::{
    Applier, ApplierError, Outcome, Project, Proposal, Proposer, ProposerError,
};
use nocodo_types::change::ChangeSet;
use nocodo_types::event::{Actor, CheckStep, EventKind};
use nocodo_types::project::ProjectBase;
use nocodo_types::schema::SchemaSpec;

/// Maximum attempts (initial proposal plus repairs) for one prompt.
const MAX_ATTEMPTS: u32 = 3;

/// Settings file `init` copies into new projects when `--base` isn't given.
const PROJECT_BASE_FILE: &str = "project-base.json";

/// Adapts the runtime to the orchestrator's `Applier` trait.
struct RuntimeApplier;

impl Applier for RuntimeApplier {
    fn apply_pending(
        &self,
        project_dir: &std::path::Path,
    ) -> Result<nocodo_types::apply::ApplyReport, ApplierError> {
        Ok(nocodo_runtime::apply_pending(project_dir)?)
    }
}

/// Apply everything pending, print one line per migration, and pick the exit code.
fn apply_and_report(project: &Project) -> Result<ExitCode> {
    let out = project.apply_pending(&RuntimeApplier)?;
    if out.details.is_empty() {
        println!("Nothing to apply: data.sqlite is up to date.");
    }
    for d in &out.details {
        println!("{d}");
    }
    let mut ok = out.report.failed.is_none();
    if let Some(check) = out.check.as_ref().filter(|c| !c.passed) {
        println!("Database check failed: {}", check.detail);
        ok = false;
    }
    Ok(if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// Adapts the Schema agent to the orchestrator's `Proposer` trait.
struct SchemaProposer {
    ctx: AgentContext,
}

impl Proposer for SchemaProposer {
    async fn propose(
        &self,
        current: &SchemaSpec,
        prompt: &str,
        feedback: Option<&nocodo_orchestrator::Feedback>,
    ) -> Result<Proposal, ProposerError> {
        let feedback = feedback.map(|f| nocodo_agent_schema::Feedback {
            previous: f.previous.clone(),
            findings: f.findings.clone(),
        });
        let (proposal, run_id) =
            SchemaAgent::propose_with_run_id(&self.ctx, current, prompt, feedback.as_ref()).await?;
        Ok(Proposal {
            change_set: proposal.change_set,
            assumptions: proposal.assumptions,
            actor: Actor::SchemaAgent {
                run_id,
                model: format!("{}:{}", self.ctx.model.provider, self.ctx.model.model),
            },
        })
    }
}

#[derive(Parser)]
#[command(name = "nocodo-api", version)]
struct Cli {
    /// Project folder.
    #[arg(long, global = true, env = "NOCODO_PROJECT")]
    project: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create a new project folder.
    Init {
        /// Project name; defaults to the folder's name.
        #[arg(long)]
        name: Option<String>,
        /// Settings file copied into `project.json` (models per agent).
        /// Defaults to `project-base.json` in the current folder, if present.
        #[arg(long)]
        base: Option<PathBuf>,
    },
    /// Propose a hand-written change set (JSON file) to the project.
    Changes {
        /// Path to the change set JSON file.
        file: PathBuf,
        /// Allow destructive changes (dropping tables or columns).
        #[arg(long)]
        confirm_destructive: bool,
    },
    /// Turn a plain-language prompt into a schema change via the Schema agent.
    Prompt {
        /// Allow destructive changes (dropping tables or columns).
        #[arg(long)]
        confirm_destructive: bool,
        /// Read the prompt from a file instead of the command line.
        #[arg(long, conflicts_with = "text")]
        file: Option<PathBuf>,
        /// The prompt text.
        #[arg(required_unless_present = "file")]
        text: Option<String>,
    },
    /// Apply every pending migration to data.sqlite (with a backup first).
    Apply,
    /// Show what is stored (for the reviewer) and a plain-language summary (for the user).
    Inspect,
    /// Fill every list with a few obviously fake rows (a test aid).
    SampleRows {
        /// Rows to add to each list.
        #[arg(long, default_value_t = 3)]
        per_table: u32,
    },
}

fn run(cli: Cli) -> Result<ExitCode> {
    let dir = cli
        .project
        .context("missing --project <dir> (or set NOCODO_PROJECT)")?;
    match cli.command {
        Command::Init { name, base } => {
            let name = match name {
                Some(n) => n,
                None => dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .context("cannot derive a project name from the folder; pass --name")?
                    .to_string(),
            };
            let base_path = base.or_else(|| {
                let default = PathBuf::from(PROJECT_BASE_FILE);
                default.is_file().then_some(default)
            });
            let (base, source) = match &base_path {
                Some(path) => {
                    let text = std::fs::read_to_string(path)
                        .with_context(|| format!("reading {}", path.display()))?;
                    let base: ProjectBase = serde_json::from_str(&text)
                        .with_context(|| format!("parsing {}", path.display()))?;
                    (base, path.display().to_string())
                }
                None => (
                    ProjectBase::default(),
                    format!("built-in defaults (no {PROJECT_BASE_FILE} found)"),
                ),
            };
            Project::init_with_base(&dir, &name, &base)?;
            println!("Initialized project \"{name}\" in {}", dir.display());
            println!("Settings from {source}");
            println!("  schema agent model: {}", base.models.schema.model);
            Ok(ExitCode::SUCCESS)
        }
        Command::Changes {
            file,
            confirm_destructive,
        } => {
            let text = std::fs::read_to_string(&file)
                .with_context(|| format!("reading {}", file.display()))?;
            let cs: ChangeSet = serde_json::from_str(&text)
                .with_context(|| format!("parsing {}", file.display()))?;
            let project = Project::open(&dir)?;
            match project.propose(cs, Actor::Developer, confirm_destructive)? {
                Outcome::Accepted { migration, checks } => {
                    println!("Accepted \u{2192} migrations/{migration}");
                    println!("{}", checks_line(&checks));
                    apply_and_report(&project)
                }
                Outcome::Rejected { findings, checks } => {
                    println!("Rejected:");
                    for f in findings {
                        println!("  [{}] {}", f.code, f.message);
                    }
                    println!("{}", checks_line(&checks));
                    Ok(ExitCode::FAILURE)
                }
            }
        }
        Command::Apply => apply_and_report(&Project::open(&dir)?),
        Command::Inspect => {
            let project = Project::open(&dir)?;
            let spec = project.current_schema()?;
            let info = nocodo_runtime::inspect(project.dir())?;
            println!("== For the reviewer ==");
            println!("Tables (rows):");
            if info.tables.is_empty() {
                println!("  (none; no data.sqlite yet or no tables)");
            }
            for (t, n) in &info.tables {
                println!("  {t}: {n}");
            }
            println!("Applied migrations: {}", names_or_none(&info.applied));
            println!("Pending migrations: {}", names_or_none(&info.pending));
            println!("Backups: {}", info.backups);
            println!();
            println!("== For the user ==");
            print!("{}", nocodo_runtime::summary(&spec, &info.tables));
            Ok(ExitCode::SUCCESS)
        }
        Command::SampleRows { per_table } => {
            let project = Project::open(&dir)?;
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
            println!("{detail}");
            Ok(ExitCode::SUCCESS)
        }
        Command::Prompt {
            confirm_destructive,
            file,
            text,
        } => {
            let prompt = match (file, text) {
                (Some(f), _) => std::fs::read_to_string(&f)
                    .with_context(|| format!("reading {}", f.display()))?,
                (None, Some(t)) => t,
                (None, None) => anyhow::bail!("give the prompt text or --file <path>"),
            };
            let project = Project::open(&dir)?;
            let ctx = AgentContext::new(
                project.dir(),
                nocodo_agent_schema::ROLE,
                project.config().models.schema.clone(),
            );
            let proposer = SchemaProposer { ctx };
            let rt = tokio::runtime::Runtime::new().context("starting async runtime")?;
            let result = rt.block_on(project.run_prompt(
                &prompt,
                &proposer,
                confirm_destructive,
                MAX_ATTEMPTS,
            ))?;
            match result.outcome {
                Outcome::Accepted { migration, checks } => {
                    println!("{}", result.change_set.summary);
                    print_assumptions(&result.assumptions);
                    println!("Migration: migrations/{migration}");
                    println!("{}", checks_line(&checks));
                    apply_and_report(&project)
                }
                Outcome::Rejected { findings, checks } => {
                    println!("Rejected after {} attempt(s):", result.attempts);
                    for f in findings {
                        println!("  [{}] {}", f.code, f.message);
                    }
                    println!("{}", checks_line(&checks));
                    print_assumptions(&result.assumptions);
                    Ok(ExitCode::FAILURE)
                }
            }
        }
    }
}

/// One line summing up the checks that ran.
fn checks_line(checks: &[CheckStep]) -> String {
    if let Some(failed) = checks.iter().find(|c| !c.passed) {
        let passed = checks.iter().filter(|c| c.passed).count();
        return format!(
            "Checks: {passed} passed, then \"{}\" failed (see events.jsonl).",
            failed.check
        );
    }
    if checks.iter().any(|c| c.check == "up_again") {
        "Checked on SQLite: up, down, up again; database matched the spec each time.".to_string()
    } else {
        format!("Checks passed: {}.", checks.len())
    }
}

fn names_or_none(names: &[String]) -> String {
    if names.is_empty() {
        "(none)".to_string()
    } else {
        names.join(", ")
    }
}

fn print_assumptions(assumptions: &[String]) {
    if assumptions.is_empty() {
        return;
    }
    println!("Assumptions:");
    for a in assumptions {
        println!("  - {a}");
    }
}

fn main() -> ExitCode {
    // Load `.env` (searched from the current folder upwards) before parsing,
    // so it can supply `ANTHROPIC_API_KEY` and `NOCODO_PROJECT`. Variables
    // already set in the shell win.
    let _ = dotenvy::dotenv();
    match run(Cli::parse()) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::from(2)
        }
    }
}
