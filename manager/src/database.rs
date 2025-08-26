use crate::error::{AppError, AppResult};
use crate::models::{AiSession, AiSessionOutput, Project, ProjectComponent};
use rusqlite::{params, Connection};
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
                updated_at INTEGER NOT NULL
            )",
            [],
        )?;

        // Create an index on the name for faster lookups
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_projects_name ON projects(name)",
            [],
        )?;

        // Create AI sessions table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS ai_sessions (
                id TEXT PRIMARY KEY,
                project_id TEXT,
                tool_name TEXT NOT NULL,
                status TEXT NOT NULL,
                prompt TEXT NOT NULL,
                project_context TEXT,
                started_at INTEGER NOT NULL,
                ended_at INTEGER,
                FOREIGN KEY (project_id) REFERENCES projects (id)
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

        // Create an index on the project_id for faster lookups
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_sessions_project_id ON ai_sessions(project_id)",
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

        tracing::info!("Database migrations completed");
        Ok(())
    }

    pub fn get_all_projects(&self) -> AppResult<Vec<Project>> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let mut stmt = conn.prepare(
            "SELECT id, name, path, language, framework, status, created_at, updated_at 
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
            "SELECT id, name, path, language, framework, status, created_at, updated_at 
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
            "SELECT id, name, path, language, framework, status, created_at, updated_at 
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
            "INSERT INTO projects (id, name, path, language, framework, status, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                project.id,
                project.name,
                project.path,
                project.language,
                project.framework,
                project.status,
                project.created_at,
                project.updated_at
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
             status = ?, updated_at = ? WHERE id = ?",
            params![
                project.name,
                project.path,
                project.language,
                project.framework,
                project.status,
                project.updated_at,
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
            "INSERT INTO ai_sessions (id, project_id, tool_name, status, prompt, project_context, started_at, ended_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                session.id,
                session.project_id,
                session.tool_name,
                session.status,
                session.prompt,
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
            "SELECT id, project_id, tool_name, status, prompt, project_context, started_at, ended_at
             FROM ai_sessions WHERE id = ?"
        )?;

        let session = stmt
            .query_row([id], |row| {
                Ok(AiSession {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    tool_name: row.get(2)?,
                    status: row.get(3)?,
                    prompt: row.get(4)?,
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
            "SELECT id, project_id, tool_name, status, prompt, project_context, started_at, ended_at
             FROM ai_sessions ORDER BY started_at DESC"
        )?;

        let session_iter = stmt.query_map([], |row| {
            Ok(AiSession {
                id: row.get(0)?,
                project_id: row.get(1)?,
                tool_name: row.get(2)?,
                status: row.get(3)?,
                prompt: row.get(4)?,
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

    // Retrieve outputs for a given AI session
    pub fn get_ai_session_outputs(&self, session_id: &str) -> AppResult<Vec<AiSessionOutput>> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let mut stmt = conn.prepare(
            "SELECT id, session_id, content, created_at FROM ai_session_outputs WHERE session_id = ? ORDER BY id ASC",
        )?;

        let iter = stmt.query_map([session_id], |row| {
            Ok(AiSessionOutput {
                id: row.get(0)?,
                session_id: row.get(1)?,
                content: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;

        let mut outputs = Vec::new();
        for item in iter {
            outputs.push(item?);
        }
        Ok(outputs)
    }
}
