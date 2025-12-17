use anyhow::{Context, Result};
use codex_core::exec::SandboxType;
use codex_core::sandboxing::{execute_env, ExecEnv};
use codex_process_hardening::pre_main_hardening;
use std::path::Path;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

use crate::bash::BashExecutorTrait;
use crate::bash_permissions::BashPermissions;

#[derive(Clone)]
pub struct BashExecutor {
    permissions: BashPermissions,
    default_timeout: Duration,
}

#[allow(dead_code)]
impl BashExecutor {
    pub fn new(permissions: BashPermissions, default_timeout_secs: u64) -> Result<Self> {
        let default_timeout = Duration::from_secs(default_timeout_secs);

        // Apply process hardening (this is called automatically in Codex binaries)
        // Note: In a real Codex integration, this would be called pre-main
        // For our use case, we'll apply it manually
        pre_main_hardening();

        Ok(Self {
            permissions,
            default_timeout,
        })
    }

    pub async fn execute(
        &self,
        command: &str,
        timeout_secs: Option<u64>,
    ) -> Result<crate::bash::BashExecutionResult> {
        let execution_timeout = timeout_secs
            .map(Duration::from_secs)
            .unwrap_or(self.default_timeout);

        info!("Executing bash command: {}", command);
        debug!("Timeout: {} seconds", execution_timeout.as_secs());

        // Check permissions before execution
        if let Err(denied_reason) = self.permissions.check_command(command) {
            warn!("Command denied: {} - {}", command, denied_reason);
            return Err(anyhow::anyhow!("Command denied: {}", denied_reason));
        }

        // Create exec env for command with current environment
        let exec_env = ExecEnv {
            command: vec!["bash".to_string(), "-c".to_string(), command.to_string()],
            cwd: std::env::current_dir().context("Failed to get current directory")?,
            timeout_ms: Some(execution_timeout.as_millis() as u64),
            env: std::env::vars().collect::<std::collections::HashMap<_, _>>(),
            sandbox: SandboxType::None,
            with_escalated_permissions: Some(false),
            justification: None,
            arg0: None,
        };

        // Create sandbox policy (using read-only policy for safety)
        let sandbox_policy = codex_core::protocol::SandboxPolicy::ReadOnly;

        // Execute with timeout using Codex's execute_env
        let execution_result = timeout(execution_timeout, async {
            execute_env(&exec_env, &sandbox_policy, None).await
        })
        .await;

        match execution_result {
            Ok(Ok(result)) => {
                let stdout = result.stdout.text;
                let stderr = result.stderr.text;
                let exit_code = result.exit_code;

                info!(
                    "Command completed - Exit code: {}, stdout_len: {}, stderr_len: {}",
                    exit_code,
                    stdout.len(),
                    stderr.len()
                );

                Ok(crate::bash::BashExecutionResult {
                    stdout,
                    stderr,
                    exit_code,
                    timed_out: false,
                })
            }
            Ok(Err(e)) => {
                error!("Command execution failed: {}", e);
                Err(anyhow::anyhow!("Command execution failed: {}", e))
            }
            Err(_) => {
                warn!(
                    "Command timed out after {} seconds",
                    execution_timeout.as_secs()
                );
                Ok(crate::bash::BashExecutionResult {
                    stdout: String::new(),
                    stderr: format!(
                        "Command timed out after {} seconds",
                        execution_timeout.as_secs()
                    ),
                    exit_code: 124, // Standard timeout exit code
                    timed_out: true,
                })
            }
        }
    }

