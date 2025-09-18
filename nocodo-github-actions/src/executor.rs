use crate::error::{Error, Result};
use crate::models::{CommandExecution, WorkflowCommand};
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::process::Command;

/// Executor for workflow commands
pub struct CommandExecutor;

impl CommandExecutor {
    /// Execute a workflow command
    pub async fn execute_command(
        command: &WorkflowCommand,
        timeout_seconds: Option<u64>,
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

        // Execute with timeout
        let timeout_duration = timeout_seconds
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(300)); // 5 minute default

        let output = match tokio::time::timeout(timeout_duration, cmd.output()).await {
            Ok(result) => result?,
            Err(_) => return Err(Error::Timeout),
        };

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

    /// Execute a command with streaming output (for WebSocket integration)
    pub async fn execute_command_streaming<F>(
        command: &WorkflowCommand,
        timeout_seconds: Option<u64>,
        on_output: F,
    ) -> Result<CommandExecution>
    where
        F: Fn(&str) + Clone + Send + Sync + 'static,
    {
        let start_time = Instant::now();

        let mut cmd = if let Some(shell) = &command.shell {
            let mut c = Command::new(shell);
            c.arg("-c").arg(&command.command);
            c
        } else {
            let mut c = Command::new("sh");
            c.arg("-c").arg(&command.command);
            c
        };

        if let Some(wd) = &command.working_directory {
            cmd.current_dir(wd);
        }

        if let Some(env) = &command.environment {
            for (key, value) in env {
                cmd.env(key, value);
            }
        }

        cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let timeout_duration = timeout_seconds
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(300));

        let mut child = cmd.spawn()?;
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        // Read stdout and stderr concurrently
        let on_output_clone = on_output.clone();
        let stdout_task = tokio::spawn(async move {
            use tokio::io::AsyncBufReadExt;
            let mut reader = tokio::io::BufReader::new(stdout).lines();
            let mut output = String::new();
            while let Ok(Some(line)) = reader.next_line().await {
                output.push_str(&line);
                output.push('\n');
                on_output_clone(&line);
            }
            output
        });

        let stderr_task = tokio::spawn(async move {
            use tokio::io::AsyncBufReadExt;
            let mut reader = tokio::io::BufReader::new(stderr).lines();
            let mut output = String::new();
            while let Ok(Some(line)) = reader.next_line().await {
                output.push_str(&line);
                output.push('\n');
                on_output(&line);
            }
            output
        });

        // Wait for process completion with timeout
        let status_result = tokio::time::timeout(timeout_duration, child.wait()).await;

        let (stdout_output, stderr_output) = tokio::try_join!(stdout_task, stderr_task)?;

        let status = match status_result {
            Ok(result) => result?,
            Err(_) => return Err(Error::Timeout),
        };

        let duration = start_time.elapsed();

        let execution = CommandExecution {
            command_id: command.id.clone(),
            exit_code: status.code(),
            stdout: stdout_output,
            stderr: stderr_output,
            duration_ms: duration.as_millis() as u64,
            executed_at: chrono::Utc::now(),
            success: status.success(),
        };

        Ok(execution)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_execute_simple_command() {
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

        let result = CommandExecutor::execute_command(&command, Some(10)).await.unwrap();

        assert!(result.success);
        assert_eq!(result.exit_code, Some(0));
        assert!(result.stdout.contains("hello world"));
    }
}