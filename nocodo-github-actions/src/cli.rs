//! Command-line interface for nocodo-github-actions

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// CLI for GitHub Actions workflow parsing and execution
#[derive(Parser)]
#[command(name = "nocodo-github-actions")]
#[command(about = "Parse and execute GitHub Actions workflows")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Parse workflow files and extract commands
    Parse {
        /// Path to workflow file or directory containing workflows
        #[arg(short, long)]
        path: PathBuf,

        /// Output format (json, yaml)
        #[arg(short, long, default_value = "json")]
        format: String,
    },
    /// Execute a command from a workflow
    Execute {
        /// Path to workflow file
        #[arg(short, long)]
        workflow: PathBuf,

        /// Job name
        #[arg(short, long)]
        job: String,

        /// Step index (0-based)
        #[arg(short, long)]
        step: usize,

        /// Working directory for execution
        #[arg(short, long)]
        working_dir: Option<PathBuf>,
    },
}

pub async fn run() -> crate::error::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Parse { path, format } => {
            if path.is_file() {
                let (info, commands) = crate::parser::WorkflowParser::parse_workflow_file(
                    &path,
                    path.parent().unwrap_or(&PathBuf::from(".")),
                )?;
                match format.as_str() {
                    "json" => println!("{}", serde_json::to_string_pretty(&(info, commands))?),
                    "yaml" => println!("{}", serde_yaml::to_string(&(info, commands))?),
                    _ => eprintln!("Unsupported format: {}", format),
                }
            } else if path.is_dir() {
                let workflows =
                    crate::parser::WorkflowParser::scan_workflows_directory(&path, &path)?;
                match format.as_str() {
                    "json" => println!("{}", serde_json::to_string_pretty(&workflows)?),
                    "yaml" => println!("{}", serde_yaml::to_string(&workflows)?),
                    _ => eprintln!("Unsupported format: {}", format),
                }
            } else {
                eprintln!("Path does not exist: {}", path.display());
            }
        }
        Commands::Execute {
            workflow,
            job,
            step,
            working_dir,
        } => {
            let (_info, commands) = crate::parser::WorkflowParser::parse_workflow_file(
                &workflow,
                workflow.parent().unwrap_or(&PathBuf::from(".")),
            )?;

            let command = commands
                .into_iter()
                .find(|c| c.job_name == job && c.id.ends_with(&format!("_{}", step)))
                .ok_or_else(|| crate::error::Error::CommandNotFound(format!("{}/{}", job, step)))?;

            let mut cmd = command.clone();
            if let Some(wd) = working_dir {
                cmd.working_directory = Some(wd.to_string_lossy().to_string());
            }

            let execution = crate::executor::CommandExecutor::execute_command(&cmd, None)?;

            println!("Exit code: {:?}", execution.exit_code);
            println!("Success: {}", execution.success);
            println!("Duration: {}ms", execution.duration_ms);
            if !execution.stdout.is_empty() {
                println!("--- STDOUT ---");
                println!("{}", execution.stdout);
            }
            if !execution.stderr.is_empty() {
                println!("--- STDERR ---");
                println!("{}", execution.stderr);
            }
        }
    }

    Ok(())
}
