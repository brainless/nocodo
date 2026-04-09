use std::sync::Mutex;

use async_trait::async_trait;
use rusqlite::{params, Connection, OptionalExtension};

use super::{AgentStorage, ChatMessage, SchemaStorage, Session, ToolCallRecord};
use crate::error::AgentError;

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

// ---------------------------------------------------------------------------
// SqliteAgentStorage
// ---------------------------------------------------------------------------

pub struct SqliteAgentStorage {
    conn: Mutex<Connection>,
}

impl SqliteAgentStorage {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Mutex::new(conn),
        }
    }

    pub fn open(path: &str) -> Result<Self, AgentError> {
        let conn = Connection::open(path)?;
        Ok(Self::new(conn))
    }
}

#[async_trait]
impl AgentStorage for SqliteAgentStorage {
    async fn get_or_create_session(
        &self,
        project_id: i64,
        agent_type: &str,
    ) -> Result<Session, AgentError> {
        let conn = self.conn.lock().unwrap();
        let existing = conn
            .query_row(
                "SELECT id, project_id, agent_type, created_at
                 FROM agent_chat_session
                 WHERE project_id = ?1 AND agent_type = ?2
                 LIMIT 1",
                params![project_id, agent_type],
                |row| {
                    Ok(Session {
                        id: Some(row.get(0)?),
                        project_id: row.get(1)?,
                        agent_type: row.get(2)?,
                        created_at: row.get(3)?,
                    })
                },
            )
            .optional()?;

        if let Some(session) = existing {
            return Ok(session);
        }

        let created_at = now();
        conn.execute(
            "INSERT INTO agent_chat_session (project_id, agent_type, created_at)
             VALUES (?1, ?2, ?3)",
            params![project_id, agent_type, created_at],
        )?;
        let id = conn.last_insert_rowid();
        Ok(Session {
            id: Some(id),
            project_id,
            agent_type: agent_type.to_string(),
            created_at,
        })
    }

    async fn create_message(&self, msg: ChatMessage) -> Result<i64, AgentError> {
        let created_at = if msg.created_at == 0 { now() } else { msg.created_at };
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO agent_chat_message (session_id, role, content, tool_call_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![msg.session_id, msg.role, msg.content, msg.tool_call_id, created_at],
        )?;
        Ok(conn.last_insert_rowid())
    }

    async fn get_messages(&self, session_id: i64) -> Result<Vec<ChatMessage>, AgentError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, role, content, tool_call_id, created_at
             FROM agent_chat_message
             WHERE session_id = ?1
             ORDER BY id ASC",
        )?;
        let rows = stmt.query_map(params![session_id], |row| {
            Ok(ChatMessage {
                id: Some(row.get(0)?),
                session_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                tool_call_id: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;
        let mut messages = Vec::new();
        for row in rows {
            messages.push(row?);
        }
        Ok(messages)
    }

    async fn create_tool_call(&self, record: ToolCallRecord) -> Result<i64, AgentError> {
        let created_at = if record.created_at == 0 { now() } else { record.created_at };
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO agent_tool_call (message_id, call_id, tool_name, arguments, result, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                record.message_id,
                record.call_id,
                record.tool_name,
                record.arguments,
                record.result,
                created_at
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    async fn update_tool_call_result(&self, id: i64, result: &str) -> Result<(), AgentError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE agent_tool_call SET result = ?1 WHERE id = ?2",
            params![result, id],
        )?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// SqliteSchemaStorage
// ---------------------------------------------------------------------------

pub struct SqliteSchemaStorage {
    conn: Mutex<Connection>,
}

impl SqliteSchemaStorage {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Mutex::new(conn),
        }
    }

    pub fn open(path: &str) -> Result<Self, AgentError> {
        let conn = Connection::open(path)?;
        Ok(Self::new(conn))
    }
}

#[async_trait]
impl SchemaStorage for SqliteSchemaStorage {
    async fn next_version(&self, project_id: i64) -> Result<i64, AgentError> {
        let conn = self.conn.lock().unwrap();
        let max: Option<i64> = conn
            .query_row(
                "SELECT MAX(version) FROM project_schema WHERE project_id = ?1",
                params![project_id],
                |row| row.get(0),
            )
            .optional()?
            .flatten();
        Ok(max.unwrap_or(0) + 1)
    }

    async fn save_schema(
        &self,
        project_id: i64,
        session_id: i64,
        schema_json: &str,
    ) -> Result<i64, AgentError> {
        let version = self.next_version(project_id).await?;
        let created_at = now();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO project_schema (project_id, session_id, schema_json, version, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![project_id, session_id, schema_json, version, created_at],
        )?;
        Ok(conn.last_insert_rowid())
    }
}
