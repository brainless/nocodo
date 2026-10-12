//! User interface: clap subcommands over the engine. Parses input and prints
//! results; it adds no logic of its own.
//!
//! Must not call agents or the DB directly.

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use nocodo_types::change::ChangeSet;
use nocodo_types::event::CheckStep;
use nocodo_types::outcome::{
    AnswerRun, ApplyOutcome, ChangesRun, ContinueRun, Outcome, PromptOutcome, PromptRun,
};
use nocodo_types::question::{Answer, Question};

/// Prints what an apply did and picks the exit code.
fn report_apply(out: &ApplyOutcome) -> ExitCode {
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
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

#[derive(Parser)]
#[command(name = "nocodo", version)]
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
    /// Show the questions waiting for an answer.
    Questions,
    /// Answer waiting questions (`q1=2 q3=1`); resumes the run once all are answered.
    Answer {
        /// Accept the default for every open non-blocking question not answered here.
        #[arg(long)]
        defaults: bool,
        /// `<question id>=<choice number>`, choices numbered from 1.
        #[arg(required_unless_present = "defaults")]
        answers: Vec<String>,
    },
    /// Retry whatever failed last, or resume an answered round.
    Continue,
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
            let out = nocodo_engine::init(&dir, name, base)?;
            println!("Initialized project \"{}\" in {}", out.name, dir.display());
            println!("Settings from {}", out.source);
            println!("  schema agent model: {}", out.schema_model);
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
            let ChangesRun { outcome, applied } =
                nocodo_engine::changes(&dir, cs, confirm_destructive)?;
            match outcome {
                Outcome::Accepted { migration, checks } => {
                    print_accepted(&migration, &checks, None, &[]);
                    Ok(applied.as_ref().map_or(ExitCode::SUCCESS, report_apply))
                }
                Outcome::Rejected { findings, checks } => {
                    print_rejected(&findings, &checks, 0, &[]);
                    Ok(ExitCode::FAILURE)
                }
                Outcome::Waiting { questions } => {
                    print_questions(&questions);
                    Ok(ExitCode::SUCCESS)
                }
            }
        }
        Command::Questions => {
            let open = nocodo_engine::questions(&dir)?;
            if open.is_empty() {
                println!("No questions waiting.");
            } else {
                print_questions(&open);
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Answer { defaults, answers } => {
            let given = answers
                .iter()
                .map(|a| parse_answer(a))
                .collect::<Result<Vec<_>>>()?;
            match nocodo_engine::answer(&dir, given, defaults)? {
                AnswerRun::StillWaiting { open } => {
                    println!("Answers recorded. Still waiting for {}:", open.len());
                    print_questions(&open);
                    Ok(ExitCode::SUCCESS)
                }
                AnswerRun::Resumed(run) => {
                    println!("Answers recorded. Continuing.");
                    Ok(print_continue(*run))
                }
            }
        }
        Command::Continue => Ok(print_continue(nocodo_engine::continue_run(&dir)?)),
        Command::Apply => Ok(report_apply(&nocodo_engine::apply(&dir)?)),
        Command::Inspect => {
            let info = nocodo_engine::inspect(&dir)?;
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
            print!("{}", info.summary);
            Ok(ExitCode::SUCCESS)
        }
        Command::SampleRows { per_table } => {
            let out = nocodo_engine::sample_rows(&dir, per_table)?;
            println!("{}", out.detail);
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
            Ok(print_prompt_run(nocodo_engine::prompt(
                &dir,
                &prompt,
                confirm_destructive,
            )?))
        }
    }
}

/// Parses `<question id>=<choice number>`.
fn parse_answer(arg: &str) -> Result<Answer> {
    let bad = || {
        anyhow::anyhow!(
            "cannot read the answer \"{arg}\": write it as <question id>=<choice number>, for example q1=2"
        )
    };
    let (id, choice) = arg.split_once('=').ok_or_else(bad)?;
    let choice: usize = choice.trim().parse().map_err(|_| bad())?;
    if id.trim().is_empty() {
        return Err(bad());
    }
    Ok(Answer {
        question_id: id.trim().to_string(),
        choice,
        by_default: false,
    })
}

/// Prints what `continue` (or a resumed `answer`) did.
fn print_continue(run: ContinueRun) -> ExitCode {
    match run {
        ContinueRun::NothingToContinue => {
            println!("Nothing to continue.");
            ExitCode::SUCCESS
        }
        ContinueRun::StillWaiting { open } => {
            println!("Still waiting for {}:", open.len());
            print_questions(&open);
            ExitCode::SUCCESS
        }
        ContinueRun::Continued(run) => print_prompt_run(*run),
    }
}

/// Prints a prompt run's result (shared by `prompt`, `answer` and `continue`).
fn print_prompt_run(run: PromptRun) -> ExitCode {
    match run.outcome {
        PromptOutcome::Waiting { questions } => {
            print_questions(&questions);
            ExitCode::SUCCESS
        }
        PromptOutcome::Done {
            outcome,
            change_set,
            assumptions,
            attempts,
        } => match outcome {
            Outcome::Accepted { migration, checks } => {
                print_accepted(&migration, &checks, change_set.as_ref(), &assumptions);
                run.applied.as_ref().map_or(ExitCode::SUCCESS, report_apply)
            }
            Outcome::Rejected { findings, checks } => {
                print_rejected(&findings, &checks, attempts, &assumptions);
                ExitCode::FAILURE
            }
            Outcome::Waiting { questions } => {
                print_questions(&questions);
                ExitCode::SUCCESS
            }
        },
    }
}

fn print_accepted(
    migration: &str,
    checks: &[CheckStep],
    change_set: Option<&ChangeSet>,
    assumptions: &[String],
) {
    if let Some(cs) = change_set {
        println!("{}", cs.summary);
    }
    print_assumptions(assumptions);
    println!("Accepted \u{2192} migrations/{migration}");
    println!("{}", checks_line(checks));
}

fn print_rejected(
    findings: &[nocodo_types::event::Finding],
    checks: &[CheckStep],
    attempts: u32,
    assumptions: &[String],
) {
    if attempts > 0 {
        println!("Rejected after {attempts} attempt(s):");
    } else {
        println!("Rejected:");
    }
    for f in findings {
        println!("  [{}] {}", f.code, f.message);
    }
    println!("{}", checks_line(checks));
    print_assumptions(assumptions);
}

/// Prints questions with numbered choices; blocking ones are marked.
fn print_questions(questions: &[Question]) {
    println!(
        "{} question(s) waiting. Answer with `nocodo answer q1=<choice number> ...`:",
        questions.len()
    );
    for q in questions {
        let mark = if q.blocking { " (needs an answer)" } else { "" };
        println!("  {}: {}{mark}", q.id, q.text);
        for (i, c) in q.choices.iter().enumerate() {
            let d = if q.default == Some(i + 1) {
                " (default)"
            } else {
                ""
            };
            println!("    {}. {c}{d}", i + 1);
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
