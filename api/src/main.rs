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
use nocodo_orchestrator::{Outcome, Project};
use nocodo_types::change::ChangeSet;
use nocodo_types::event::Actor;

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
    },
    /// Propose a hand-written change set (JSON file) to the project.
    Changes {
        /// Path to the change set JSON file.
        file: PathBuf,
        /// Allow destructive changes (dropping tables or columns).
        #[arg(long)]
        confirm_destructive: bool,
    },
}

fn run(cli: Cli) -> Result<ExitCode> {
    let dir = cli
        .project
        .context("missing --project <dir> (or set NOCODO_PROJECT)")?;
    match cli.command {
        Command::Init { name } => {
            let name = match name {
                Some(n) => n,
                None => dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .context("cannot derive a project name from the folder; pass --name")?
                    .to_string(),
            };
            Project::init(&dir, &name)?;
            println!("Initialized project \"{name}\" in {}", dir.display());
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
                Outcome::Accepted { migration } => {
                    println!("Accepted \u{2192} migrations/{migration}");
                    Ok(ExitCode::SUCCESS)
                }
                Outcome::Rejected { findings } => {
                    println!("Rejected:");
                    for f in findings {
                        println!("  [{}] {}", f.code, f.message);
                    }
                    Ok(ExitCode::FAILURE)
                }
            }
        }
    }
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::from(2)
        }
    }
}
