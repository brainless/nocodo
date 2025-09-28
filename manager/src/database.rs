use crate::error::{AppError, AppResult};
use crate::models::{
    AiSession, AiSessionResult, LlmAgentMessage, LlmAgentSession, LlmAgentToolCall, Project,
    ProjectComponent,
};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub type DbConnection = Arc<Mutex<Connection>>;

pub struct Database {
    connection: DbConnection,
}

impl Database {
    pub fn new(db_path: &PathBuf) -> AppResult<Self> {
        // Ensure the database directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(db_path)?;

        // Enable foreign key constraints (SQLite3 has them disabled by default)
        conn.execute("PRAGMA foreign_keys = ON", [])?;

        let database = Database {
            connection: Arc::new(Mutex::new(conn)),
        };

        database.run_migrations()?;

        Ok(database)
    }

    #[allow(dead_code)]
    pub fn connection(&self) -> DbConnection {
        Arc::clone(&self.connection)
    }

    fn run_migrations(&self) -> AppResult<()> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        // Create projects table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                path TEXT NOT NULL,
                language TEXT,
                framework TEXT,
                status TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                technologies TEXT
            )",
            [],
        )?;

        // Create an index on the name for faster lookups
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_projects_name ON projects(name)",
            [],
        )?;

        // Create AI sessions table - now links to Work and Message instead of storing prompt directly
        conn.execute(
            "CREATE TABLE IF NOT EXISTS ai_sessions (
                id TEXT PRIMARY KEY,
                work_id TEXT NOT NULL,
                message_id TEXT NOT NULL,
                tool_name TEXT NOT NULL,
                status TEXT NOT NULL,
                project_context TEXT,
                started_at INTEGER NOT NULL,
                ended_at INTEGER,
                FOREIGN KEY (work_id) REFERENCES works (id),
                FOREIGN KEY (message_id) REFERENCES work_messages (id)
            )",
            [],
        )?;

        // Create project components table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS project_components (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                name TEXT NOT NULL,
                path TEXT NOT NULL,
                language TEXT NOT NULL,
                framework TEXT,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (project_id) REFERENCES projects (id)
            )",
            [],
        )?;

        // Index for project components by project
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_components_project_id ON project_components(project_id)",
            [],
        )?;

        // Create an index on work_id and message_id for faster lookups
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_sessions_work_id ON ai_sessions(work_id)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_sessions_message_id ON ai_sessions(message_id)",
            [],
        )?;

        // Create AI session results table to track AI responses
        conn.execute(
            "CREATE TABLE IF NOT EXISTS ai_session_results (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                response_message_id TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                completed_at INTEGER,
                FOREIGN KEY (session_id) REFERENCES ai_sessions (id),
                FOREIGN KEY (response_message_id) REFERENCES work_messages (id)
            )",
            [],
        )?;

        // Index for results by session
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_session_results_session_id ON ai_session_results(session_id)",
            [],
        )?;

        // Create AI session outputs table for one-shot output capture
        conn.execute(
            "CREATE TABLE IF NOT EXISTS ai_session_outputs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                content TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (session_id) REFERENCES ai_sessions (id)
            )",
            [],
        )?;

        // Index for outputs by session
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_outputs_session_id ON ai_session_outputs(session_id)",
            [],
        )?;

        // Create works table (sessions/conversations)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS works (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                project_id TEXT,
                tool_name TEXT,
                status TEXT NOT NULL DEFAULT 'active',
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                FOREIGN KEY (project_id) REFERENCES projects (id)
            )",
            [],
        )?;

        // Add tool_name column if it doesn't exist (migration for existing databases)
        conn.execute("ALTER TABLE works ADD COLUMN tool_name TEXT", [])
            .or_else(|e| {
                // Ignore error if column already exists
                if e.to_string().contains("duplicate column name") {
                    Ok(0)
                } else {
                    Err(e)
                }
            })?;

        // Create work messages with content types and history
        conn.execute(
            "CREATE TABLE IF NOT EXISTS work_messages (
                id TEXT PRIMARY KEY,
                work_id TEXT NOT NULL,
                content TEXT NOT NULL,
                content_type TEXT NOT NULL CHECK (content_type IN ('text', 'markdown', 'json', 'code')),
                code_language TEXT, -- Only for code content type
                author_type TEXT NOT NULL CHECK (author_type IN ('user', 'ai')),
                author_id TEXT, -- User ID or AI tool identifier
                sequence_order INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (work_id) REFERENCES works (id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Index for efficient history retrieval
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_work_messages_work_sequence ON work_messages(work_id, sequence_order)",
            [],
        )?;

        // Create LLM agent sessions table for direct LLM integration
        conn.execute(
            "CREATE TABLE IF NOT EXISTS llm_agent_sessions (
                id TEXT PRIMARY KEY,
                work_id TEXT NOT NULL,
                provider TEXT NOT NULL,
                model TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'running',
                system_prompt TEXT,
                started_at INTEGER NOT NULL,
                ended_at INTEGER,
                FOREIGN KEY (work_id) REFERENCES works (id)
            )",
            [],
        )?;

        // Index for LLM agent sessions by work_id
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_llm_agent_sessions_work_id ON llm_agent_sessions(work_id)",
            [],
        )?;

        // Create LLM agent messages table for conversation history
        conn.execute(
            "CREATE TABLE IF NOT EXISTS llm_agent_messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system', 'tool')),
                content TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (session_id) REFERENCES llm_agent_sessions (id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Index for LLM agent messages by session_id and created_at
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_llm_agent_messages_session_created ON llm_agent_messages(session_id, created_at)",
            [],
        )?;

        // Create LLM agent tool calls table for tool execution tracking
        conn.execute(
            "CREATE TABLE IF NOT EXISTS llm_agent_tool_calls (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                message_id INTEGER,
                tool_name TEXT NOT NULL,
                request TEXT NOT NULL,
                response TEXT,
                status TEXT NOT NULL DEFAULT 'pending',
                created_at INTEGER NOT NULL,
                completed_at INTEGER,
                FOREIGN KEY (session_id) REFERENCES llm_agent_sessions (id) ON DELETE CASCADE,
                FOREIGN KEY (message_id) REFERENCES llm_agent_messages (id) ON DELETE SET NULL
            )",
            [],
        )?;

        // Index for LLM agent tool calls by session_id
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_llm_agent_tool_calls_session_id ON llm_agent_tool_calls(session_id)",
            [],
        )?;

        // Add new columns for enhanced tool call tracking (migration for issue #107)
        // Add execution_time_ms column
        let _ = conn.execute(
            "ALTER TABLE llm_agent_tool_calls ADD COLUMN execution_time_ms INTEGER",
            [],
        ); // Ignore error if column already exists

        // Add progress_updates column
        let _ = conn.execute(
            "ALTER TABLE llm_agent_tool_calls ADD COLUMN progress_updates TEXT",
            [],
        ); // Ignore error if column already exists

        // Add error_details column
        let _ = conn.execute(
            "ALTER TABLE llm_agent_tool_calls ADD COLUMN error_details TEXT",
            [],
        ); // Ignore error if column already exists

        // Add indexes for performance
        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_tool_calls_session_status ON llm_agent_tool_calls(session_id, status)",
            [],
        ); // Ignore error if index already exists

        let _ = conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_tool_calls_created_at ON llm_agent_tool_calls(created_at)",
            [],
        ); // Ignore error if index already exists

        // GitHub Actions workflow tables
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
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (project_id) REFERENCES projects (id) ON DELETE CASCADE
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
                FOREIGN KEY (command_id) REFERENCES workflow_commands (id) ON DELETE CASCADE
            )
            "#,
            [],
        )?;

        // Indexes for workflow tables
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_workflow_commands_project_id ON workflow_commands(project_id)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_command_executions_command_id ON command_executions(command_id)",
            [],
        )?;

        tracing::info!("Database migrations completed");
        Ok(())
    }

    pub fn get_all_projects(&self) -> AppResult<Vec<Project>> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let mut stmt = conn.prepare(
            "SELECT id, name, path, language, framework, status, created_at, updated_at, technologies 
             FROM projects ORDER BY created_at DESC",
        )?;

        let project_iter = stmt.query_map([], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                language: row.get(3)?,
                framework: row.get(4)?,
                status: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
                technologies: row.get(8)?,
            })
        })?;

        let mut projects = Vec::new();
        for project in project_iter {
            projects.push(project?);
        }

        Ok(projects)
    }

    pub fn get_project_by_id(&self, id: &str) -> AppResult<Project> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let mut stmt = conn.prepare(
            "SELECT id, name, path, language, framework, status, created_at, updated_at, technologies 
             FROM projects WHERE id = ?",
        )?;

        let project = stmt
            .query_row([id], |row| {
                Ok(Project {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    path: row.get(2)?,
                    language: row.get(3)?,
                    framework: row.get(4)?,
                    status: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                    technologies: row.get(8)?,
                })
            })
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => AppError::ProjectNotFound(id.to_string()),
                _ => AppError::Database(e),
            })?;

        Ok(project)
    }

    pub fn get_project_by_path(&self, path: &str) -> AppResult<Project> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let mut stmt = conn.prepare(
            "SELECT id, name, path, language, framework, status, created_at, updated_at, technologies 
             FROM projects WHERE path = ?",
        )?;

        let project = stmt
            .query_row([path], |row| {
                Ok(Project {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    path: row.get(2)?,
                    language: row.get(3)?,
                    framework: row.get(4)?,
                    status: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                    technologies: row.get(8)?,
                })
            })
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    AppError::ProjectNotFound(format!("No project found at path: {path}"))
                }
                _ => AppError::Database(e),
            })?;

        Ok(project)
    }

    pub fn create_project(&self, project: &Project) -> AppResult<()> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        conn.execute(
            "INSERT INTO projects (id, name, path, language, framework, status, created_at, updated_at, technologies)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                project.id,
                project.name,
                project.path,
                project.language,
                project.framework,
                project.status,
                project.created_at,
                project.updated_at,
                project.technologies,
            ],
        )?;

        tracing::info!("Created project: {} ({})", project.name, project.id);
        Ok(())
    }

    #[allow(dead_code)]
    pub fn update_project(&self, project: &Project) -> AppResult<()> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let rows_affected = conn.execute(
            "UPDATE projects SET name = ?, path = ?, language = ?, framework = ?, 
             status = ?, updated_at = ?, technologies = ? WHERE id = ?",
            params![
                project.name,
                project.path,
                project.language,
                project.framework,
                project.status,
                project.updated_at,
                project.technologies,
                project.id
            ],
        )?;

        if rows_affected == 0 {
            return Err(AppError::ProjectNotFound(project.id.clone()));
        }

        tracing::info!("Updated project: {} ({})", project.name, project.id);
        Ok(())
    }

    pub fn delete_project(&self, id: &str) -> AppResult<()> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        // First delete components
        conn.execute("DELETE FROM project_components WHERE project_id = ?", [id])?;

        let rows_affected = conn.execute("DELETE FROM projects WHERE id = ?", [id])?;

        if rows_affected == 0 {
            return Err(AppError::ProjectNotFound(id.to_string()));
        }

        tracing::info!("Deleted project: {}", id);
        Ok(())
    }

    // Project components methods
    pub fn create_project_component(&self, component: &ProjectComponent) -> AppResult<()> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        conn.execute(
            "INSERT INTO project_components (id, project_id, name, path, language, framework, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                component.id,
                component.project_id,
                component.name,
                component.path,
                component.language,
                component.framework,
                component.created_at
            ],
        )?;

        Ok(())
    }

    pub fn get_components_for_project(&self, project_id: &str) -> AppResult<Vec<ProjectComponent>> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let mut stmt = conn.prepare(
            "SELECT id, project_id, name, path, language, framework, created_at
             FROM project_components WHERE project_id = ? ORDER BY created_at ASC",
        )?;

        let iter = stmt.query_map([project_id], |row| {
            Ok(ProjectComponent {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                path: row.get(3)?,
                language: row.get(4)?,
                framework: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;

        let mut components = Vec::new();
        for item in iter {
            components.push(item?);
        }
        Ok(components)
    }

    // AI Session methods
    pub fn create_ai_session(&self, session: &AiSession) -> AppResult<()> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        conn.execute(
            "INSERT INTO ai_sessions (id, work_id, message_id, tool_name, status, project_context, started_at, ended_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                session.id,
                session.work_id,
                session.message_id,
                session.tool_name,
                session.status,
                session.project_context,
                session.started_at,
                session.ended_at
            ],
        )?;

        tracing::info!(
            "Created AI session: {} with tool {}",
            session.id,
            session.tool_name
        );
        Ok(())
    }

    pub fn get_ai_session_by_id(&self, id: &str) -> AppResult<AiSession> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let mut stmt = conn.prepare(
            "SELECT id, work_id, message_id, tool_name, status, project_context, started_at, ended_at
             FROM ai_sessions WHERE id = ?"
        )?;

        let session = stmt
            .query_row([id], |row| {
                Ok(AiSession {
                    id: row.get(0)?,
                    work_id: row.get(1)?,
                    message_id: row.get(2)?,
                    tool_name: row.get(3)?,
                    status: row.get(4)?,
                    project_context: row.get(5)?,
                    started_at: row.get(6)?,
                    ended_at: row.get(7)?,
                })
            })
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    AppError::Internal(format!("AI session not found: {id}"))
                }
                _ => AppError::Database(e),
            })?;

        Ok(session)
    }

    pub fn update_ai_session(&self, session: &AiSession) -> AppResult<()> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let rows_affected = conn.execute(
            "UPDATE ai_sessions SET status = ?, ended_at = ? WHERE id = ?",
            params![session.status, session.ended_at, session.id],
        )?;

        if rows_affected == 0 {
            return Err(AppError::Internal(format!(
                "AI session not found: {}",
                session.id
            )));
        }

        tracing::info!(
            "Updated AI session: {} status to {}",
            session.id,
            session.status
        );
        Ok(())
    }

    pub fn get_all_ai_sessions(&self) -> AppResult<Vec<AiSession>> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let mut stmt = conn.prepare(
            "SELECT id, work_id, message_id, tool_name, status, project_context, started_at, ended_at
             FROM ai_sessions ORDER BY started_at DESC"
        )?;

        let session_iter = stmt.query_map([], |row| {
            Ok(AiSession {
                id: row.get(0)?,
                work_id: row.get(1)?,
                message_id: row.get(2)?,
                tool_name: row.get(3)?,
                status: row.get(4)?,
                project_context: row.get(5)?,
                started_at: row.get(6)?,
                ended_at: row.get(7)?,
            })
        })?;

        let mut sessions = Vec::new();
        for session in session_iter {
            sessions.push(session?);
        }

        Ok(sessions)
    }

    pub fn get_ai_sessions_by_work_id(&self, work_id: &str) -> AppResult<Vec<AiSession>> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let mut stmt = conn.prepare(
            "SELECT id, work_id, message_id, tool_name, status, project_context, started_at, ended_at
             FROM ai_sessions WHERE work_id = ? ORDER BY started_at DESC"
        )?;

        let session_iter = stmt.query_map([work_id], |row| {
            Ok(AiSession {
                id: row.get(0)?,
                work_id: row.get(1)?,
                message_id: row.get(2)?,
                tool_name: row.get(3)?,
                status: row.get(4)?,
                project_context: row.get(5)?,
                started_at: row.get(6)?,
                ended_at: row.get(7)?,
            })
        })?;

        let mut sessions = Vec::new();
        for session in session_iter {
            sessions.push(session?);
        }

        tracing::debug!(
            "Retrieved {} AI sessions for work {}",
            sessions.len(),
            work_id
        );
        Ok(sessions)
    }

    // Store one-shot AI output content for a session
    pub fn create_ai_session_output(&self, session_id: &str, content: &str) -> AppResult<()> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        conn.execute(
            "INSERT INTO ai_session_outputs (session_id, content, created_at) VALUES (?, ?, strftime('%s','now'))",
            params![session_id, content],
        )?;

        tracing::info!(
            "Recorded AI output for session: {} ({} bytes)",
            session_id,
            content.len()
        );
        Ok(())
    }

    pub fn list_ai_session_outputs(
        &self,
        session_id: &str,
    ) -> AppResult<Vec<crate::models::AiSessionOutput>> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let mut stmt = conn.prepare(
            "SELECT id, session_id, content, created_at FROM ai_session_outputs WHERE session_id = ? ORDER BY id ASC"
        )?;

        let output_iter = stmt.query_map(params![session_id], |row| {
            Ok(crate::models::AiSessionOutput {
                id: row.get(0)?,
                session_id: row.get(1)?,
                content: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;

        let mut outputs = Vec::new();
        for output in output_iter {
            outputs.push(output?);
        }

        tracing::debug!(
            "Retrieved {} outputs for session: {}",
            outputs.len(),
            session_id
        );
        Ok(outputs)
    }

    // Work management methods
    pub fn create_work(&self, work: &crate::models::Work) -> AppResult<()> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        conn.execute(
            "INSERT INTO works (id, title, project_id, tool_name, status, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                work.id,
                work.title,
                work.project_id,
                work.tool_name,
                work.status,
                work.created_at,
                work.updated_at
            ],
        )?;

        tracing::info!("Created work: {} ({})", work.title, work.id);
        Ok(())
    }

    pub fn get_work_by_id(&self, id: &str) -> AppResult<crate::models::Work> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let mut stmt = conn.prepare(
            "SELECT id, title, project_id, tool_name, status, created_at, updated_at
             FROM works WHERE id = ?",
        )?;

        let work = stmt
            .query_row([id], |row| {
                Ok(crate::models::Work {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    project_id: row.get(2)?,
                    tool_name: row.get(3)?,
                    status: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            })
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => {
                    AppError::Internal(format!("Work not found: {id}"))
                }
                _ => AppError::Database(e),
            })?;

        Ok(work)
    }

    pub fn get_all_works(&self) -> AppResult<Vec<crate::models::Work>> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let mut stmt = conn.prepare(
            "SELECT id, title, project_id, tool_name, status, created_at, updated_at
             FROM works ORDER BY created_at DESC",
        )?;

        let work_iter = stmt.query_map([], |row| {
            Ok(crate::models::Work {
                id: row.get(0)?,
                title: row.get(1)?,
                project_id: row.get(2)?,
                tool_name: row.get(3)?,
                status: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;

        let mut works = Vec::new();
        for work in work_iter {
            works.push(work?);
        }

        Ok(works)
    }

    pub fn update_work(&self, work: &crate::models::Work) -> AppResult<()> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")));

        let rows_affected = conn?.execute(
            "UPDATE works SET title = ?, project_id = ?, tool_name = ?, status = ?, updated_at = ? WHERE id = ?",
            params![
                work.title,
                work.project_id,
                work.tool_name,
                work.status,
                work.updated_at,
                work.id
            ],
        )?;

        if rows_affected == 0 {
            return Err(AppError::Internal(format!("Work not found: {}", work.id)));
        }

        tracing::info!("Updated work: {} ({})", work.title, work.id);
        Ok(())
    }

    pub fn delete_work(&self, id: &str) -> AppResult<()> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let rows_affected = conn.execute("DELETE FROM works WHERE id = ?", [id])?;

        if rows_affected == 0 {
            return Err(AppError::Internal(format!("Work not found: {id}")));
        }

        tracing::info!("Deleted work: {}", id);
        Ok(())
    }

    // Work message management methods
    pub fn create_work_message(&self, message: &crate::models::WorkMessage) -> AppResult<()> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        // For code content type, extract language from the enum variant
        let (content_type_str, code_language) = match &message.content_type {
            crate::models::MessageContentType::Code { language } => {
                ("code", Some(language.clone()))
            }
            crate::models::MessageContentType::Text => ("text", None),
            crate::models::MessageContentType::Markdown => ("markdown", None),
            crate::models::MessageContentType::Json => ("json", None),
        };

        conn.execute(
            "INSERT INTO work_messages (id, work_id, content, content_type, code_language, author_type, author_id, sequence_order, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                message.id,
                message.work_id,
                message.content,
                content_type_str,
                code_language,
                match &message.author_type {
                    crate::models::MessageAuthorType::User => "user",
                    crate::models::MessageAuthorType::Ai => "ai",
                },
                message.author_id,
                message.sequence_order,
                message.created_at
            ],
        )?;

        tracing::info!(
            "Created work message: {} for work {}",
            message.id,
            message.work_id
        );
        Ok(())
    }

    pub fn get_work_messages(&self, work_id: &str) -> AppResult<Vec<crate::models::WorkMessage>> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let mut stmt = conn.prepare(
            "SELECT id, work_id, content, content_type, code_language, author_type, author_id, sequence_order, created_at
             FROM work_messages WHERE work_id = ? ORDER BY sequence_order ASC"
        )?;

        let iter = stmt.query_map([work_id], |row| {
            let content_type_str: String = row.get(3)?;
            let code_language: Option<String> = row.get(4)?;
            let author_type_str: String = row.get(5)?;

            let content_type = match content_type_str.as_str() {
                "text" => crate::models::MessageContentType::Text,
                "markdown" => crate::models::MessageContentType::Markdown,
                "json" => crate::models::MessageContentType::Json,
                "code" => crate::models::MessageContentType::Code {
                    language: code_language.unwrap_or_default(),
                },
                _ => crate::models::MessageContentType::Text, // fallback
            };

            let author_type = match author_type_str.as_str() {
                "user" => crate::models::MessageAuthorType::User,
                "ai" => crate::models::MessageAuthorType::Ai,
                _ => crate::models::MessageAuthorType::User, // fallback
            };

            Ok(crate::models::WorkMessage {
                id: row.get(0)?,
                work_id: row.get(1)?,
                content: row.get(2)?,
                content_type,
                author_type,
                author_id: row.get(6)?,
                sequence_order: row.get(7)?,
                created_at: row.get(8)?,
            })
        })?;

        let mut messages = Vec::new();
        for item in iter {
            messages.push(item?);
        }
        Ok(messages)
    }

    pub fn get_work_with_messages(
        &self,
        work_id: &str,
    ) -> AppResult<crate::models::WorkWithHistory> {
        let work = self.get_work_by_id(work_id)?;
        let messages = self.get_work_messages(work_id)?;
        let total_messages = messages.len() as i32;

        Ok(crate::models::WorkWithHistory {
            work,
            messages,
            total_messages,
        })
    }

    pub fn get_next_message_sequence(&self, work_id: &str) -> AppResult<i32> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let mut stmt = conn.prepare(
            "SELECT COALESCE(MAX(sequence_order), -1) + 1 FROM work_messages WHERE work_id = ?",
        )?;

        let sequence: i32 = stmt.query_row([work_id], |row| row.get(0))?;
        Ok(sequence)
    }

    // AI Session Result methods
    #[allow(dead_code)]
    pub fn create_ai_session_result(&self, result: &AiSessionResult) -> AppResult<()> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        conn.execute(
            "INSERT INTO ai_session_results (id, session_id, response_message_id, status, created_at, completed_at)
             VALUES (?, ?, ?, ?, ?, ?)",
            params![
                result.id,
                result.session_id,
                result.response_message_id,
                result.status,
                result.created_at,
                result.completed_at
            ],
        )?;

        tracing::info!(
            "Created AI session result: {} for session {}",
            result.id,
            result.session_id
        );
        Ok(())
    }

    #[allow(dead_code)]
    pub fn update_ai_session_result(&self, result: &AiSessionResult) -> AppResult<()> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let rows_affected = conn.execute(
            "UPDATE ai_session_results SET status = ?, completed_at = ? WHERE id = ?",
            params![result.status, result.completed_at, result.id],
        )?;

        if rows_affected == 0 {
            return Err(AppError::Internal(format!(
                "AI session result not found: {}",
                result.id
            )));
        }

        tracing::info!(
            "Updated AI session result: {} status to {}",
            result.id,
            result.status
        );
        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_ai_session_result_by_session(
        &self,
        session_id: &str,
    ) -> AppResult<Option<AiSessionResult>> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let mut stmt = conn.prepare(
            "SELECT id, session_id, response_message_id, status, created_at, completed_at
             FROM ai_session_results WHERE session_id = ?",
        )?;

        let result = stmt
            .query_row([session_id], |row| {
                Ok(AiSessionResult {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    response_message_id: row.get(2)?,
                    status: row.get(3)?,
                    created_at: row.get(4)?,
                    completed_at: row.get(5)?,
                })
            })
            .optional()?;

        Ok(result)
    }

    // LLM Agent Methods

    pub fn create_llm_agent_session(&self, session: &LlmAgentSession) -> AppResult<()> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        conn.execute(
            "INSERT INTO llm_agent_sessions (id, work_id, provider, model, status, system_prompt, started_at, ended_at) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                session.id,
                session.work_id,
                session.provider,
                session.model,
                session.status,
                session.system_prompt,
                session.started_at,
                session.ended_at,
            ],
        )?;

        tracing::info!("Created LLM agent session: {}", session.id);
        Ok(())
    }

    pub fn get_llm_agent_session(&self, session_id: &str) -> AppResult<LlmAgentSession> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let session = conn
            .query_row(
                "SELECT id, work_id, provider, model, status, system_prompt, started_at, ended_at 
                 FROM llm_agent_sessions WHERE id = ?",
                [session_id],
                |row| {
                    Ok(LlmAgentSession {
                        id: row.get(0)?,
                        work_id: row.get(1)?,
                        provider: row.get(2)?,
                        model: row.get(3)?,
                        status: row.get(4)?,
                        system_prompt: row.get(5)?,
                        started_at: row.get(6)?,
                        ended_at: row.get(7)?,
                    })
                },
            )
            .optional()?;

        match session {
            Some(session) => Ok(session),
            None => Err(AppError::NotFound(format!(
                "LLM agent session not found: {}",
                session_id
            ))),
        }
    }

    pub fn update_llm_agent_session(&self, session: &LlmAgentSession) -> AppResult<()> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        conn.execute(
            "UPDATE llm_agent_sessions 
             SET work_id = ?, provider = ?, model = ?, status = ?, system_prompt = ?, started_at = ?, ended_at = ? 
             WHERE id = ?",
            params![
                session.work_id,
                session.provider,
                session.model,
                session.status,
                session.system_prompt,
                session.started_at,
                session.ended_at,
                session.id,
            ],
        )?;

        tracing::info!("Updated LLM agent session: {}", session.id);
        Ok(())
    }

    pub fn get_llm_agent_sessions_by_work(&self, work_id: &str) -> AppResult<Vec<LlmAgentSession>> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let mut stmt = conn.prepare(
            "SELECT id, work_id, provider, model, status, system_prompt, started_at, ended_at 
             FROM llm_agent_sessions WHERE work_id = ? ORDER BY started_at DESC",
        )?;

        let session_iter = stmt.query_map([work_id], |row| {
            Ok(LlmAgentSession {
                id: row.get(0)?,
                work_id: row.get(1)?,
                provider: row.get(2)?,
                model: row.get(3)?,
                status: row.get(4)?,
                system_prompt: row.get(5)?,
                started_at: row.get(6)?,
                ended_at: row.get(7)?,
            })
        })?;

        let sessions: Result<Vec<_>, _> = session_iter.collect();
        sessions.map_err(AppError::from)
    }

    pub fn get_llm_agent_session_by_work_id(&self, work_id: &str) -> AppResult<LlmAgentSession> {
        let sessions = self.get_llm_agent_sessions_by_work(work_id)?;

        match sessions.first() {
            Some(session) => Ok(session.clone()),
            None => Err(AppError::NotFound(format!(
                "LLM agent session not found for work: {}",
                work_id
            ))),
        }
    }

    pub fn create_llm_agent_message(
        &self,
        session_id: &str,
        role: &str,
        content: String,
    ) -> AppResult<i64> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let now = chrono::Utc::now().timestamp();

        conn.execute(
            "INSERT INTO llm_agent_messages (session_id, role, content, created_at) VALUES (?, ?, ?, ?)",
            params![session_id, role, content, now],
        )?;

        let message_id = conn.last_insert_rowid();
        tracing::info!(
            "Created LLM agent message: {} for session: {}",
            message_id,
            session_id
        );
        Ok(message_id)
    }

    pub fn get_llm_agent_messages(&self, session_id: &str) -> AppResult<Vec<LlmAgentMessage>> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let mut stmt = conn.prepare(
            "SELECT id, session_id, role, content, created_at 
             FROM llm_agent_messages WHERE session_id = ? ORDER BY created_at ASC",
        )?;

        let message_iter = stmt.query_map([session_id], |row| {
            Ok(LlmAgentMessage {
                id: row.get(0)?,
                session_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;

        let messages: Result<Vec<_>, _> = message_iter.collect();
        messages.map_err(AppError::from)
    }

    pub fn create_llm_agent_tool_call(&self, tool_call: &LlmAgentToolCall) -> AppResult<i64> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        conn.execute(
            "INSERT INTO llm_agent_tool_calls (session_id, message_id, tool_name, request, response, status, created_at, completed_at, execution_time_ms, progress_updates, error_details)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                tool_call.session_id,
                tool_call.message_id,
                tool_call.tool_name,
                serde_json::to_string(&tool_call.request).unwrap_or_default(),
                tool_call.response.as_ref().map(|r| serde_json::to_string(r).unwrap_or_default()),
                tool_call.status,
                tool_call.created_at,
                tool_call.completed_at,
                tool_call.execution_time_ms,
                tool_call.progress_updates,
                tool_call.error_details,
            ],
        )?;

        let tool_call_id = conn.last_insert_rowid();
        tracing::info!(
            "Created LLM agent tool call: {} for session: {}",
            tool_call_id,
            tool_call.session_id
        );
        Ok(tool_call_id)
    }

    pub fn update_llm_agent_tool_call(&self, tool_call: &LlmAgentToolCall) -> AppResult<()> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        conn.execute(
            "UPDATE llm_agent_tool_calls
             SET session_id = ?, message_id = ?, tool_name = ?, request = ?, response = ?, status = ?, created_at = ?, completed_at = ?, execution_time_ms = ?, progress_updates = ?, error_details = ?
             WHERE id = ?",
            params![
                tool_call.session_id,
                tool_call.message_id,
                tool_call.tool_name,
                serde_json::to_string(&tool_call.request).unwrap_or_default(),
                tool_call.response.as_ref().map(|r| serde_json::to_string(r).unwrap_or_default()),
                tool_call.status,
                tool_call.created_at,
                tool_call.completed_at,
                tool_call.execution_time_ms,
                tool_call.progress_updates,
                tool_call.error_details,
                tool_call.id,
            ],
        )?;

        tracing::info!("Updated LLM agent tool call: {}", tool_call.id);
        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_llm_agent_tool_calls(&self, session_id: &str) -> AppResult<Vec<LlmAgentToolCall>> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let mut stmt = conn.prepare(
            "SELECT id, session_id, message_id, tool_name, request, response, status, created_at, completed_at, execution_time_ms, progress_updates, error_details
             FROM llm_agent_tool_calls WHERE session_id = ? ORDER BY created_at ASC",
        )?;

        let tool_call_iter = stmt.query_map([session_id], |row| {
            let request_str: String = row.get(4)?;
            let response_str: Option<String> = row.get(5)?;

            Ok(LlmAgentToolCall {
                id: row.get(0)?,
                session_id: row.get(1)?,
                message_id: row.get(2)?,
                tool_name: row.get(3)?,
                request: serde_json::from_str(&request_str).unwrap_or_default(),
                response: response_str.and_then(|s| serde_json::from_str(&s).ok()),
                status: row.get(6)?,
                created_at: row.get(7)?,
                completed_at: row.get(8)?,
                execution_time_ms: row.get(9)?,
                progress_updates: row.get(10)?,
                error_details: row.get(11)?,
            })
        })?;

        let tool_calls: Result<Vec<_>, _> = tool_call_iter.collect();
        tool_calls.map_err(AppError::from)
    }

    // Workflow methods

    #[allow(dead_code)]
    pub fn store_workflow_commands(
        &self,
        project_id: &str,
        commands: &[nocodo_github_actions::WorkflowCommand],
    ) -> AppResult<()> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        for command in commands {
            let environment_json = command
                .environment
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .map_err(|e| {
                    AppError::Internal(format!("Failed to serialize environment: {}", e))
                })?;

            conn.execute(
                r#"
                INSERT OR REPLACE INTO workflow_commands
                (id, workflow_name, job_name, step_name, command, shell, working_directory, environment, file_path, project_id)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                rusqlite::params![
                    command.id,
                    command.workflow_name,
                    command.job_name,
                    command.step_name,
                    command.command,
                    command.shell,
                    command.working_directory,
                    environment_json,
                    command.file_path,
                    project_id
                ],
            )?;
        }

        Ok(())
    }

    pub fn get_workflow_commands(
        &self,
        project_id: &str,
    ) -> AppResult<Vec<nocodo_github_actions::WorkflowCommand>> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let mut stmt = conn.prepare(
            r#"
            SELECT id, workflow_name, job_name, step_name, command, shell, working_directory, environment, file_path
            FROM workflow_commands
            WHERE project_id = ?
            ORDER BY workflow_name, job_name
            "#,
        )?;

        let command_iter = stmt.query_map([project_id], |row| {
            let environment: Option<String> = row.get("environment")?;
            let environment_parsed = if let Some(env_json) = environment {
                Some(serde_json::from_str(&env_json).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?)
            } else {
                None
            };

            Ok(nocodo_github_actions::WorkflowCommand {
                id: row.get("id")?,
                workflow_name: row.get("workflow_name")?,
                job_name: row.get("job_name")?,
                step_name: row.get("step_name")?,
                command: row.get("command")?,
                shell: row.get("shell")?,
                working_directory: row.get("working_directory")?,
                environment: environment_parsed,
                file_path: row.get("file_path")?,
            })
        })?;

        let mut commands = Vec::new();
        for command in command_iter {
            commands.push(command?);
        }

        Ok(commands)
    }

    #[allow(dead_code)]
    pub fn store_command_execution(
        &self,
        execution: &nocodo_github_actions::CommandExecution,
    ) -> AppResult<i64> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        conn.execute(
            r#"
            INSERT INTO command_executions
            (command_id, exit_code, stdout, stderr, duration_ms, executed_at, success)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
            rusqlite::params![
                execution.command_id,
                execution.exit_code,
                execution.stdout,
                execution.stderr,
                execution.duration_ms as i64,
                execution.executed_at.timestamp(),
                execution.success
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    pub fn get_command_executions(
        &self,
        command_id: &str,
    ) -> AppResult<Vec<nocodo_github_actions::CommandExecution>> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let mut stmt = conn.prepare(
            r#"
            SELECT command_id, exit_code, stdout, stderr, duration_ms, executed_at, success
            FROM command_executions
            WHERE command_id = ?
            ORDER BY executed_at DESC
            "#,
        )?;

        let execution_iter = stmt.query_map([command_id], |row| {
            Ok(nocodo_github_actions::CommandExecution {
                command_id: row.get("command_id")?,
                exit_code: row.get("exit_code")?,
                stdout: row.get("stdout")?,
                stderr: row.get("stderr")?,
                duration_ms: row.get::<_, i64>("duration_ms")? as u64,
                executed_at: chrono::DateTime::from_timestamp(row.get::<_, i64>("executed_at")?, 0)
                    .unwrap_or_else(chrono::Utc::now),
                success: row.get("success")?,
            })
        })?;

        let mut executions = Vec::new();
        for execution in execution_iter {
            executions.push(execution?);
        }

        Ok(executions)
    }
}