    pub async fn execute_with_cwd(
        &self,
        command: &str,
        working_dir: &Path,
        timeout_secs: Option<u64>,
    ) -> Result<crate::bash::BashExecutionResult> {
        let execution_timeout = timeout_secs
            .map(Duration::from_secs)
            .unwrap_or(self.default_timeout);

        info!("Executing bash command in {:?}: {}", working_dir, command);

        // Check permissions before execution
        if let Err(denied_reason) = self.permissions.check_command(command) {
            warn!("Command denied: {} - {}", command, denied_reason);
            return Err(anyhow::anyhow!("Command denied: {}", denied_reason));
        }

        // Check working directory permissions
        if let Err(denied_reason) = self.permissions.check_working_directory(working_dir) {
            warn!(
                "Working directory denied: {:?} - {}",
                working_dir, denied_reason
            );
            return Err(anyhow::anyhow!(
                "Working directory denied: {}",
                denied_reason
            ));
        }

        // Create exec env for command with custom working directory
        let exec_env = ExecEnv {
            command: vec!["bash".to_string(), "-c".to_string(), command.to_string()],
            cwd: working_dir.to_path_buf(),
            timeout_ms: Some(execution_timeout.as_millis() as u64),
            env: std::env::vars().collect::<std::collections::HashMap<_, _>>(),
            sandbox: SandboxType::None,
            with_escalated_permissions: Some(false),
            justification: None,
            arg0: None,
        };

        // Create sandbox policy (using read-only policy for safety)
        let sandbox_policy = codex_core::protocol::SandboxPolicy::ReadOnly;

        // Execute with timeout using Codex's execute_env
        let execution_result = timeout(execution_timeout, async {
            execute_env(&exec_env, &sandbox_policy, None).await
        })
        .await;

        match execution_result {
            Ok(Ok(result)) => {
                let stdout = result.stdout.text;
                let stderr = result.stderr.text;
                let exit_code = result.exit_code;

                info!(
                    "Command completed in {:?} - Exit code: {}, stdout_len: {}, stderr_len: {}",
                    working_dir,
                    exit_code,
                    stdout.len(),
                    stderr.len()
                );

                Ok(crate::bash::BashExecutionResult {
                    stdout,
                    stderr,
                    exit_code,
                    timed_out: false,
                })
            }
            Ok(Err(e)) => {
                error!("Command execution failed: {}", e);
                Err(anyhow::anyhow!("Command execution failed: {}", e))
            }
            Err(_) => {
                warn!(
                    "Command timed out after {} seconds",
                    execution_timeout.as_secs()
                );
                Ok(crate::bash::BashExecutionResult {
                    stdout: String::new(),
                    stderr: format!(
                        "Command timed out after {} seconds",
                        execution_timeout.as_secs()
                    ),
                    exit_code: 124,
                    timed_out: true,
                })
            }
        }
    }

    pub fn get_permissions(&self) -> &BashPermissions {
        &self.permissions
    }

    pub fn update_permissions(&mut self, permissions: BashPermissions) {
        self.permissions = permissions;
        info!("Updated bash executor permissions");
    }
}

