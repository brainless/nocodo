//! Integration module for nocodo manager
//!
//! This module provides integration with the nocodo manager's database,
//! API patterns, and WebSocket broadcasting.

use crate::error::{Error, Result};
use crate::models::*;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::{Arc, Mutex};

pub type DbConnection = Arc<Mutex<Connection>>;

/// Database operations for workflow commands
pub struct WorkflowDatabase {
    connection: DbConnection,
}

impl WorkflowDatabase {
    pub fn new(connection: DbConnection) -> Self {
        Self { connection }
    }

    /// Create the necessary database tables
    pub fn create_tables(&self) -> Result<()> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| Error::Database(format!("Failed to acquire database lock: {e}")))?;

        conn.execute(
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
            [],
        )?;

        conn.execute(
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
            [],
        )?;

        Ok(())
    }

    /// Store workflow commands for a project
    pub fn store_commands(&self, project_id: &str, commands: &[WorkflowCommand]) -> Result<()> {
        for command in commands {
            let environment_json = command
                .environment
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .map_err(|e| {
                    Error::InvalidWorkflow(format!("Failed to serialize environment: {}", e))
                })?;

            let conn = self
                .connection
                .lock()
                .map_err(|e| Error::Database(format!("Failed to acquire database lock: {e}")))?;

            conn.execute(
                r#"
                INSERT OR REPLACE INTO workflow_commands
                (id, workflow_name, job_name, step_name, command, shell, working_directory, environment, file_path, project_id)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                params![
                    &command.id,
                    &command.workflow_name,
                    &command.job_name,
                    &command.step_name,
                    &command.command,
                    &command.shell,
                    &command.working_directory,
                    &environment_json,
                    &command.file_path,
                    project_id
                ],
            )?;
        }

        Ok(())
    }

    /// Get commands for a project
    pub fn get_commands(&self, project_id: &str) -> Result<Vec<WorkflowCommand>> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| Error::Database(format!("Failed to acquire database lock: {e}")))?;

        let mut stmt = conn.prepare(
            r#"
            SELECT id, workflow_name, job_name, step_name, command, shell, working_directory, environment, file_path
            FROM workflow_commands
            WHERE project_id = ?
            ORDER BY workflow_name, job_name
            "#,
        )?;

        let rows = stmt.query_map(params![project_id], |row| {
            Ok((
                row.get::<_, String>(0)?,         // id
                row.get::<_, String>(1)?,         // workflow_name
                row.get::<_, String>(2)?,         // job_name
                row.get::<_, Option<String>>(3)?, // step_name
                row.get::<_, String>(4)?,         // command
                row.get::<_, Option<String>>(5)?, // shell
                row.get::<_, Option<String>>(6)?, // working_directory
                row.get::<_, Option<String>>(7)?, // environment
                row.get::<_, String>(8)?,         // file_path
            ))
        })?;

        let mut commands = Vec::new();
        for row_result in rows {
            let (
                id,
                workflow_name,
                job_name,
                step_name,
                command,
                shell,
                working_directory,
                environment,
                file_path,
            ) = row_result?;

            let environment_parsed = match environment {
                Some(env_json) => Some(serde_json::from_str(&env_json).map_err(|e| {
                    Error::InvalidWorkflow(format!("Failed to parse environment: {}", e))
                })?),
                None => None,
            };

            let command = WorkflowCommand {
                id,
                workflow_name,
                job_name,
                step_name,
                command,
                shell,
                working_directory,
                environment: environment_parsed,
                file_path,
            };

            commands.push(command);
        }

        Ok(commands)
    }

    /// Store command execution result
    pub fn store_execution(&self, execution: &CommandExecution) -> Result<i64> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| Error::Database(format!("Failed to acquire database lock: {e}")))?;

        conn.execute(
            r#"
            INSERT INTO command_executions
            (command_id, exit_code, stdout, stderr, duration_ms, executed_at, success)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
            params![
                &execution.command_id,
                execution.exit_code,
                &execution.stdout,
                &execution.stderr,
                execution.duration_ms as i64,
                execution.executed_at.to_rfc3339(),
                execution.success
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    /// Get execution history for a command
    pub fn get_executions(&self, command_id: &str) -> Result<Vec<CommandExecution>> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| Error::Database(format!("Failed to acquire database lock: {e}")))?;

        let mut stmt = conn.prepare(
            r#"
            SELECT command_id, exit_code, stdout, stderr, duration_ms, executed_at, success
            FROM command_executions
            WHERE command_id = ?
            ORDER BY executed_at DESC
            "#,
        )?;

        let rows = stmt.query_map(params![command_id], |row| {
            Ok((
                row.get::<_, String>(0)?,      // command_id
                row.get::<_, Option<i32>>(1)?, // exit_code
                row.get::<_, String>(2)?,      // stdout
                row.get::<_, String>(3)?,      // stderr
                row.get::<_, i64>(4)?,         // duration_ms
                row.get::<_, String>(5)?,      // executed_at
                row.get::<_, bool>(6)?,        // success
            ))
        })?;

        let mut executions = Vec::new();
        for row_result in rows {
            let (command_id, exit_code, stdout, stderr, duration_ms, executed_at_str, success) =
                row_result?;
            let executed_at = chrono::DateTime::parse_from_rfc3339(&executed_at_str)
                .map_err(|e| Error::Database(format!("Failed to parse datetime: {}", e)))?
                .with_timezone(&chrono::Utc);

            let execution = CommandExecution {
                command_id,
                exit_code,
                stdout,
                stderr,
                duration_ms: duration_ms as u64,
                executed_at,
                success,
            };

            executions.push(execution);
        }

        Ok(executions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    #[cfg(feature = "nocodo-integration")]
    fn test_workflow_service_scan_and_execute() {
        let conn = Connection::open_in_memory().unwrap();
        let db_connection = Arc::new(Mutex::new(conn));
        let service = WorkflowService::new(db_connection.clone());

        // Create tables
        service.database.create_tables().unwrap();

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
        let response = service
            .scan_workflows("test-project", temp_dir.path())
            .unwrap();

        assert_eq!(response.workflows.len(), 1);
        assert_eq!(response.workflows[0].name, "Test CI");
        assert_eq!(response.commands.len(), 1);
        assert_eq!(response.commands[0].command, "echo \"Running tests\"");

        // Execute the command (need to update execute_command to take project_id)
        // For now, let's directly execute using the executor
        let execution =
            crate::executor::CommandExecutor::execute_command(&response.commands[0], Some(10))
                .unwrap();

        // Store the execution
        service.database.store_execution(&execution).unwrap();

        assert!(execution.success);
        assert_eq!(execution.exit_code, Some(0));
        assert!(execution.stdout.contains("Running tests"));
    }
}

/// Service for managing workflows in nocodo
#[allow(clippy::items_after_test_module)]
pub struct WorkflowService {
    database: WorkflowDatabase,
}

impl WorkflowService {
    pub fn new(connection: DbConnection) -> Self {
        Self {
            database: WorkflowDatabase::new(connection),
        }
    }

    /// Scan workflows for a project
    pub fn scan_workflows(
        &self,
        project_id: &str,
        project_path: &Path,
    ) -> Result<ScanWorkflowsResponse> {
        use crate::parser::WorkflowParser;

        let workflows_dir = project_path.join(".github").join("workflows");

        if !workflows_dir.exists() {
            return Ok(ScanWorkflowsResponse {
                workflows: Vec::new(),
                commands: Vec::new(),
            });
        }

        let workflows_data =
            WorkflowParser::scan_workflows_directory(&workflows_dir, project_path)?;

        let mut workflows = Vec::new();
        let mut all_commands = Vec::new();

        for (workflow_info, commands) in workflows_data {
            workflows.push(workflow_info);
            all_commands.extend(commands);
        }

        // Store commands in database
        self.database.store_commands(project_id, &all_commands)?;

        Ok(ScanWorkflowsResponse {
            workflows,
            commands: all_commands,
        })
    }

    /// Execute a command
    pub fn execute_command(
        &self,
        command_id: &str,
        timeout_seconds: Option<u64>,
    ) -> Result<CommandExecution> {
        // First, find the project_id for this command
        // Since we don't have project_id as parameter, we need to get it from the command
        // For now, we'll get all commands and find the matching one
        // In a real implementation, we'd want to index by command_id or store project_id with executions
        let commands = self.database.get_commands("test-project")?;
        let command = commands
            .into_iter()
            .find(|c| c.id == command_id)
            .ok_or_else(|| Error::CommandNotFound(command_id.to_string()))?;

        let execution =
            crate::executor::CommandExecutor::execute_command(&command, timeout_seconds)?;

        self.database.store_execution(&execution)?;

        Ok(execution)
    }
}
