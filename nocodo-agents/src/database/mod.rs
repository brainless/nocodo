pub mod migrations;
pub mod models;

use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub type DbConnection = Arc<Mutex<Connection>>;

pub struct Database {
    pub connection: DbConnection,
}

impl Database {
    pub fn new(db_path: &PathBuf) -> anyhow::Result<Self> {
        // Ensure directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(db_path)?;
        conn.execute("PRAGMA foreign_keys = ON", [])?;

        let database = Database {
            connection: Arc::new(Mutex::new(conn)),
        };

        database.run_migrations()?;
        Ok(database)
    }

    fn run_migrations(&self) -> anyhow::Result<()> {
        let conn = self.connection.lock().unwrap();

        // Create agent_sessions table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS agent_sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                agent_name TEXT NOT NULL,
                provider TEXT NOT NULL,
                model TEXT NOT NULL,
                system_prompt TEXT,
                user_prompt TEXT NOT NULL,
                config TEXT,
                status TEXT NOT NULL DEFAULT 'running' CHECK (status IN ('running', 'completed', 'failed', 'waiting_for_user_input')),
                started_at INTEGER NOT NULL,
                ended_at INTEGER,
                result TEXT,
                error TEXT
            )",
            [],
        )?;

        // Add config column if it doesn't exist (for existing databases)
        let has_config: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('agent_sessions') WHERE name = 'config'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0)
            == 1;

        if !has_config {
            conn.execute("ALTER TABLE agent_sessions ADD COLUMN config TEXT", [])?;
        }

        // Migrate agent_sessions to support waiting_for_user_input status
        // Check the table schema to see if it needs migration
        let needs_migration: bool = {
            let schema: Option<String> = conn
                .query_row(
                    "SELECT sql FROM sqlite_master WHERE type='table' AND name='agent_sessions'",
                    [],
                    |row| row.get(0),
                )
                .ok();

            if let Some(sql) = schema {
                // Check if the schema contains the new status value
                !sql.contains("waiting_for_user_input")
            } else {
                false // Table doesn't exist yet, will be created with correct constraint
            }
        };

        if needs_migration {
            tracing::info!(
                "Migrating agent_sessions table to support waiting_for_user_input status"
            );

            // Temporarily disable foreign keys for migration
            conn.execute("PRAGMA foreign_keys = OFF", [])?;

            // Create new table with updated constraint
            conn.execute(
                "CREATE TABLE agent_sessions_new (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    agent_name TEXT NOT NULL,
                    provider TEXT NOT NULL,
                    model TEXT NOT NULL,
                    system_prompt TEXT,
                    user_prompt TEXT NOT NULL,
                    config TEXT,
                    status TEXT NOT NULL DEFAULT 'running' CHECK (status IN ('running', 'completed', 'failed', 'waiting_for_user_input')),
                    started_at INTEGER NOT NULL,
                    ended_at INTEGER,
                    result TEXT,
                    error TEXT
                )",
                [],
            )?;

            // Copy data from old table
            conn.execute(
                "INSERT INTO agent_sessions_new
                    SELECT id, agent_name, provider, model, system_prompt, user_prompt,
                           config, status, started_at, ended_at, result, error
                    FROM agent_sessions",
                [],
            )?;

            // Drop old table
            conn.execute("DROP TABLE agent_sessions", [])?;

            // Rename new table
            conn.execute(
                "ALTER TABLE agent_sessions_new RENAME TO agent_sessions",
                [],
            )?;

            // Re-enable foreign keys
            conn.execute("PRAGMA foreign_keys = ON", [])?;

            tracing::info!("Successfully migrated agent_sessions table");
        }

        // Create agent_messages table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS agent_messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id INTEGER NOT NULL,
                role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system', 'tool')),
                content TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (session_id) REFERENCES agent_sessions (id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Create agent_tool_calls table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS agent_tool_calls (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id INTEGER NOT NULL,
                message_id INTEGER,
                tool_call_id TEXT NOT NULL,
                tool_name TEXT NOT NULL,
                request TEXT NOT NULL,
                response TEXT,
                status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'executing', 'completed', 'failed')),
                created_at INTEGER NOT NULL,
                completed_at INTEGER,
                execution_time_ms INTEGER,
                error_details TEXT,
                FOREIGN KEY (session_id) REFERENCES agent_sessions (id) ON DELETE CASCADE,
                FOREIGN KEY (message_id) REFERENCES agent_messages (id) ON DELETE SET NULL
            )",
            [],
        )?;

        // Create project_requirements_qna table for storing questions and answers
        conn.execute(
            "CREATE TABLE IF NOT EXISTS project_requirements_qna (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id INTEGER NOT NULL,
                tool_call_id INTEGER,
                question_id TEXT NOT NULL,
                question TEXT NOT NULL,
                description TEXT,
                response_type TEXT NOT NULL DEFAULT 'text',
                answer TEXT,
                created_at INTEGER NOT NULL,
                answered_at INTEGER,
                FOREIGN KEY (session_id) REFERENCES agent_sessions (id) ON DELETE CASCADE,
                FOREIGN KEY (tool_call_id) REFERENCES agent_tool_calls (id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Add tool_call_id column if it doesn't exist (for existing databases)
        let has_tool_call_id: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('project_requirements_qna') WHERE name = 'tool_call_id'",
                [],
                |row| row.get(0),
            )
            .unwrap_or(0)
            == 1;

        if !has_tool_call_id {
            tracing::info!("Adding tool_call_id column to project_requirements_qna table");
            conn.execute(
                "ALTER TABLE project_requirements_qna ADD COLUMN tool_call_id INTEGER REFERENCES agent_tool_calls(id) ON DELETE CASCADE",
                [],
            )?;
        }

        // Create indexes for performance
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_agent_messages_session_created
                ON agent_messages(session_id, created_at)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_agent_tool_calls_session
                ON agent_tool_calls(session_id)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_agent_tool_calls_status
                ON agent_tool_calls(session_id, status)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_project_requirements_qna_session
                ON project_requirements_qna(session_id)",
            [],
        )?;

        Ok(())
    }

    // Session management
    pub fn create_session(
        &self,
        agent_name: &str,
        provider: &str,
        model: &str,
        system_prompt: Option<&str>,
        user_prompt: &str,
        config: Option<serde_json::Value>,
    ) -> anyhow::Result<i64> {
        let conn = self.connection.lock().unwrap();
        let now = chrono::Utc::now().timestamp();

        let config_json = config
            .map(|c| serde_json::to_string(&c).unwrap())
            .unwrap_or_else(|| "null".to_string());

        conn.execute(
            "INSERT INTO agent_sessions (agent_name, provider, model, system_prompt, user_prompt, config, started_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![agent_name, provider, model, system_prompt, user_prompt, config_json, now],
        )?;

        Ok(conn.last_insert_rowid())
    }

    pub fn complete_session(&self, session_id: i64, result: &str) -> anyhow::Result<()> {
        let conn = self.connection.lock().unwrap();
        let now = chrono::Utc::now().timestamp();

        conn.execute(
            "UPDATE agent_sessions SET status = 'completed', ended_at = ?1, result = ?2
                WHERE id = ?3",
            params![now, result, session_id],
        )?;

        Ok(())
    }

    pub fn fail_session(&self, session_id: i64, error: &str) -> anyhow::Result<()> {
        let conn = self.connection.lock().unwrap();
        let now = chrono::Utc::now().timestamp();

        conn.execute(
            "UPDATE agent_sessions SET status = 'failed', ended_at = ?1, error = ?2
                WHERE id = ?3",
            params![now, error, session_id],
        )?;

        Ok(())
    }

    pub fn pause_session_for_user_input(&self, session_id: i64) -> anyhow::Result<()> {
        let conn = self.connection.lock().unwrap();

        conn.execute(
            "UPDATE agent_sessions SET status = 'waiting_for_user_input'
                WHERE id = ?1",
            params![session_id],
        )?;

        Ok(())
    }

    pub fn resume_session(&self, session_id: i64) -> anyhow::Result<()> {
        let conn = self.connection.lock().unwrap();

        conn.execute(
            "UPDATE agent_sessions SET status = 'running'
                WHERE id = ?1",
            params![session_id],
        )?;

        Ok(())
    }

    // Project requirements Q&A management
    pub fn store_questions(
        &self,
        session_id: i64,
        tool_call_id: Option<i64>,
        questions: &[shared_types::user_interaction::UserQuestion],
    ) -> anyhow::Result<()> {
        let conn = self.connection.lock().unwrap();
        let now = chrono::Utc::now().timestamp();

        for question in questions {
            conn.execute(
                "INSERT INTO project_requirements_qna (session_id, tool_call_id, question_id, question, description, response_type, created_at)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    session_id,
                    tool_call_id,
                    &question.id,
                    &question.question,
                    &question.description,
                    format!("{:?}", question.response_type).to_lowercase(),
                    now
                ],
            )?;
        }

        Ok(())
    }

    pub fn get_pending_questions(
        &self,
        session_id: i64,
    ) -> anyhow::Result<Vec<shared_types::user_interaction::UserQuestion>> {
        let conn = self.connection.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT question_id, question, description, response_type
                FROM project_requirements_qna
                WHERE session_id = ?1 AND answer IS NULL
                ORDER BY created_at ASC",
        )?;

        let questions = stmt.query_map([session_id], |row| {
            let response_type_str: String = row.get(3)?;
            let response_type = match response_type_str.as_str() {
                "text" => shared_types::user_interaction::QuestionType::Text,
                _ => shared_types::user_interaction::QuestionType::Text,
            };

            Ok(shared_types::user_interaction::UserQuestion {
                id: row.get(0)?,
                question: row.get(1)?,
                description: row.get(2)?,
                response_type,
                default: None,
                options: None,
            })
        })?;

        let mut result = Vec::new();
        for question in questions {
            result.push(question?);
        }

        Ok(result)
    }

    pub fn store_answers(
        &self,
        session_id: i64,
        answers: &std::collections::HashMap<String, String>,
    ) -> anyhow::Result<()> {
        let conn = self.connection.lock().unwrap();
        let now = chrono::Utc::now().timestamp();

        for (question_id, answer) in answers {
            conn.execute(
                "UPDATE project_requirements_qna
                    SET answer = ?1, answered_at = ?2
                    WHERE session_id = ?3 AND question_id = ?4",
                params![answer, now, session_id, question_id],
            )?;
        }

        Ok(())
    }

    // Message management
    pub fn create_message(
        &self,
        session_id: i64,
        role: &str,
        content: &str,
    ) -> anyhow::Result<i64> {
        let conn = self.connection.lock().unwrap();
        let now = chrono::Utc::now().timestamp();

        conn.execute(
            "INSERT INTO agent_messages (session_id, role, content, created_at)
                VALUES (?1, ?2, ?3, ?4)",
            params![session_id, role, content, now],
        )?;

        Ok(conn.last_insert_rowid())
    }

    pub fn get_messages(&self, session_id: i64) -> anyhow::Result<Vec<models::AgentMessage>> {
        let conn = self.connection.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, role, content, created_at
                FROM agent_messages
                WHERE session_id = ?1
                ORDER BY created_at ASC",
        )?;

        let messages = stmt.query_map([session_id], |row| {
            Ok(models::AgentMessage {
                id: row.get(0)?,
                session_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;

        let mut result = Vec::new();
        for message in messages {
            result.push(message?);
        }

        Ok(result)
    }

    // Tool call management
    pub fn create_tool_call(
        &self,
        session_id: i64,
        message_id: Option<i64>,
        tool_call_id: &str,
        tool_name: &str,
        request: serde_json::Value,
    ) -> anyhow::Result<i64> {
        let conn = self.connection.lock().unwrap();
        let now = chrono::Utc::now().timestamp();
        let request_json = serde_json::to_string(&request)
            .map_err(|e| anyhow::anyhow!("Failed to serialize request: {}", e))?;

        conn.execute(
            "INSERT INTO agent_tool_calls (session_id, message_id, tool_call_id, tool_name, request, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![session_id, message_id, tool_call_id, tool_name, request_json, now],
        )?;

        Ok(conn.last_insert_rowid())
    }

    pub fn complete_tool_call(
        &self,
        call_id: i64,
        response: serde_json::Value,
        execution_time_ms: i64,
    ) -> anyhow::Result<()> {
        let conn = self.connection.lock().unwrap();
        let now = chrono::Utc::now().timestamp();
        let response_json = serde_json::to_string(&response)?;

        conn.execute(
            "UPDATE agent_tool_calls SET status = 'completed', response = ?1, completed_at = ?2, execution_time_ms = ?3
                WHERE id = ?4",
            params![response_json, now, execution_time_ms, call_id],
        )?;

        Ok(())
    }

    pub fn fail_tool_call(&self, call_id: i64, error: &str) -> anyhow::Result<()> {
        let conn = self.connection.lock().unwrap();
        let now = chrono::Utc::now().timestamp();

        conn.execute(
            "UPDATE agent_tool_calls SET status = 'failed', error_details = ?1, completed_at = ?2
                WHERE id = ?3",
            params![error, now, call_id],
        )?;

        Ok(())
    }

    pub fn get_pending_tool_calls(
        &self,
        session_id: i64,
    ) -> anyhow::Result<Vec<models::AgentToolCall>> {
        let conn = self.connection.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, message_id, tool_call_id, tool_name, request, response, status, created_at, completed_at, execution_time_ms, error_details
                FROM agent_tool_calls
                WHERE session_id = ?1 AND status = 'pending'
                ORDER BY created_at ASC"
        )?;

        let calls = stmt.query_map([session_id], |row| {
            let request_str: String = row.get(5)?;
            let request: serde_json::Value = serde_json::from_str(&request_str).map_err(|_e| {
                rusqlite::Error::InvalidColumnType(
                    5,
                    request_str.clone(),
                    rusqlite::types::Type::Text,
                )
            })?;

            let response: Option<serde_json::Value> = row
                .get(6)
                .ok()
                .and_then(|s: String| serde_json::from_str(&s).ok());

            Ok(models::AgentToolCall {
                id: row.get(0)?,
                session_id: row.get(1)?,
                message_id: row.get(2)?,
                tool_call_id: row.get(3)?,
                tool_name: row.get(4)?,
                request,
                response,
                status: row.get(7)?,
                created_at: row.get(8)?,
                completed_at: row.get(9)?,
                execution_time_ms: row.get(10)?,
                error_details: row.get(11)?,
            })
        })?;

        let mut result = Vec::new();
        for call in calls {
            result.push(call?);
        }

        Ok(result)
    }
}
