use crate::tool_error::ToolError;
use anyhow::Result;
use manager_models::{BashRequest, BashResponse, ToolErrorResponse, ToolResponse};
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Bash execution result type (re-exported from manager)
#[derive(Debug, Clone)]
pub struct BashExecutionResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub timed_out: bool,
}

/// Bash executor trait to avoid circular dependency
pub trait BashExecutorTrait {
    fn execute_with_cwd(
        &self,
        command: &str,
        working_dir: &Path,
        timeout_secs: Option<u64>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<BashExecutionResult>> + Send + '_>>;
}

pub async fn execute_bash(
    base_path: &Path,
    bash_executor: Option<&(dyn BashExecutorTrait + Send + Sync)>,
    request: BashRequest,
) -> Result<ToolResponse> {
    // Check if bash executor is available
    let bash_executor = match bash_executor {
        Some(executor) => executor,
        None => {
            return Ok(ToolResponse::Error(ToolErrorResponse {
                tool: "bash".to_string(),
                error: "BashExecutorNotAvailable".to_string(),
                message: "Bash executor is not configured".to_string(),
            }));
        }
    };

    let start_time = Instant::now();

    // Determine working directory
    let working_dir = if let Some(dir) = &request.working_dir {
        // Validate and resolve the working directory
        match validate_and_resolve_path(base_path, dir) {
            Ok(path) => path,
            Err(e) => {
                return Ok(ToolResponse::Error(ToolErrorResponse {
                    tool: "bash".to_string(),
                    error: "InvalidWorkingDirectory".to_string(),
                    message: format!("Invalid working directory '{}': {}", dir, e),
                }));
            }
        }
    } else {
        base_path.to_path_buf()
    };

    // Execute the command
    let result = bash_executor
        .execute_with_cwd(&request.command, &working_dir, request.timeout_secs)
        .await;

    let execution_time = start_time.elapsed().as_secs_f64();

    match result {
        Ok(bash_result) => Ok(ToolResponse::Bash(BashResponse {
            command: request.command,
            working_dir: request.working_dir,
            stdout: bash_result.stdout,
            stderr: bash_result.stderr,
            exit_code: bash_result.exit_code,
            timed_out: bash_result.timed_out,
            execution_time_secs: execution_time,
        })),
        Err(e) => Ok(ToolResponse::Error(ToolErrorResponse {
            tool: "bash".to_string(),
            error: "BashExecutionError".to_string(),
            message: format!("Failed to execute bash command: {}", e),
        })),
    }
}

/// Validate and resolve a path relative to the base path
fn validate_and_resolve_path(base_path: &Path, path: &str) -> Result<PathBuf> {
    let input_path = Path::new(path);

    // Normalize the input path to handle . and .. components
    let normalized_input = normalize_path(input_path)?;

    // Handle absolute paths
    if normalized_input.is_absolute() {
        // If the absolute path equals our base path, allow it
        let canonical_input = match normalized_input.canonicalize() {
            Ok(path) => path,
            Err(_) => normalized_input.to_path_buf(), // Fallback if it doesn't exist yet
        };

        let canonical_base = match base_path.canonicalize() {
            Ok(path) => path,
            Err(_) => base_path.to_path_buf(),
        };

        // Security check: ensure the path is within or equals the base directory
        if canonical_input == canonical_base || canonical_input.starts_with(&canonical_base) {
            return Ok(canonical_input);
        } else {
            return Err(ToolError::InvalidPath(format!(
                "Absolute path '{}' is outside the allowed directory '{}'",
                path,
                base_path.display()
            ))
            .into());
        }
    }

    // Handle relative paths
    let target_path = if normalized_input == Path::new(".") {
        base_path.to_path_buf()
    } else {
        base_path.join(&normalized_input)
    };

    // Canonicalize the path to resolve any remaining relative components
    let canonical_path = match target_path.canonicalize() {
        Ok(path) => path,
        Err(_) => target_path, // Fallback to non-canonical path if it doesn't exist
    };

    // Security check: ensure the path is within the base directory
    if !canonical_path.starts_with(base_path) {
        return Err(ToolError::InvalidPath(format!(
            "Path '{}' resolves to location outside the allowed directory",
            path
        ))
        .into());
    }

    Ok(canonical_path)
}

/// Normalize a path by resolving . and .. components while preventing directory traversal
fn normalize_path(path: &Path) -> Result<PathBuf> {
    let mut components = Vec::new();

    for component in path.components() {
        match component {
            std::path::Component::Prefix(_) | std::path::Component::RootDir => {
                // For absolute paths, keep the prefix/root
                components.push(component);
            }
            std::path::Component::CurDir => {
                // Skip current directory components
                continue;
            }
            std::path::Component::ParentDir => {
                // Prevent directory traversal attacks
                if components.is_empty()
                    || matches!(components.last(), Some(std::path::Component::ParentDir))
                {
                    return Err(ToolError::InvalidPath(format!(
                        "Invalid path '{}': contains directory traversal",
                        path.display()
                    ))
                    .into());
                }
                // Remove the last component (go up one level)
                components.pop();
            }
            std::path::Component::Normal(_name) => {
                components.push(component);
            }
        }
    }

    // Reconstruct the path from components
    let mut result = PathBuf::new();
    for component in components {
        result.push(component);
    }

    Ok(result)
}
