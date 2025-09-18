//! # nocodo-github-actions
//!
//! A library for parsing GitHub Actions workflows and extracting executable commands.
//!
//! This crate provides functionality to:
//! - Parse GitHub Actions workflow YAML files
//! - Extract run commands with their execution context
//! - Execute commands in isolated environments
//! - Integrate with nocodo manager for workflow management

pub mod error;
pub mod models;
pub mod parser;
pub mod executor;
pub mod workflow_tests;

#[cfg(feature = "nocodo-integration")]
pub mod nocodo;

#[cfg(feature = "cli")]
pub mod cli;

/// Re-export commonly used types
pub use error::Error;
pub use models::*;
pub use parser::WorkflowParser;
pub use executor::CommandExecutor;