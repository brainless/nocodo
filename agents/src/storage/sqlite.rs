use std::sync::Mutex;

use async_trait::async_trait;
use rusqlite::{params, Connection, OptionalExtension};

use super::{AgentStorage, ChatMessage, SchemaStorage, Session};
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

    pub async fn create_session(
        &self,
        project_id: i64,
        agent_type: &str,
    ) -> Result<Session, AgentError> {
        let created_at = now();
        let conn = self.conn.lock().unwrap();
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

    pub async fn list_sessions(
        &self,
        project_id: i64,
        agent_type: Option<&str>,
    ) -> Result<Vec<Session>, AgentError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = match agent_type {
            Some(at) => {
                let mut s = conn.prepare(
                    "SELECT id, project_id, agent_type, created_at
                     FROM agent_chat_session
                     WHERE project_id = ?1 AND agent_type = ?2
                     ORDER BY id ASC",
                )?;
                let rows = s.query_map(params![project_id, at], |row| {
                    Ok(Session {
                        id: Some(row.get(0)?),
                        project_id: row.get(1)?,
                        agent_type: row.get(2)?,
                        created_at: row.get(3)?,
                    })
                })?;
                let mut sessions = Vec::new();
                for row in rows {
                    sessions.push(row?);
                }
                return Ok(sessions);
            }
            None => conn.prepare(
                "SELECT id, project_id, agent_type, created_at
                 FROM agent_chat_session
                 WHERE project_id = ?1
                 ORDER BY id ASC",
            )?,
        };
        let rows = stmt.query_map(params![project_id], |row| {
            Ok(Session {
                id: Some(row.get(0)?),
                project_id: row.get(1)?,
                agent_type: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;
        let mut sessions = Vec::new();
        for row in rows {
            sessions.push(row?);
        }
        Ok(sessions)
    }

    pub async fn get_session_by_id(&self, session_id: i64) -> Result<Option<Session>, AgentError> {
        let conn = self.conn.lock().unwrap();
        let session = conn
            .query_row(
                "SELECT id, project_id, agent_type, created_at
                 FROM agent_chat_session
                 WHERE id = ?1
                 LIMIT 1",
                params![session_id],
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
        Ok(session)
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
            "INSERT INTO agent_chat_message
                 (session_id, role, agent_type, content, tool_call_id, tool_name, turn_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7)",
            params![
                msg.session_id, msg.role, msg.agent_type, msg.content,
                msg.tool_call_id, msg.tool_name, created_at
            ],
        )?;
        let id = conn.last_insert_rowid();
        conn.execute(
            "UPDATE agent_chat_message SET turn_id = ?1 WHERE id = ?1",
            params![id],
        )?;
        Ok(id)
    }

    async fn create_turn(&self, messages: Vec<ChatMessage>) -> Result<i64, AgentError> {
        if messages.is_empty() {
            return Err(AgentError::Other("create_turn requires at least one message".into()));
        }
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction()?;

        // Insert first row with placeholder turn_id, then patch it with its own id.
        let first = &messages[0];
        let created_at = now();
        tx.execute(
            "INSERT INTO agent_chat_message
                 (session_id, role, agent_type, content, tool_call_id, tool_name, turn_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7)",
            params![
                first.session_id, first.role, first.agent_type, first.content,
                first.tool_call_id, first.tool_name, created_at
            ],
        )?;
        let turn_id = tx.last_insert_rowid();
        tx.execute(
            "UPDATE agent_chat_message SET turn_id = ?1 WHERE id = ?1",
            params![turn_id],
        )?;

        for msg in &messages[1..] {
            let created_at = now();
            tx.execute(
                "INSERT INTO agent_chat_message
                     (session_id, role, agent_type, content, tool_call_id, tool_name, turn_id, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    msg.session_id, msg.role, msg.agent_type, msg.content,
                    msg.tool_call_id, msg.tool_name, turn_id, created_at
                ],
            )?;
        }

        tx.commit()?;
        Ok(turn_id)
    }

    async fn get_messages(&self, session_id: i64) -> Result<Vec<ChatMessage>, AgentError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, role, agent_type, content, tool_call_id, tool_name, turn_id, created_at
             FROM agent_chat_message
             WHERE session_id = ?1
             ORDER BY id ASC",
        )?;
        let rows = stmt.query_map(params![session_id], |row| {
            Ok(ChatMessage {
                id: Some(row.get(0)?),
                session_id: row.get(1)?,
                role: row.get(2)?,
                agent_type: row.get(3)?,
                content: row.get(4)?,
                tool_call_id: row.get(5)?,
                tool_name: row.get(6)?,
                turn_id: row.get(7)?,
                created_at: row.get(8)?,
            })
        })?;
        let mut messages = Vec::new();
        for row in rows {
            messages.push(row?);
        }
        Ok(messages)
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

    pub async fn get_latest_schema_for_session(
        &self,
        session_id: i64,
    ) -> Result<Option<(String, i64)>, AgentError> {
        self.get_schema_for_session(session_id, None).await
    }

    pub async fn get_schema_version_by_json(
        &self,
        session_id: i64,
        schema_json: &str,
    ) -> Result<Option<i64>, AgentError> {
        let conn = self.conn.lock().unwrap();
        let result = conn
            .query_row(
                "SELECT version FROM project_schema
                 WHERE session_id = ?1 AND schema_json = ?2
                 LIMIT 1",
                params![session_id, schema_json],
                |row| row.get::<_, i64>(0),
            )
            .optional()?;
        Ok(result)
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

    async fn get_schema_for_session(
        &self,
        session_id: i64,
        version: Option<i64>,
    ) -> Result<Option<(String, i64)>, AgentError> {
        let conn = self.conn.lock().unwrap();
        let result = match version {
            Some(v) => conn
                .query_row(
                    "SELECT schema_json, version FROM project_schema
                     WHERE session_id = ?1 AND version = ?2
                     LIMIT 1",
                    params![session_id, v],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
                )
                .optional()?,
            None => conn
                .query_row(
                    "SELECT schema_json, version FROM project_schema
                     WHERE session_id = ?1
                     ORDER BY version DESC, id DESC LIMIT 1",
                    params![session_id],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
                )
                .optional()?,
        };
        Ok(result)
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
        let row_id = conn.last_insert_rowid();
        log::info!(
            "[SchemaStorage] Saved schema: project_id={}, session_id={}, version={}, row_id={}",
            project_id,
            session_id,
            version,
            row_id
        );
        Ok(row_id)
    }
}
