use crate::filesystem::path_utils::validate_and_resolve_path;
use crate::types::{BashRequest, BashResponse, ToolErrorResponse, ToolResponse};
use anyhow::Result;
use std::path::Path;
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
