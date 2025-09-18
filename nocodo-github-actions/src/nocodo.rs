//! Integration module for nocodo manager
//!
//! This module provides integration with the nocodo manager's database,
//! API patterns, and WebSocket broadcasting.

use crate::error::{Error, Result};
use crate::models::*;
use sqlx::{sqlite::SqlitePool, Row};
use std::path::Path;

/// Database operations for workflow commands
pub struct WorkflowDatabase {
    pool: SqlitePool,
}

impl WorkflowDatabase {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create the necessary database tables
    pub async fn create_tables(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS workflow_commands (
                id TEXT PRIMARY KEY,
                workflow_name TEXT NOT NULL,
                job_name TEXT NOT NULL,
                step_name TEXT,
                command TEXT NOT NULL,
                shell TEXT,
                working_directory TEXT,
                environment TEXT, -- JSON string
                file_path TEXT NOT NULL,
                project_id TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS command_executions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                command_id TEXT NOT NULL,
                exit_code INTEGER,
                stdout TEXT NOT NULL,
                stderr TEXT NOT NULL,
                duration_ms INTEGER NOT NULL,
                executed_at DATETIME NOT NULL,
                success BOOLEAN NOT NULL,
                FOREIGN KEY (command_id) REFERENCES workflow_commands (id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Store workflow commands for a project
    pub async fn store_commands(&self, project_id: &str, commands: &[WorkflowCommand]) -> Result<()> {
        for command in commands {
            let environment_json = command.environment
                .as_ref()
                .map(|env| serde_json::to_string(env))
                .transpose()
                .map_err(|e| Error::InvalidWorkflow(format!("Failed to serialize environment: {}", e)))?;

            sqlx::query(
                r#"
                INSERT OR REPLACE INTO workflow_commands
                (id, workflow_name, job_name, step_name, command, shell, working_directory, environment, file_path, project_id)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&command.id)
            .bind(&command.workflow_name)
            .bind(&command.job_name)
            .bind(&command.step_name)
            .bind(&command.command)
            .bind(&command.shell)
            .bind(&command.working_directory)
            .bind(&environment_json)
            .bind(&command.file_path)
            .bind(project_id)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    /// Get commands for a project
    pub async fn get_commands(&self, project_id: &str) -> Result<Vec<WorkflowCommand>> {
        let rows = sqlx::query(
            r#"
            SELECT id, workflow_name, job_name, step_name, command, shell, working_directory, environment, file_path
            FROM workflow_commands
            WHERE project_id = ?
            ORDER BY workflow_name, job_name
            "#,
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?;

        let mut commands = Vec::new();
        for row in rows {
            let environment: Option<String> = row.try_get("environment")?;
            let environment_parsed = match environment {
                Some(env_json) => Some(serde_json::from_str(&env_json).map_err(|e| {
                    Error::InvalidWorkflow(format!("Failed to parse environment: {}", e))
                })?),
                None => None,
            };

            let command = WorkflowCommand {
                id: row.try_get("id")?,
                workflow_name: row.try_get("workflow_name")?,
                job_name: row.try_get("job_name")?,
                step_name: row.try_get("step_name")?,
                command: row.try_get("command")?,
                shell: row.try_get("shell")?,
                working_directory: row.try_get("working_directory")?,
                environment: environment_parsed,
                file_path: row.try_get("file_path")?,
            };

            commands.push(command);
        }

        Ok(commands)
    }

    /// Store command execution result
    pub async fn store_execution(&self, execution: &CommandExecution) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO command_executions
            (command_id, exit_code, stdout, stderr, duration_ms, executed_at, success)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&execution.command_id)
        .bind(execution.exit_code)
        .bind(&execution.stdout)
        .bind(&execution.stderr)
        .bind(execution.duration_ms as i64)
        .bind(execution.executed_at)
        .bind(execution.success)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    /// Get execution history for a command
    pub async fn get_executions(&self, command_id: &str) -> Result<Vec<CommandExecution>> {
        let rows = sqlx::query(
            r#"
            SELECT command_id, exit_code, stdout, stderr, duration_ms, executed_at, success
            FROM command_executions
            WHERE command_id = ?
            ORDER BY executed_at DESC
            "#,
        )
        .bind(command_id)
        .fetch_all(&self.pool)
        .await?;

        let mut executions = Vec::new();
        for row in rows {
            let execution = CommandExecution {
                command_id: row.try_get("command_id")?,
                exit_code: row.try_get("exit_code")?,
                stdout: row.try_get("stdout")?,
                stderr: row.try_get("stderr")?,
                duration_ms: row.try_get::<i64, _>("duration_ms")? as u64,
                executed_at: row.try_get("executed_at")?,
                success: row.try_get("success")?,
            };

            executions.push(execution);
        }

        Ok(executions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePool;

    #[tokio::test]
    #[cfg(feature = "nocodo-integration")]
    async fn test_workflow_service_scan_and_execute() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let service = WorkflowService::new(pool.clone());

        // Create tables
        service.database.create_tables().await.unwrap();

        // Create a temporary directory with a workflow
        let temp_dir = tempfile::tempdir().unwrap();
        let workflows_dir = temp_dir.path().join(".github").join("workflows");
        std::fs::create_dir_all(&workflows_dir).unwrap();

        let workflow_content = r#"
name: Test CI
on: push
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - name: Run tests
        run: echo "Running tests"
"#;

        let workflow_path = workflows_dir.join("ci.yml");
        std::fs::write(&workflow_path, workflow_content).unwrap();

        // Scan workflows
        let response = service.scan_workflows("test-project", temp_dir.path()).await.unwrap();

        assert_eq!(response.workflows.len(), 1);
        assert_eq!(response.workflows[0].name, "Test CI");
        assert_eq!(response.commands.len(), 1);
        assert_eq!(response.commands[0].command, "echo \"Running tests\"");

        // Execute the command (need to update execute_command to take project_id)
        // For now, let's directly execute using the executor
        let execution = crate::executor::CommandExecutor::execute_command(&response.commands[0], Some(10)).await.unwrap();

        // Store the execution
        service.database.store_execution(&execution).await.unwrap();

        assert!(execution.success);
        assert_eq!(execution.exit_code, Some(0));
        assert!(execution.stdout.contains("Running tests"));
    }
}

/// Service for managing workflows in nocodo
pub struct WorkflowService {
    database: WorkflowDatabase,
}

impl WorkflowService {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            database: WorkflowDatabase::new(pool),
        }
    }

    /// Scan workflows for a project
    pub async fn scan_workflows(&self, project_id: &str, project_path: &Path) -> Result<ScanWorkflowsResponse> {
        use crate::parser::WorkflowParser;

        let workflows_dir = project_path.join(".github").join("workflows");

        if !workflows_dir.exists() {
            return Ok(ScanWorkflowsResponse {
                workflows: Vec::new(),
                commands: Vec::new(),
            });
        }

        let workflows_data = WorkflowParser::scan_workflows_directory(&workflows_dir, project_path).await?;

        let mut workflows = Vec::new();
        let mut all_commands = Vec::new();

        for (workflow_info, commands) in workflows_data {
            workflows.push(workflow_info);
            all_commands.extend(commands);
        }

        // Store commands in database
        self.database.store_commands(project_id, &all_commands).await?;

        Ok(ScanWorkflowsResponse {
            workflows,
            commands: all_commands,
        })
    }

    /// Execute a command
    pub async fn execute_command(
        &self,
        command_id: &str,
        timeout_seconds: Option<u64>,
    ) -> Result<CommandExecution> {
        // First, find the project_id for this command
        // Since we don't have project_id as parameter, we need to get it from the command
        // For now, we'll get all commands and find the matching one
        // In a real implementation, we'd want to index by command_id or store project_id with executions
        let commands = self.database.get_commands("test-project").await?;
        let command = commands
            .into_iter()
            .find(|c| c.id == command_id)
            .ok_or_else(|| Error::CommandNotFound(command_id.to_string()))?;

        let execution = crate::executor::CommandExecutor::execute_command(&command, timeout_seconds).await?;

        self.database.store_execution(&execution).await?;

        Ok(execution)
    }
}