pub mod migrations;
pub mod models;

use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub type DbConnection = Arc<Mutex<Connection>>;

pub struct Database {
    pub(crate) connection: DbConnection,
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
                status TEXT NOT NULL DEFAULT 'running' CHECK (status IN ('running', 'completed', 'failed')),
                started_at INTEGER NOT NULL,
                ended_at INTEGER,
                result TEXT,
                error TEXT
            )",
            [],
        )?;

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
    ) -> anyhow::Result<i64> {
        let conn = self.connection.lock().unwrap();
        let now = chrono::Utc::now().timestamp();

        conn.execute(
            "INSERT INTO agent_sessions (agent_name, provider, model, system_prompt, user_prompt, started_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![agent_name, provider, model, system_prompt, user_prompt, now],
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
