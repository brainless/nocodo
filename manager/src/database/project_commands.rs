use crate::error::{AppError, AppResult};
use crate::models::{ProjectCommand, ProjectCommandExecution};
use rusqlite::{params, OptionalExtension};

/// Project Commands database methods
#[allow(dead_code)]
impl super::Database {
    /// Check if a command already exists for a project (by command string)
    pub fn command_exists(
        &self,
        project_id: i64,
        command_str: &str,
    ) -> AppResult<Option<ProjectCommand>> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let command = conn
            .query_row(
                r#"
                SELECT id, project_id, name, description, command, shell, working_directory, environment, timeout_seconds, os_filter, created_at, updated_at
                FROM project_commands
                WHERE project_id = ? AND command = ?
                "#,
                params![project_id, command_str],
                |row| {
                    let environment: Option<String> = row.get(7)?;
                    let environment_parsed = environment.as_ref()
                        .map(|env_json| serde_json::from_str(env_json))
                        .transpose()
                        .map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                7,
                                rusqlite::types::Type::Text,
                                Box::new(e),
                            )
                        })?;

                    let timeout_seconds: Option<i64> = row.get(8)?;
                    let os_filter: Option<String> = row.get(9)?;
                    let os_filter_parsed = os_filter.as_ref()
                        .map(|os_json| serde_json::from_str(os_json))
                        .transpose()
                        .map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                9,
                                rusqlite::types::Type::Text,
                                Box::new(e),
                            )
                        })?;

                    Ok(ProjectCommand {
                        id: row.get(0)?,
                        project_id: row.get(1)?,
                        name: row.get(2)?,
                        description: row.get(3)?,
                        command: row.get(4)?,
                        shell: row.get(5)?,
                        working_directory: row.get(6)?,
                        environment: environment_parsed,
                        timeout_seconds: timeout_seconds.map(|v| v as u64),
                        os_filter: os_filter_parsed,
                        created_at: row.get(10)?,
                        updated_at: row.get(11)?,
                    })
                },
            )
            .optional()?;

        Ok(command)
    }

    /// Create a new project command
    pub fn create_project_command(&self, command: &ProjectCommand) -> AppResult<()> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let environment_json = command
            .environment
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| AppError::Internal(format!("Failed to serialize environment: {}", e)))?;

        let os_filter_json = command
            .os_filter
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| AppError::Internal(format!("Failed to serialize os_filter: {}", e)))?;

        conn.execute(
            r#"
            INSERT INTO project_commands
            (id, project_id, name, description, command, shell, working_directory, environment, timeout_seconds, os_filter, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            params![
                command.id,
                command.project_id,
                command.name,
                command.description,
                command.command,
                command.shell,
                command.working_directory,
                environment_json,
                command.timeout_seconds,
                os_filter_json,
                command.created_at,
                command.updated_at,
            ],
        )?;

        tracing::info!("Created project command: {} ({})", command.name, command.id);
        Ok(())
    }

    /// Get all commands for a project
    pub fn get_project_commands(&self, project_id: i64) -> AppResult<Vec<ProjectCommand>> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let mut stmt = conn.prepare(
            r#"
            SELECT id, project_id, name, description, command, shell, working_directory, environment, timeout_seconds, os_filter, created_at, updated_at
            FROM project_commands
            WHERE project_id = ?
            ORDER BY name ASC
            "#,
        )?;

        let command_iter = stmt.query_map([project_id], |row| {
            let environment: Option<String> = row.get(7)?;
            let environment_parsed = environment
                .as_ref()
                .map(|env_json| serde_json::from_str(env_json))
                .transpose()
                .map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        7,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;

            let timeout_seconds: Option<i64> = row.get(8)?;
            let os_filter: Option<String> = row.get(9)?;
            let os_filter_parsed = os_filter
                .as_ref()
                .map(|os_json| serde_json::from_str(os_json))
                .transpose()
                .map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        9,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;

            Ok(ProjectCommand {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                description: row.get(3)?,
                command: row.get(4)?,
                shell: row.get(5)?,
                working_directory: row.get(6)?,
                environment: environment_parsed,
                timeout_seconds: timeout_seconds.map(|v| v as u64),
                os_filter: os_filter_parsed,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
            })
        })?;

        let mut commands = Vec::new();
        for command in command_iter {
            commands.push(command?);
        }

        Ok(commands)
    }

    /// Get a specific project command by ID
    pub fn get_project_command_by_id(&self, id: &str) -> AppResult<ProjectCommand> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let command = conn
            .query_row(
                r#"
                SELECT id, project_id, name, description, command, shell, working_directory, environment, timeout_seconds, os_filter, created_at, updated_at
                FROM project_commands
                WHERE id = ?
                "#,
                [id],
                |row| {
                    let environment: Option<String> = row.get(7)?;
                    let environment_parsed = environment.as_ref()
                        .map(|env_json| serde_json::from_str(env_json))
                        .transpose()
                        .map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                7,
                                rusqlite::types::Type::Text,
                                Box::new(e),
                            )
                        })?;

                    let timeout_seconds: Option<i64> = row.get(8)?;
                    let os_filter: Option<String> = row.get(9)?;
                    let os_filter_parsed = os_filter.as_ref()
                        .map(|os_json| serde_json::from_str(os_json))
                        .transpose()
                        .map_err(|e| {
                            rusqlite::Error::FromSqlConversionFailure(
                                9,
                                rusqlite::types::Type::Text,
                                Box::new(e),
                            )
                        })?;

                    Ok(ProjectCommand {
                        id: row.get(0)?,
                        project_id: row.get(1)?,
                        name: row.get(2)?,
                        description: row.get(3)?,
                        command: row.get(4)?,
                        shell: row.get(5)?,
                        working_directory: row.get(6)?,
                        environment: environment_parsed,
                        timeout_seconds: timeout_seconds.map(|v| v as u64),
                        os_filter: os_filter_parsed,
                        created_at: row.get(10)?,
                        updated_at: row.get(11)?,
                    })
                },
            )
            .optional()?;

        match command {
            Some(command) => Ok(command),
            None => Err(AppError::NotFound(format!(
                "Project command not found: {}",
                id
            ))),
        }
    }

    /// Update a project command
    pub fn update_project_command(&self, command: &ProjectCommand) -> AppResult<()> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let environment_json = command
            .environment
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| AppError::Internal(format!("Failed to serialize environment: {}", e)))?;

        let os_filter_json = command
            .os_filter
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| AppError::Internal(format!("Failed to serialize os_filter: {}", e)))?;

        let rows_affected = conn.execute(
            r#"
            UPDATE project_commands
            SET name = ?, description = ?, command = ?, shell = ?, working_directory = ?, environment = ?, timeout_seconds = ?, os_filter = ?, updated_at = ?
            WHERE id = ?
            "#,
            params![
                command.name,
                command.description,
                command.command,
                command.shell,
                command.working_directory,
                environment_json,
                command.timeout_seconds,
                os_filter_json,
                command.updated_at,
                command.id,
            ],
        )?;

        if rows_affected == 0 {
            return Err(AppError::NotFound(format!(
                "Project command not found: {}",
                command.id
            )));
        }

        tracing::info!("Updated project command: {} ({})", command.name, command.id);
        Ok(())
    }

    /// Delete a project command
    pub fn delete_project_command(&self, id: &str) -> AppResult<()> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let rows_affected = conn.execute("DELETE FROM project_commands WHERE id = ?", [id])?;

        if rows_affected == 0 {
            return Err(AppError::NotFound(format!(
                "Project command not found: {}",
                id
            )));
        }

        tracing::info!("Deleted project command: {}", id);
        Ok(())
    }

    /// Create a project command execution record
    pub fn create_project_command_execution(
        &self,
        execution: &ProjectCommandExecution,
    ) -> AppResult<i64> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        conn.execute(
            r#"
            INSERT INTO project_command_executions
            (command_id, git_branch, exit_code, stdout, stderr, duration_ms, executed_at, success)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            params![
                execution.command_id,
                execution.git_branch,
                execution.exit_code,
                execution.stdout,
                execution.stderr,
                execution.duration_ms as i64,
                execution.executed_at,
                if execution.success { 1i64 } else { 0i64 }
            ],
        )?;

        let execution_id = conn.last_insert_rowid();
        tracing::info!(
            "Created project command execution: {} for command {}",
            execution_id,
            execution.command_id
        );
        Ok(execution_id)
    }

    /// Get execution history for a project command
    pub fn get_project_command_executions(
        &self,
        command_id: &str,
        limit: i64,
    ) -> AppResult<Vec<ProjectCommandExecution>> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let mut stmt = conn.prepare(
            r#"
            SELECT id, command_id, git_branch, exit_code, stdout, stderr, duration_ms, executed_at, success
            FROM project_command_executions
            WHERE command_id = ?
            ORDER BY executed_at DESC
            LIMIT ?
            "#,
        )?;

        let execution_iter = stmt.query_map(params![command_id, limit], |row| {
            Ok(ProjectCommandExecution {
                id: row.get(0)?,
                command_id: row.get(1)?,
                git_branch: row.get(2)?,
                exit_code: row.get(3)?,
                stdout: row.get(4)?,
                stderr: row.get(5)?,
                duration_ms: row.get::<_, i64>(6)? as u64,
                executed_at: row.get(7)?,
                success: row.get::<_, i64>(8)? != 0,
            })
        })?;

        let mut executions = Vec::new();
        for execution in execution_iter {
            executions.push(execution?);
        }

        Ok(executions)
    }
}
