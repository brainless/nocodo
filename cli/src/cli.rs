use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::{commands, error::CliError};
use commands::*;

#[derive(Debug, Parser)]
#[command(name = "nocodo")]
#[command(about = "AI-powered development assistant CLI with guardrails")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(author = env!("CARGO_PKG_AUTHORS"))]
#[command(long_about = None)]
pub struct Cli {
    /// Enable verbose logging
    #[arg(long, global = true)]
    pub verbose: bool,

    /// Configuration file path
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,

    /// Project path to work with
    #[arg(long, global = true)]
    pub project_path: Option<PathBuf>,

    /// Subcommands
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Analyze project structure and provide recommendations
    Analyze {
        /// Path to analyze (defaults to current directory)
        #[arg(short, long)]
        path: Option<PathBuf>,

        /// Output format
        #[arg(short, long, value_enum)]
        format: Option<OutputFormat>,
    },

    /// Generate context-aware prompt for AI tools
    Prompt {
        /// Intent for the prompt
        #[arg(short, long)]
        intent: String,

        /// Template to use
        #[arg(short, long)]
        template: Option<String>,
    },

    /// Validate code against project guardrails
    Validate {
        /// File to validate
        #[arg(short, long)]
        file: PathBuf,

        /// Programming language (auto-detected if not specified)
        #[arg(short, long)]
        language: Option<String>,
    },

    /// Initialize new project with nocodo support
    Init {
        /// Project template to use
        #[arg(short, long)]
        template: Option<String>,

        /// Path where to create the project
        #[arg(short, long)]
        path: PathBuf,
    },

    /// Project structure operations
    Structure {
        #[command(subcommand)]
        action: StructureCommands,
    },

    /// Configuration management
    Config {
        #[command(subcommand)]
        action: ConfigCommands,
    },

    /// Execute AI coding session with enhanced context
    Session {
        /// AI tool to use (e.g., claude, gemini, openai)
        tool: String,

        /// Prompt for the AI tool
        prompt: String,
    },

    /// Show version information
    Version,
}

#[derive(Debug, Subcommand)]
pub enum StructureCommands {
    /// Display project tree
    Tree {
        /// Maximum depth to display
        #[arg(short, long)]
        depth: Option<usize>,
    },

    /// List project files
    List {
        /// File pattern to match
        #[arg(short, long)]
        pattern: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommands {
    /// Show current configuration
    Show,

    /// Set configuration value
    Set {
        /// Configuration key
        key: String,

        /// Configuration value
        value: String,
    },

    /// Get configuration value
    Get {
        /// Configuration key
        key: String,
    },

    /// Initialize default configuration
    Init,
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum OutputFormat {
    Json,
    Yaml,
    Text,
}

impl Cli {
    pub async fn run(&self) -> Result<(), CliError> {
        match &self.command {
            Some(Commands::Analyze { path, format }) => self.handle_analyze(path, format).await,
            Some(Commands::Prompt { intent, template }) => {
                self.handle_prompt(intent, template).await
            }
            Some(Commands::Validate { file, language }) => {
                self.handle_validate(file, language).await
            }
            Some(Commands::Init { template, path }) => self.handle_init(template, path).await,
            Some(Commands::Structure { action }) => self.handle_structure(action).await,
            Some(Commands::Config { action }) => self.handle_config(action).await,
            Some(Commands::Session { tool, prompt }) => self.handle_session(tool, prompt).await,
            Some(Commands::Version) => self.handle_version().await,
            None => {
                // No subcommand provided, show help
                println!("nocodo CLI - AI-powered development assistant with guardrails");
                println!("Run 'nocodo --help' for usage information.");
                Ok(())
            }
        }
    }

    async fn handle_analyze(
        &self,
        path: &Option<PathBuf>,
        format: &Option<OutputFormat>,
    ) -> Result<(), CliError> {
        analyze_project(path, format).await
    }

    async fn handle_prompt(&self, intent: &str, template: &Option<String>) -> Result<(), CliError> {
        generate_prompt(intent, template).await
    }

    async fn handle_validate(
        &self,
        file: &PathBuf,
        language: &Option<String>,
    ) -> Result<(), CliError> {
        validate_code(file, language).await
    }

    async fn handle_init(&self, template: &Option<String>, path: &PathBuf) -> Result<(), CliError> {
        init_project(template, path).await
    }

    async fn handle_structure(&self, action: &StructureCommands) -> Result<(), CliError> {
        handle_structure_command(action).await
    }

    async fn handle_config(&self, action: &ConfigCommands) -> Result<(), CliError> {
        handle_config_command(action).await
    }

    async fn handle_session(&self, tool: &str, prompt: &str) -> Result<(), CliError> {
        execute_ai_session(tool, prompt).await
    }

    async fn handle_version(&self) -> Result<(), CliError> {
        println!("nocodo CLI version: {}", env!("CARGO_PKG_VERSION"));
        println!("Author: {}", env!("CARGO_PKG_AUTHORS"));
        println!("Description: {}", env!("CARGO_PKG_DESCRIPTION"));
        Ok(())
    }
}