// Implement the BashExecutorTrait for manager-tools integration
impl BashExecutorTrait for BashExecutor {
    fn execute_with_cwd(
        &self,
        command: &str,
        working_dir: &std::path::Path,
        timeout_secs: Option<u64>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<crate::bash::BashExecutionResult>> + Send + '_>,
    > {
        let command = command.to_string();
        let working_dir = working_dir.to_path_buf();
        let executor = self.clone();
        Box::pin(async move {
            executor
                .execute_with_cwd(&command, &working_dir, timeout_secs)
                .await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bash_permissions::{BashPermissions, PermissionRule};

    #[tokio::test]
    async fn test_basic_execution() {
        let permissions = BashPermissions::new(vec![PermissionRule::allow("echo*").unwrap()]);

        let executor = BashExecutor::new(permissions, 5).unwrap();
        let result = executor
            .execute("echo 'Hello, World!'", None)
            .await
            .unwrap();

        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.contains("Hello, World!"));
        assert!(!result.timed_out);
    }

    #[tokio::test]
    async fn test_command_timeout() {
        let permissions = BashPermissions::new(vec![PermissionRule::allow("sleep*").unwrap()]);

        let executor = BashExecutor::new(permissions, 2).unwrap();
        let result = executor.execute("sleep 5", None).await.unwrap();

        assert_eq!(result.exit_code, 124);
        assert!(result.timed_out);
        assert!(result.stderr.contains("timed out"));
    }

    #[tokio::test]
    async fn test_permission_denied() {
        let permissions = BashPermissions::new(vec![PermissionRule::allow("echo*").unwrap()]);

        let executor = BashExecutor::new(permissions, 5).unwrap();
        let result = executor.execute("rm -rf /", None).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("denied"));
    }

    #[tokio::test]
    async fn test_bash_executor_creation() {
        let executor = BashExecutor::new(BashPermissions::default(), 30).unwrap();

        assert_eq!(executor.default_timeout.as_secs(), 30);
    }

    #[tokio::test]
    async fn test_bash_executor_command_with_stderr() {
        let permissions = BashPermissions::new(vec![PermissionRule::allow("echo*").unwrap()]);

        let executor = BashExecutor::new(permissions, 10).unwrap();

        // This command writes to stderr
        let result = executor
            .execute("echo 'Error message' >&2", None)
            .await
            .unwrap();

        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.is_empty());
        assert!(result.stderr.contains("Error message"));
    }

    #[tokio::test]
    async fn test_bash_executor_nonexistent_command() {
        let permissions = BashPermissions::new(vec![PermissionRule::allow("*").unwrap()]);

        let executor = BashExecutor::new(permissions, 10).unwrap();

        let result = executor
            .execute("nonexistent_command_12345", None)
            .await
            .unwrap();

        assert!(result.exit_code != 0);
        assert!(result.stderr.contains("not found") || result.stderr.contains("command not found"));
    }

    #[tokio::test]
    async fn test_bash_executor_working_directory() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let temp_dir_str = temp_dir.path().to_string_lossy().to_string();

        let permissions = BashPermissions::new(vec![
            PermissionRule::allow("ls*").unwrap(),
            PermissionRule::allow("test*").unwrap(),
        ])
        .with_allowed_working_dirs(vec![temp_dir_str]);

        let executor = BashExecutor::new(permissions, 10).unwrap();

        // Create a test file in the project directory
        let test_file = temp_dir.path().join("test_file.txt");
        std::fs::write(&test_file, "test content").unwrap();

        // List files in the project directory
        let result = executor
            .execute_with_cwd("ls test_file.txt", temp_dir.path(), None)
            .await
            .unwrap();

        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.contains("test_file.txt"));
    }

    #[tokio::test]
    async fn test_bash_executor_git_commands() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let temp_dir_str = temp_dir.path().to_string_lossy().to_string();

        let permissions = BashPermissions::new(vec![PermissionRule::allow("git*").unwrap()])
            .with_allowed_working_dirs(vec![temp_dir_str]);

        let executor = BashExecutor::new(permissions, 10).unwrap();

        // Initialize git repository
        let init_result = executor
            .execute_with_cwd("git init", temp_dir.path(), None)
            .await
            .unwrap();
        assert_eq!(init_result.exit_code, 0);

        // Check git status
        let status_result = executor
            .execute_with_cwd("git status", temp_dir.path(), None)
            .await
            .unwrap();
        assert_eq!(status_result.exit_code, 0);

        // Create a test file and add it
        let test_file = temp_dir.path().join("test.txt");
        std::fs::write(&test_file, "test").unwrap();

        let add_result = executor
            .execute_with_cwd("git add test.txt", temp_dir.path(), None)
            .await
            .unwrap();
        assert_eq!(add_result.exit_code, 0);
    }

    #[tokio::test]
    async fn test_bash_executor_cargo_commands() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let temp_dir_str = temp_dir.path().to_string_lossy().to_string();

        let permissions = BashPermissions::new(vec![
            PermissionRule::allow("cargo*").unwrap(),
            PermissionRule::allow("ls*").unwrap(),
        ])
        .with_allowed_working_dirs(vec![temp_dir_str]);

        let executor = BashExecutor::new(permissions, 10).unwrap();

        // Initialize a minimal Rust project
        let init_result = executor
            .execute_with_cwd("cargo init --bin test_project", temp_dir.path(), None)
            .await
            .unwrap();
        assert_eq!(init_result.exit_code, 0, "stderr: {}", init_result.stderr);

        // Check if project was created
        let check_result = executor
            .execute_with_cwd("ls test_project", temp_dir.path(), None)
            .await
            .unwrap();
        assert_eq!(check_result.exit_code, 0);
        assert!(check_result.stdout.contains("Cargo.toml"));
    }

    #[tokio::test]
    async fn test_bash_executor_permissions_get_set() {
        let mut executor = BashExecutor::new(BashPermissions::default(), 10).unwrap();

        // Get initial permissions
        let initial_perms = executor.get_permissions();
        assert!(initial_perms.is_command_allowed("git status"));

        // Update permissions to block git
        let new_perms = BashPermissions::new(vec![PermissionRule::deny("git*").unwrap()]);

        executor.update_permissions(new_perms);

        // Check that git is now blocked
        let updated_perms = executor.get_permissions();
        assert!(!updated_perms.is_command_allowed("git status"));
    }

    #[tokio::test]
    async fn test_bash_executor_complex_command() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let temp_dir_str = temp_dir.path().to_string_lossy().to_string();

        let permissions = BashPermissions::new(vec![
            PermissionRule::allow("cat*").unwrap(),
            PermissionRule::allow("grep*").unwrap(),
            PermissionRule::allow("wc*").unwrap(),
        ])
        .with_allowed_working_dirs(vec![temp_dir_str]);

        let executor = BashExecutor::new(permissions, 10).unwrap();

        // Create a test file
        let test_file = temp_dir.path().join("test.txt");
        std::fs::write(&test_file, "hello\nworld\ntest").unwrap();

        // Run a complex command with pipes and redirects
        let result = executor
            .execute_with_cwd("cat test.txt | grep hello | wc -l", temp_dir.path(), None)
            .await
            .unwrap();

        assert_eq!(result.exit_code, 0);
        assert!(result.stdout.trim().parse::<u32>().unwrap() > 0);
    }

    #[tokio::test]
    async fn test_bash_executor_environment_variables() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let temp_dir_str = temp_dir.path().to_string_lossy().to_string();

        let permissions = BashPermissions::new(vec![PermissionRule::allow("echo*").unwrap()])
            .with_allowed_working_dirs(vec![temp_dir_str]);

        let executor = BashExecutor::new(permissions, 10).unwrap();

        // Test environment variable usage
        let result = executor
            .execute_with_cwd("echo $HOME", temp_dir.path(), None)
            .await
            .unwrap();

        assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);
        assert!(
            !result.stdout.trim().is_empty(),
            "stdout was empty: {}",
            result.stdout
        );
    }
}
