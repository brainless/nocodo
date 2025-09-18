use crate::error::Result;
use crate::models::{CommandExecution, WorkflowCommand};
use std::process::{Command, Stdio};
use std::time::Instant;

/// Executor for workflow commands
pub struct CommandExecutor;

impl CommandExecutor {
    /// Execute a workflow command
    pub fn execute_command(
        command: &WorkflowCommand,
        _timeout_seconds: Option<u64>,
    ) -> Result<CommandExecution> {
        let start_time = Instant::now();

        // Prepare the command
        let mut cmd = if let Some(shell) = &command.shell {
            let mut c = Command::new(shell);
            c.arg("-c").arg(&command.command);
            c
        } else {
            // Default to sh if no shell specified
            let mut c = Command::new("sh");
            c.arg("-c").arg(&command.command);
            c
        };

        // Set working directory
        if let Some(wd) = &command.working_directory {
            cmd.current_dir(wd);
        }

        // Set environment variables
        if let Some(env) = &command.environment {
            for (key, value) in env {
                cmd.env(key, value);
            }
        }

        // Configure stdout/stderr capture
        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        // Execute command
        let output = cmd.output()?;

        let duration = start_time.elapsed();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let execution = CommandExecution {
            command_id: command.id.clone(),
            exit_code: output.status.code(),
            stdout,
            stderr,
            duration_ms: duration.as_millis() as u64,
            executed_at: chrono::Utc::now(),
            success: output.status.success(),
        };

        Ok(execution)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_simple_command() {
        let command = WorkflowCommand {
            id: "test".to_string(),
            workflow_name: "test".to_string(),
            job_name: "test".to_string(),
            step_name: Some("echo".to_string()),
            command: "echo 'hello world'".to_string(),
            shell: None,
            working_directory: None,
            environment: None,
            file_path: "test.yml".to_string(),
        };

        let result = CommandExecutor::execute_command(&command, Some(10)).unwrap();

        assert!(result.success);
        assert_eq!(result.exit_code, Some(0));
        assert!(result.stdout.contains("hello world"));
    }
}