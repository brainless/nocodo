use crate::error::{AppError, AppResult};
use crate::models::{
    AiSession, AiSessionResult, LlmAgentMessage, LlmAgentSession, LlmAgentToolCall, Project,
    ProjectComponent,
};
use crate::permissions::Action;
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
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 name TEXT NOT NULL,
                 path TEXT NOT NULL UNIQUE,
                 description TEXT,
                 parent_id INTEGER,
                 created_at INTEGER NOT NULL,
                 updated_at INTEGER NOT NULL,
                 FOREIGN KEY (parent_id) REFERENCES projects (id)
             )",
            [],
        )?;

        // Create an index on the name for faster lookups
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_projects_name ON projects(name)",
            [],
        )?;

        // Migration: Add new columns if they don't exist (for existing databases)
        let _ = conn.execute("ALTER TABLE projects ADD COLUMN description TEXT", []);
        let _ = conn.execute(
            "ALTER TABLE projects ADD COLUMN parent_id INTEGER REFERENCES projects(id)",
            [],
        );

        // Migration: Drop old columns if they exist (SQLite doesn't support DROP COLUMN before 3.35.0)
        // We'll handle this by creating a new table and migrating data
        let has_old_columns: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('projects') WHERE name IN ('language', 'framework', 'status', 'technologies')",
                [],
                |row| row.get::<_, i32>(0)
            )
            .unwrap_or(0) > 0;

        if has_old_columns {
            // Create new table with correct schema
            conn.execute(
                "CREATE TABLE IF NOT EXISTS projects_new (
                     id INTEGER PRIMARY KEY AUTOINCREMENT,
                     name TEXT NOT NULL,
                     path TEXT NOT NULL UNIQUE,
                     description TEXT,
                     parent_id INTEGER,
                     created_at INTEGER NOT NULL,
                     updated_at INTEGER NOT NULL,
                     FOREIGN KEY (parent_id) REFERENCES projects (id)
                 )",
                [],
            )?;

            // Copy data from old table to new table
            conn.execute(
                "INSERT INTO projects_new (id, name, path, description, parent_id, created_at, updated_at)
                 SELECT id, name, path, NULL, NULL, created_at, updated_at FROM projects",
                [],
            )?;

            // Drop old table
            conn.execute("DROP TABLE projects", [])?;

            // Rename new table to old table name
            conn.execute("ALTER TABLE projects_new RENAME TO projects", [])?;

            // Recreate index
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_projects_name ON projects(name)",
                [],
            )?;
        }

        // Create AI sessions table - now links to Work and Message instead of storing prompt directly
        conn.execute(
            "CREATE TABLE IF NOT EXISTS ai_sessions (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 work_id INTEGER NOT NULL,
                 message_id INTEGER NOT NULL,
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
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 project_id INTEGER NOT NULL,
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
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 session_id INTEGER NOT NULL,
                 response_message_id INTEGER NOT NULL,
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
                 session_id INTEGER NOT NULL,
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
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 title TEXT NOT NULL,
                 project_id INTEGER,
                 model TEXT,
                 status TEXT NOT NULL DEFAULT 'active',
                 created_at INTEGER NOT NULL,
                 updated_at INTEGER NOT NULL,
                 FOREIGN KEY (project_id) REFERENCES projects (id)
             )",
            [],
        )?;

        // Create work messages with content types and history
        conn.execute(
             "CREATE TABLE IF NOT EXISTS work_messages (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 work_id INTEGER NOT NULL,
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
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 work_id INTEGER NOT NULL,
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
                 session_id INTEGER NOT NULL,
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
                 session_id INTEGER NOT NULL,
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

        // User authentication tables
        conn.execute(
            "CREATE TABLE IF NOT EXISTS users (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 username TEXT NOT NULL UNIQUE,
                 email TEXT NOT NULL UNIQUE,
                 password_hash TEXT NOT NULL,
                 is_active INTEGER NOT NULL DEFAULT 1,
                 created_at INTEGER NOT NULL,
                 updated_at INTEGER NOT NULL
             )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_users_username ON users(username)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_users_email ON users(email)",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS user_ssh_keys (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 user_id INTEGER NOT NULL,
                 key_type TEXT NOT NULL,
                 fingerprint TEXT NOT NULL UNIQUE,
                 public_key_data TEXT NOT NULL,
                 label TEXT,
                 is_active INTEGER NOT NULL DEFAULT 1,
                 created_at INTEGER NOT NULL,
                 last_used_at INTEGER,
                 FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
             )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_user_ssh_keys_user_id ON user_ssh_keys(user_id)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_user_ssh_keys_fingerprint ON user_ssh_keys(fingerprint)",
            [],
        )?;

        // GitHub Actions workflow tables
        conn.execute(
            r#"
             CREATE TABLE IF NOT EXISTS workflow_commands (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 workflow_name TEXT NOT NULL,
                 job_name TEXT NOT NULL,
                 step_name TEXT,
                 command TEXT NOT NULL,
                 shell TEXT,
                 working_directory TEXT,
                 environment TEXT, -- JSON string
                 file_path TEXT NOT NULL,
                 project_id INTEGER NOT NULL,
                 created_at INTEGER DEFAULT (strftime('%s','now')),
                 FOREIGN KEY (project_id) REFERENCES projects (id) ON DELETE CASCADE
             )
             "#,
            [],
        )?;

        conn.execute(
            r#"
             CREATE TABLE IF NOT EXISTS command_executions (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 command_id INTEGER NOT NULL,
                 exit_code INTEGER,
                 stdout TEXT NOT NULL,
                 stderr TEXT NOT NULL,
                 duration_ms INTEGER NOT NULL,
                 executed_at INTEGER NOT NULL,
                 success INTEGER NOT NULL,
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

        // Migration: Fix DATETIME and BOOLEAN types in workflow tables
        let has_datetime_columns: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('workflow_commands') WHERE name = 'created_at' AND type LIKE '%DATETIME%'",
                [],
                |row| row.get::<_, i32>(0)
            )
            .unwrap_or(0) > 0;

        if has_datetime_columns {
            tracing::info!("Migrating DATETIME columns to INTEGER in workflow tables");

            // Migrate workflow_commands table
            conn.execute(
                "CREATE TABLE workflow_commands_new (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    workflow_name TEXT NOT NULL,
                    job_name TEXT NOT NULL,
                    step_name TEXT,
                    command TEXT NOT NULL,
                    shell TEXT,
                    working_directory TEXT,
                    environment TEXT,
                    file_path TEXT NOT NULL,
                    project_id INTEGER NOT NULL,
                    created_at INTEGER DEFAULT (strftime('%s','now')),
                    FOREIGN KEY (project_id) REFERENCES projects (id) ON DELETE CASCADE
                )",
                [],
            )?;

            conn.execute(
                "INSERT INTO workflow_commands_new 
                 SELECT id, workflow_name, job_name, step_name, command, shell, working_directory, environment, file_path, project_id, 
                        CASE 
                            WHEN created_at IS NULL THEN strftime('%s','now')
                            WHEN typeof(created_at) = 'integer' THEN created_at
                            ELSE strftime('%s', created_at)
                        END
                 FROM workflow_commands",
                [],
            )?;

            conn.execute("DROP TABLE workflow_commands", [])?;
            conn.execute(
                "ALTER TABLE workflow_commands_new RENAME TO workflow_commands",
                [],
            )?;
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_workflow_commands_project_id ON workflow_commands(project_id)",
                [],
            )?;
        }

        // Migration: Fix BOOLEAN to INTEGER for success column in command_executions
        let has_boolean_success: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('command_executions') WHERE name = 'success' AND type LIKE '%BOOLEAN%'",
                [],
                |row| row.get::<_, i32>(0)
            )
            .unwrap_or(0) > 0;

        let has_datetime_executed_at: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('command_executions') WHERE name = 'executed_at' AND type LIKE '%DATETIME%'",
                [],
                |row| row.get::<_, i32>(0)
            )
            .unwrap_or(0) > 0;

        if has_boolean_success || has_datetime_executed_at {
            tracing::info!("Migrating command_executions table schema");
            // SQLite doesn't support ALTER COLUMN directly, so we need to recreate the table
            conn.execute(
                "CREATE TABLE command_executions_new (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    command_id INTEGER NOT NULL,
                    exit_code INTEGER,
                    stdout TEXT NOT NULL,
                    stderr TEXT NOT NULL,
                    duration_ms INTEGER NOT NULL,
                    executed_at INTEGER NOT NULL,
                    success INTEGER NOT NULL,
                    FOREIGN KEY (command_id) REFERENCES workflow_commands (id) ON DELETE CASCADE
                )",
                [],
            )?;

            // Copy data, converting types as needed
            conn.execute(
                "INSERT INTO command_executions_new 
                 SELECT id, command_id, exit_code, stdout, stderr, duration_ms, 
                        CASE 
                            WHEN typeof(executed_at) = 'integer' THEN executed_at
                            ELSE strftime('%s', executed_at)
                        END,
                        CASE WHEN success THEN 1 ELSE 0 END 
                 FROM command_executions",
                [],
            )?;

            // Drop old table and rename new one
            conn.execute("DROP TABLE command_executions", [])?;
            conn.execute(
                "ALTER TABLE command_executions_new RENAME TO command_executions",
                [],
            )?;

            // Recreate index
            conn.execute(
                "CREATE INDEX IF NOT EXISTS idx_command_executions_command_id ON command_executions(command_id)",
                [],
            )?;

            tracing::info!("Successfully migrated command_executions table schema");
        }

        // Permission system tables (Phase 1: DB & Models)

        // Teams table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS teams (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                created_by INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                FOREIGN KEY (created_by) REFERENCES users (id) ON DELETE SET NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_teams_created_by ON teams(created_by)",
            [],
        )?;

        // Team members table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS team_members (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                team_id INTEGER NOT NULL,
                user_id INTEGER NOT NULL,
                added_by INTEGER,
                added_at INTEGER NOT NULL,
                FOREIGN KEY (team_id) REFERENCES teams (id) ON DELETE CASCADE,
                FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
                FOREIGN KEY (added_by) REFERENCES users (id) ON DELETE SET NULL,
                UNIQUE(team_id, user_id)
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_team_members_team_id ON team_members(team_id)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_team_members_user_id ON team_members(user_id)",
            [],
        )?;

        // Permissions table (team-based only)
        conn.execute(
            "CREATE TABLE IF NOT EXISTS permissions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                team_id INTEGER NOT NULL,
                resource_type TEXT NOT NULL CHECK (resource_type IN ('project', 'work', 'settings', 'user', 'team', 'ai_session')),
                resource_id INTEGER,
                action TEXT NOT NULL CHECK (action IN ('read', 'write', 'delete', 'admin')),
                granted_by INTEGER,
                granted_at INTEGER NOT NULL,
                FOREIGN KEY (team_id) REFERENCES teams (id) ON DELETE CASCADE,
                FOREIGN KEY (granted_by) REFERENCES users (id) ON DELETE SET NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_permissions_team_id ON permissions(team_id)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_permissions_resource ON permissions(resource_type, resource_id)",
            [],
        )?;

        // Resource ownership table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS resource_ownership (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                resource_type TEXT NOT NULL CHECK (resource_type IN ('project', 'work', 'settings', 'user', 'team', 'ai_session')),
                resource_id INTEGER NOT NULL,
                owner_id INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (owner_id) REFERENCES users (id) ON DELETE CASCADE,
                UNIQUE(resource_type, resource_id)
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_resource_ownership_owner ON resource_ownership(owner_id)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_resource_ownership_resource ON resource_ownership(resource_type, resource_id)",
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
            "SELECT id, name, path, description, parent_id, created_at, updated_at
             FROM projects WHERE parent_id IS NULL ORDER BY created_at DESC",
        )?;

        let project_iter = stmt.query_map([], |row| {
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                path: row.get(2)?,
                description: row.get(3)?,
                parent_id: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;

        let mut projects = Vec::new();
        for project in project_iter {
            projects.push(project?);
        }

        Ok(projects)
    }

    pub fn get_project_by_id(&self, id: i64) -> AppResult<Project> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let mut stmt = conn.prepare(
            "SELECT id, name, path, description, parent_id, created_at, updated_at
             FROM projects WHERE id = ?",
        )?;

        let project = stmt
            .query_row([id], |row| {
                Ok(Project {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    path: row.get(2)?,
                    description: row.get(3)?,
                    parent_id: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
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
            "SELECT id, name, path, description, parent_id, created_at, updated_at
             FROM projects WHERE path = ?",
        )?;

        let project = stmt
            .query_row([path], |row| {
                Ok(Project {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    path: row.get(2)?,
                    description: row.get(3)?,
                    parent_id: row.get(4)?,
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
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

    pub fn create_project(&self, project: &Project) -> AppResult<i64> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let id_param = if project.id == 0 {
            None
        } else {
            Some(project.id)
        };

        conn.execute(
            "INSERT INTO projects (id, name, path, description, parent_id, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                id_param,
                project.name,
                project.path,
                project.description,
                project.parent_id,
                project.created_at,
                project.updated_at,
            ],
        )?;

        let project_id = conn.last_insert_rowid();
        tracing::info!("Created project: {} ({})", project.name, project_id);
        Ok(project_id)
    }

    #[allow(dead_code)]
    pub fn update_project(&self, project: &Project) -> AppResult<()> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let rows_affected = conn.execute(
            "UPDATE projects SET name = ?, path = ?, description = ?, parent_id = ?, updated_at = ? WHERE id = ?",
            params![
                project.name,
                project.path,
                project.description,
                project.parent_id,
                project.updated_at,
                project.id
            ],
        )?;

        if rows_affected == 0 {
            return Err(AppError::ProjectNotFound(project.id.to_string()));
        }

        tracing::info!("Updated project: {} ({})", project.name, project.id);
        Ok(())
    }

    pub fn delete_project(&self, id: i64) -> AppResult<()> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        // First delete components
        conn.execute("DELETE FROM project_components WHERE project_id = ?", [id])?;

        // Clean up ownership records for this project
        conn.execute(
            "DELETE FROM resource_ownership WHERE resource_type = 'project' AND resource_id = ?",
            [id],
        )?;

        // Clean up permissions referencing this specific project
        conn.execute(
            "DELETE FROM permissions WHERE resource_type = 'project' AND resource_id = ?",
            [id],
        )?;

        let rows_affected = conn.execute("DELETE FROM projects WHERE id = ?", [id])?;

        if rows_affected == 0 {
            return Err(AppError::ProjectNotFound(id.to_string()));
        }

        tracing::info!("Deleted project: {}", id);
        Ok(())
    }

    // Project components methods
    pub fn get_components_for_project(&self, project_id: i64) -> AppResult<Vec<ProjectComponent>> {
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
    pub fn create_ai_session(&self, session: &AiSession) -> AppResult<i64> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let id_param = if session.id == 0 {
            None
        } else {
            Some(session.id)
        };

        conn.execute(
            "INSERT INTO ai_sessions (id, work_id, message_id, tool_name, status, project_context, started_at, ended_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                id_param,
                session.work_id,
                session.message_id,
                session.tool_name,
                session.status,
                session.project_context,
                session.started_at,
                session.ended_at
            ],
        )?;

        let session_id = conn.last_insert_rowid();

        tracing::info!(
            "Created AI session: {} with tool {}",
            session_id,
            session.tool_name
        );
        Ok(session_id)
    }

    pub fn get_ai_session_by_id(&self, id: i64) -> AppResult<AiSession> {
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
                    AppError::Internal(format!("AI session not found: {}", id))
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

    pub fn get_ai_sessions_by_work_id(&self, work_id: i64) -> AppResult<Vec<AiSession>> {
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
    pub fn create_ai_session_output(&self, session_id: i64, content: &str) -> AppResult<()> {
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
        session_id: i64,
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
                role: None,  // Old outputs don't have role information
                model: None, // Old outputs don't have model information
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
    pub fn create_work(&self, work: &crate::models::Work) -> AppResult<i64> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let id_param = if work.id == 0 { None } else { Some(work.id) };

        conn.execute(
            "INSERT INTO works (id, title, project_id, model, status, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                id_param,
                work.title,
                work.project_id,
                work.model,
                work.status,
                work.created_at,
                work.updated_at
            ],
        )?;

        let work_id = conn.last_insert_rowid();
        tracing::info!("Created work: {} ({})", work.title, work_id);
        Ok(work_id)
    }

    pub fn get_work_by_id(&self, id: i64) -> AppResult<crate::models::Work> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let mut stmt = conn.prepare(
            "SELECT id, title, project_id, model, status, created_at, updated_at
             FROM works WHERE id = ?",
        )?;

        let work = stmt
            .query_row([id], |row| {
                Ok(crate::models::Work {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    project_id: row.get(2)?,
                    model: row.get(3)?,
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
            "SELECT id, title, project_id, model, status, created_at, updated_at
             FROM works ORDER BY created_at DESC",
        )?;

        let work_iter = stmt.query_map([], |row| {
            Ok(crate::models::Work {
                id: row.get(0)?,
                title: row.get(1)?,
                project_id: row.get(2)?,
                model: row.get(3)?,
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

    #[allow(dead_code)]
    pub fn update_work(&self, work: &crate::models::Work) -> AppResult<()> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")));

        let rows_affected = conn?.execute(
            "UPDATE works SET title = ?, project_id = ?, status = ?, updated_at = ? WHERE id = ?",
            params![
                work.title,
                work.project_id,
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

    pub fn delete_work(&self, id: i64) -> AppResult<()> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        // Clean up ownership records for this work
        conn.execute(
            "DELETE FROM resource_ownership WHERE resource_type = 'work' AND resource_id = ?",
            [id],
        )?;

        // Clean up permissions referencing this specific work
        conn.execute(
            "DELETE FROM permissions WHERE resource_type = 'work' AND resource_id = ?",
            [id],
        )?;

        let rows_affected = conn.execute("DELETE FROM works WHERE id = ?", [id])?;

        if rows_affected == 0 {
            return Err(AppError::Internal(format!("Work not found: {id}")));
        }

        tracing::info!("Deleted work: {}", id);
        Ok(())
    }

    // Work message management methods
    /// Create work with initial message in a single transaction
    pub fn create_work_with_message(
        &self,
        work: &crate::models::Work,
        message_content: String,
    ) -> AppResult<(i64, i64)> {
        let mut conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        // Begin transaction
        let tx = conn.transaction()?;

        // Create work
        let id_param = if work.id == 0 { None } else { Some(work.id) };

        tx.execute(
            "INSERT INTO works (id, title, project_id, model, status, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                id_param,
                work.title,
                work.project_id,
                work.model,
                work.status,
                work.created_at,
                work.updated_at
            ],
        )?;

        let work_id = tx.last_insert_rowid();

        // Create initial message with the work title as content
        let now = chrono::Utc::now().timestamp();
        tx.execute(
            "INSERT INTO work_messages (work_id, content, content_type, author_type, author_id, sequence_order, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![work_id, message_content, "text", "user", Option::<String>::None, 0, now],
        )?;

        let message_id = tx.last_insert_rowid();

        // Commit transaction
        tx.commit()?;

        tracing::info!(
            "Created work '{}' ({}) with initial message ({})",
            work.title,
            work_id,
            message_id
        );

        Ok((work_id, message_id))
    }

    pub fn create_work_message(&self, message: &crate::models::WorkMessage) -> AppResult<i64> {
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

        // Debug logging to understand parameter types
        let author_type_str = match &message.author_type {
            crate::models::MessageAuthorType::User => "user",
            crate::models::MessageAuthorType::Ai => "ai",
        };

        let id_param = if message.id == 0 {
            None
        } else {
            Some(message.id)
        };

        conn.execute(
            "INSERT INTO work_messages (id, work_id, content, content_type, code_language, author_type, author_id, sequence_order, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                id_param,
                message.work_id,
                message.content,
                content_type_str,
                code_language,
                author_type_str,
                message.author_id,
                message.sequence_order,
                message.created_at
            ],
        )?;

        let message_id = conn.last_insert_rowid();

        tracing::info!(
            "Created work message: {} for work {}",
            message_id,
            message.work_id
        );
        Ok(message_id)
    }

    pub fn get_work_messages(&self, work_id: i64) -> AppResult<Vec<crate::models::WorkMessage>> {
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
        work_id: i64,
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

    pub fn get_next_message_sequence(&self, work_id: i64) -> AppResult<i32> {
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

        let id_param = if result.id == 0 {
            None
        } else {
            Some(result.id)
        };

        conn.execute(
            "INSERT INTO ai_session_results (id, session_id, response_message_id, status, created_at, completed_at)
             VALUES (?, ?, ?, ?, ?, ?)",
            params![
                id_param,
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

    pub fn create_llm_agent_session(&self, session: &LlmAgentSession) -> AppResult<i64> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let id_param = if session.id == 0 {
            None
        } else {
            Some(session.id)
        };

        conn.execute(
            "INSERT INTO llm_agent_sessions (id, work_id, provider, model, status, system_prompt, started_at, ended_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                id_param,
                session.work_id,
                session.provider,
                session.model,
                session.status,
                session.system_prompt,
                session.started_at,
                session.ended_at,
            ],
        )?;

        let session_id = conn.last_insert_rowid();

        tracing::info!("Created LLM agent session: {}", session_id);
        Ok(session_id)
    }

    pub fn get_llm_agent_session(&self, session_id: i64) -> AppResult<LlmAgentSession> {
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

    pub fn get_llm_agent_sessions_by_work(&self, work_id: i64) -> AppResult<Vec<LlmAgentSession>> {
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

    pub fn get_llm_agent_session_by_work_id(&self, work_id: i64) -> AppResult<LlmAgentSession> {
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
        session_id: i64,
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

    pub fn get_llm_agent_messages(&self, session_id: i64) -> AppResult<Vec<LlmAgentMessage>> {
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
    pub fn get_llm_agent_tool_calls(&self, session_id: i64) -> AppResult<Vec<LlmAgentToolCall>> {
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
        project_id: i64,
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
                if execution.success { 1i64 } else { 0i64 }
            ],
        )?;

        Ok(conn.last_insert_rowid())
    }

    pub fn get_command_executions(
        &self,
        command_id: i64,
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
                success: row.get::<_, i64>("success")? != 0,
            })
        })?;

        let mut executions = Vec::new();
        for execution in execution_iter {
            executions.push(execution?);
        }

        Ok(executions)
    }

    // User authentication methods

    pub fn create_user(&self, user: &crate::models::User) -> AppResult<i64> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let id_param = if user.id == 0 { None } else { Some(user.id) };

        conn.execute(
            "INSERT INTO users (id, name, email, role, password_hash, is_active, created_at, updated_at, last_login_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                id_param,
                user.name,
                user.email,
                user.role,
                user.password_hash,
                if user.is_active { 1 } else { 0 },
                user.created_at,
                user.updated_at,
                user.last_login_at,
            ],
        )?;

        let user_id = conn.last_insert_rowid();
        tracing::info!("Created user: {} ({})", user.name, user_id);
        Ok(user_id)
    }

    pub fn get_user_by_id(&self, id: i64) -> AppResult<crate::models::User> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let user = conn
            .query_row(
                "SELECT id, name, email, role, password_hash, is_active, created_at, updated_at, last_login_at
                 FROM users WHERE id = ?",
                [id],
                |row| {
                    Ok(crate::models::User {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        email: row.get(2)?,
                        role: row.get(3)?,
                        password_hash: row.get(4)?,
                        is_active: row.get::<_, i64>(5)? != 0,
                        created_at: row.get(6)?,
                        updated_at: row.get(7)?,
                        last_login_at: row.get(8)?,
                    })
                },
            )
            .optional()?;

        match user {
            Some(user) => Ok(user),
            None => Err(AppError::NotFound(format!("User not found: {}", id))),
        }
    }

    pub fn get_user_by_name(&self, name: &str) -> AppResult<crate::models::User> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let user = conn
            .query_row(
                "SELECT id, name, email, role, password_hash, is_active, created_at, updated_at, last_login_at
                 FROM users WHERE name = ?",
                [name],
                |row| {
                    Ok(crate::models::User {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        email: row.get(2)?,
                        role: row.get(3)?,
                        password_hash: row.get(4)?,
                        is_active: row.get::<_, i64>(5)? != 0,
                        created_at: row.get(6)?,
                        updated_at: row.get(7)?,
                        last_login_at: row.get(8)?,
                    })
                },
            )
            .optional()?;

        match user {
            Some(user) => Ok(user),
            None => Err(AppError::NotFound(format!("User not found: {}", name))),
        }
    }

    #[allow(dead_code)]
    pub fn get_user_by_email(&self, email: &str) -> AppResult<crate::models::User> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let user = conn
            .query_row(
                "SELECT id, name, email, role, password_hash, is_active, created_at, updated_at, last_login_at
                 FROM users WHERE email = ?",
                [email],
                |row| {
                    Ok(crate::models::User {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        email: row.get(2)?,
                        role: row.get(3)?,
                        password_hash: row.get(4)?,
                        is_active: row.get::<_, i64>(5)? != 0,
                        created_at: row.get(6)?,
                        updated_at: row.get(7)?,
                        last_login_at: row.get(8)?,
                    })
                },
            )
            .optional()?;

        match user {
            Some(user) => Ok(user),
            None => Err(AppError::NotFound(format!("User not found: {}", email))),
        }
    }

    // SSH key methods

    #[allow(dead_code)]
    pub fn create_ssh_key(&self, key: &crate::models::UserSshKey) -> AppResult<i64> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let id_param = if key.id == 0 { None } else { Some(key.id) };

        conn.execute(
            "INSERT INTO user_ssh_keys (id, user_id, key_type, fingerprint, public_key_data, label, is_active, created_at, last_used_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                id_param,
                key.user_id,
                key.key_type,
                key.fingerprint,
                key.public_key_data,
                key.label,
                if key.is_active { 1 } else { 0 },
                key.created_at,
                key.last_used_at,
            ],
        )?;

        let key_id = conn.last_insert_rowid();
        tracing::info!(
            "Created SSH key for user {}: {} ({})",
            key.user_id,
            key.fingerprint,
            key_id
        );
        Ok(key_id)
    }

    pub fn get_ssh_key_by_fingerprint(
        &self,
        fingerprint: &str,
    ) -> AppResult<crate::models::UserSshKey> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let key = conn
            .query_row(
                "SELECT id, user_id, key_type, fingerprint, public_key_data, label, is_active, created_at, last_used_at
                 FROM user_ssh_keys WHERE fingerprint = ? AND is_active = 1",
                [fingerprint],
                |row| {
                    Ok(crate::models::UserSshKey {
                        id: row.get(0)?,
                        user_id: row.get(1)?,
                        key_type: row.get(2)?,
                        fingerprint: row.get(3)?,
                        public_key_data: row.get(4)?,
                        label: row.get(5)?,
                        is_active: row.get::<_, i64>(6)? != 0,
                        created_at: row.get(7)?,
                        last_used_at: row.get(8)?,
                    })
                },
            )
            .optional()?;

        match key {
            Some(key) => Ok(key),
            None => Err(AppError::NotFound(format!(
                "SSH key not found or inactive: {}",
                fingerprint
            ))),
        }
    }

    #[allow(dead_code)]
    pub fn get_ssh_keys_for_user(&self, user_id: i64) -> AppResult<Vec<crate::models::UserSshKey>> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let mut stmt = conn.prepare(
            "SELECT id, user_id, key_type, fingerprint, public_key_data, label, is_active, created_at, last_used_at
             FROM user_ssh_keys WHERE user_id = ? ORDER BY created_at DESC",
        )?;

        let key_iter = stmt.query_map([user_id], |row| {
            Ok(crate::models::UserSshKey {
                id: row.get(0)?,
                user_id: row.get(1)?,
                key_type: row.get(2)?,
                fingerprint: row.get(3)?,
                public_key_data: row.get(4)?,
                label: row.get(5)?,
                is_active: row.get::<_, i64>(6)? != 0,
                created_at: row.get(7)?,
                last_used_at: row.get(8)?,
            })
        })?;

        let keys: Result<Vec<_>, _> = key_iter.collect();
        keys.map_err(AppError::from)
    }

    pub fn update_ssh_key_last_used(&self, key_id: i64) -> AppResult<()> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let now = chrono::Utc::now().timestamp();

        conn.execute(
            "UPDATE user_ssh_keys SET last_used_at = ? WHERE id = ?",
            params![now, key_id],
        )?;

        tracing::debug!("Updated SSH key last_used_at: {}", key_id);
        Ok(())
    }

    // User management methods

    pub fn get_all_users(&self) -> AppResult<Vec<crate::models::User>> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let mut stmt = conn.prepare(
            "SELECT id, name, email, role, password_hash, is_active, created_at, updated_at, last_login_at
             FROM users ORDER BY name",
        )?;

        let user_iter = stmt.query_map([], |row| {
            Ok(crate::models::User {
                id: row.get(0)?,
                name: row.get(1)?,
                email: row.get(2)?,
                role: row.get(3)?,
                password_hash: row.get(4)?,
                is_active: row.get::<_, i64>(5)? != 0,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
                last_login_at: row.get(8)?,
            })
        })?;

        let users: Result<Vec<_>, _> = user_iter.collect();
        users.map_err(AppError::from)
    }

    pub fn update_user(
        &self,
        user_id: i64,
        name: Option<&str>,
        email: Option<&str>,
    ) -> AppResult<()> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let now = chrono::Utc::now().timestamp();

        if let Some(name) = name {
            conn.execute(
                "UPDATE users SET name = ?, updated_at = ? WHERE id = ?",
                params![name, now, user_id],
            )?;
        }

        if let Some(email) = email {
            conn.execute(
                "UPDATE users SET email = ?, updated_at = ? WHERE id = ?",
                params![email, now, user_id],
            )?;
        }

        tracing::info!("Updated user {}", user_id);
        Ok(())
    }

    pub fn delete_user(&self, user_id: i64) -> AppResult<()> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        conn.execute("DELETE FROM users WHERE id = ?", params![user_id])?;
        tracing::info!("Deleted user {}", user_id);
        Ok(())
    }

    pub fn search_users(&self, query: &str) -> AppResult<Vec<crate::models::User>> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let search_pattern = format!("%{}%", query);
        let mut stmt = conn.prepare(
            "SELECT id, name, email, role, password_hash, is_active, created_at, updated_at, last_login_at
             FROM users
             WHERE name LIKE ? OR email LIKE ?
             ORDER BY name",
        )?;

        let user_iter = stmt.query_map(params![search_pattern, search_pattern], |row| {
            Ok(crate::models::User {
                id: row.get(0)?,
                name: row.get(1)?,
                email: row.get(2)?,
                role: row.get(3)?,
                password_hash: row.get(4)?,
                is_active: row.get::<_, i64>(5)? != 0,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
                last_login_at: row.get(8)?,
            })
        })?;

        let users: Result<Vec<_>, _> = user_iter.collect();
        users.map_err(AppError::from)
    }

    pub fn update_user_teams(&self, user_id: i64, team_ids: &[i64]) -> AppResult<()> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        // Remove user from all teams first
        conn.execute("DELETE FROM team_members WHERE user_id = ?", params![user_id])?;

        // Add user to specified teams
        for &team_id in team_ids {
            conn.execute(
                "INSERT INTO team_members (team_id, user_id, added_at) VALUES (?, ?, ?)",
                params![team_id, user_id, chrono::Utc::now().timestamp()],
            )?;
        }

        tracing::info!("Updated teams for user {}", user_id);
        Ok(())
    }

    // Permission management methods

    pub fn get_all_permissions(&self) -> AppResult<Vec<crate::models::Permission>> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let mut stmt = conn.prepare(
            "SELECT id, team_id, resource_type, resource_id, action, granted_by, granted_at
             FROM permissions ORDER BY granted_at DESC",
        )?;

        let permission_iter = stmt.query_map([], |row| {
            Ok(crate::models::Permission {
                id: row.get(0)?,
                team_id: row.get(1)?,
                resource_type: row.get(2)?,
                resource_id: row.get(3)?,
                action: row.get(4)?,
                granted_by: row.get(5)?,
                granted_at: row.get(6)?,
            })
        })?;

        let permissions: Result<Vec<_>, _> = permission_iter.collect();
        permissions.map_err(AppError::from)
    }

    // Permission system methods (Phase 2: Permission Checking)

    /// Get all teams that a user belongs to
    pub fn get_user_teams(&self, user_id: i64) -> AppResult<Vec<crate::models::Team>> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let mut stmt = conn.prepare(
            "SELECT t.id, t.name, t.description, t.created_by, t.created_at, t.updated_at
             FROM teams t
             INNER JOIN team_members tm ON t.id = tm.team_id
             WHERE tm.user_id = ?",
        )?;

        let team_iter = stmt.query_map([user_id], |row| {
            Ok(crate::models::Team {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                created_by: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?;

        let teams: Result<Vec<_>, _> = team_iter.collect();
        teams.map_err(AppError::from)
    }

    /// Check if a team has a specific permission
    /// Checks action hierarchy (admin implies all, write implies read, etc.)
    pub fn team_has_permission(
        &self,
        team_id: i64,
        resource_type: &str,
        resource_id: Option<i64>,
        action: &str,
    ) -> AppResult<bool> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        // Parse the requested action
        let requested_action = Action::parse(action)
            .ok_or_else(|| AppError::InvalidRequest(format!("Invalid action: {}", action)))?;

        // Query all permissions for this team on this resource
        let mut stmt = conn.prepare(
            "SELECT action FROM permissions
             WHERE team_id = ?
             AND resource_type = ?
             AND (resource_id = ? OR (? IS NULL AND resource_id IS NULL))",
        )?;

        let action_iter = stmt.query_map(
            params![team_id, resource_type, resource_id, resource_id],
            |row| row.get::<_, String>(0),
        )?;

        // Check if any permission implies the requested action
        for action_result in action_iter {
            let action_str = action_result?;
            if let Some(granted_action) = Action::parse(&action_str) {
                if granted_action.implies(&requested_action) {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Check if a user owns a resource
    pub fn is_owner(&self, user_id: i64, resource_type: &str, resource_id: i64) -> AppResult<bool> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM resource_ownership
             WHERE owner_id = ? AND resource_type = ? AND resource_id = ?",
            params![user_id, resource_type, resource_id],
            |row| row.get(0),
        )?;

        Ok(count > 0)
    }

    /// Get the parent project ID for a project (for permission inheritance)
    pub fn get_parent_project_id(&self, project_id: i64) -> AppResult<Option<i64>> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let parent_id: Option<i64> = conn
            .query_row(
                "SELECT parent_id FROM projects WHERE id = ?",
                [project_id],
                |row| row.get(0),
            )
            .optional()?;

        Ok(parent_id)
    }

    // Team CRUD methods

    /// Create a new team
    pub fn create_team(&self, team: &crate::models::Team) -> AppResult<i64> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        conn.execute(
            "INSERT INTO teams (name, description, created_by, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?)",
            params![
                team.name,
                team.description,
                team.created_by,
                team.created_at,
                team.updated_at,
            ],
        )?;

        let id = conn.last_insert_rowid();
        tracing::debug!("Created team: {} (id={})", team.name, id);
        Ok(id)
    }

    /// Get a team by ID
    pub fn get_team_by_id(&self, team_id: i64) -> AppResult<crate::models::Team> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let team = conn.query_row(
            "SELECT id, name, description, created_by, created_at, updated_at
             FROM teams WHERE id = ?",
            [team_id],
            |row| {
                Ok(crate::models::Team {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    created_by: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            },
        )?;

        Ok(team)
    }

    /// Get all teams
    pub fn get_all_teams(&self) -> AppResult<Vec<crate::models::Team>> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let mut stmt = conn.prepare(
            "SELECT id, name, description, created_by, created_at, updated_at
             FROM teams ORDER BY name",
        )?;

        let team_iter = stmt.query_map([], |row| {
            Ok(crate::models::Team {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                created_by: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?;

        let teams: Result<Vec<_>, _> = team_iter.collect();
        teams.map_err(AppError::from)
    }

    /// Update a team
    pub fn update_team(
        &self,
        team_id: i64,
        update: &crate::models::UpdateTeamRequest,
    ) -> AppResult<()> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let now = chrono::Utc::now().timestamp();

        if let Some(name) = &update.name {
            conn.execute(
                "UPDATE teams SET name = ?, updated_at = ? WHERE id = ?",
                params![name, now, team_id],
            )?;
        }

        if let Some(description) = &update.description {
            conn.execute(
                "UPDATE teams SET description = ?, updated_at = ? WHERE id = ?",
                params![description, now, team_id],
            )?;
        }

        tracing::info!("Updated team {}", team_id);
        Ok(())
    }

    /// Delete a team
    pub fn delete_team(&self, team_id: i64) -> AppResult<()> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        conn.execute("DELETE FROM teams WHERE id = ?", [team_id])?;

        tracing::debug!("Deleted team: {}", team_id);
        Ok(())
    }

    // Team member methods

    /// Add a user to a team
    pub fn add_team_member(
        &self,
        team_id: i64,
        user_id: i64,
        added_by: Option<i64>,
    ) -> AppResult<i64> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let now = chrono::Utc::now().timestamp();

        conn.execute(
            "INSERT INTO team_members (team_id, user_id, added_by, added_at)
             VALUES (?, ?, ?, ?)",
            params![team_id, user_id, added_by, &now],
        )?;

        let member_id = conn.last_insert_rowid();
        tracing::debug!("Added user {} to team {}", user_id, team_id);
        Ok(member_id)
    }

    /// Remove a user from a team
    pub fn remove_team_member(&self, team_id: i64, user_id: i64) -> AppResult<()> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        conn.execute(
            "DELETE FROM team_members WHERE team_id = ? AND user_id = ?",
            params![team_id, user_id],
        )?;

        tracing::debug!("Removed user {} from team {}", user_id, team_id);
        Ok(())
    }

    /// Get all members of a team
    pub fn get_team_members(&self, team_id: i64) -> AppResult<Vec<crate::models::TeamMember>> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let mut stmt = conn.prepare(
            "SELECT id, team_id, user_id, added_by, added_at
             FROM team_members WHERE team_id = ?",
        )?;

        let member_iter = stmt.query_map([team_id], |row| {
            Ok(crate::models::TeamMember {
                id: row.get(0)?,
                team_id: row.get(1)?,
                user_id: row.get(2)?,
                added_by: row.get(3)?,
                added_at: row.get(4)?,
            })
        })?;

        let members: Result<Vec<_>, _> = member_iter.collect();
        members.map_err(AppError::from)
    }

    // Permission methods

    /// Create a new permission
    pub fn create_permission(&self, permission: &crate::models::Permission) -> AppResult<i64> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        conn.execute(
            "INSERT INTO permissions (team_id, resource_type, resource_id, action, granted_by, granted_at)
             VALUES (?, ?, ?, ?, ?, ?)",
            params![
                permission.team_id,
                permission.resource_type,
                permission.resource_id,
                permission.action,
                permission.granted_by,
                permission.granted_at,
            ],
        )?;

        let id = conn.last_insert_rowid();
        tracing::debug!(
            "Created permission: team={}, resource={}:{:?}, action={} (id={})",
            permission.team_id,
            permission.resource_type,
            permission.resource_id,
            permission.action,
            id
        );
        Ok(id)
    }

    /// Get all permissions for a team
    pub fn get_team_permissions(&self, team_id: i64) -> AppResult<Vec<crate::models::Permission>> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let mut stmt = conn.prepare(
            "SELECT id, team_id, resource_type, resource_id, action, granted_by, granted_at
             FROM permissions WHERE team_id = ?",
        )?;

        let perm_iter = stmt.query_map([team_id], |row| {
            Ok(crate::models::Permission {
                id: row.get(0)?,
                team_id: row.get(1)?,
                resource_type: row.get(2)?,
                resource_id: row.get(3)?,
                action: row.get(4)?,
                granted_by: row.get(5)?,
                granted_at: row.get(6)?,
            })
        })?;

        let permissions: Result<Vec<_>, _> = perm_iter.collect();
        permissions.map_err(AppError::from)
    }

    /// Delete a permission
    pub fn delete_permission(&self, permission_id: i64) -> AppResult<()> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        conn.execute("DELETE FROM permissions WHERE id = ?", [permission_id])?;

        tracing::debug!("Deleted permission: {}", permission_id);
        Ok(())
    }

    // Resource ownership methods

    /// Create a resource ownership record
    pub fn create_ownership(&self, ownership: &crate::models::ResourceOwnership) -> AppResult<i64> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        conn.execute(
            "INSERT INTO resource_ownership (resource_type, resource_id, owner_id, created_at)
             VALUES (?, ?, ?, ?)",
            params![
                ownership.resource_type,
                ownership.resource_id,
                ownership.owner_id,
                ownership.created_at,
            ],
        )?;

        let id = conn.last_insert_rowid();
        tracing::debug!(
            "Created ownership: user={} owns {}:{} (id={})",
            ownership.owner_id,
            ownership.resource_type,
            ownership.resource_id,
            id
        );
        Ok(id)
    }

    /// Get the owner of a resource
    #[allow(dead_code)]
    pub fn get_resource_owner(
        &self,
        resource_type: &str,
        resource_id: i64,
    ) -> AppResult<Option<i64>> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        let owner_id: Option<i64> = conn
            .query_row(
                "SELECT owner_id FROM resource_ownership
                 WHERE resource_type = ? AND resource_id = ?",
                params![resource_type, resource_id],
                |row| row.get(0),
            )
            .optional()?;

        Ok(owner_id)
    }

    /// Delete a resource ownership record
    #[allow(dead_code)]
    pub fn delete_ownership(&self, resource_type: &str, resource_id: i64) -> AppResult<()> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| AppError::Internal(format!("Failed to acquire database lock: {e}")))?;

        conn.execute(
            "DELETE FROM resource_ownership WHERE resource_type = ? AND resource_id = ?",
            params![resource_type, resource_id],
        )?;

        tracing::debug!("Deleted ownership: {}:{}", resource_type, resource_id);
        Ok(())
    }
}
