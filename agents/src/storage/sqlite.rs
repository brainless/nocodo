use std::sync::Mutex;

use async_trait::async_trait;
use rusqlite::{params, Connection, OptionalExtension};

use super::{
    AgentStorage, ChatMessage, Epic, EpicStatus, SchemaStorage, Session, Task, TaskStatus,
    TaskStorage,
};
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

    pub async fn list_sessions(
        &self,
        project_id: i64,
        agent_type: Option<&str>,
    ) -> Result<Vec<Session>, AgentError> {
        let conn = self.conn.lock().unwrap();
        let sessions = match agent_type {
            Some(at) => {
                let mut stmt = conn.prepare(
                    "SELECT id, project_id, agent_type, task_id, created_at
                     FROM agent_chat_session
                     WHERE project_id = ?1 AND agent_type = ?2
                     ORDER BY id ASC",
                )?;
                let rows = stmt.query_map(params![project_id, at], map_session)?
                    .collect::<Result<Vec<_>, _>>()?;
                rows
            }
            None => {
                let mut stmt = conn.prepare(
                    "SELECT id, project_id, agent_type, task_id, created_at
                     FROM agent_chat_session
                     WHERE project_id = ?1
                     ORDER BY id ASC",
                )?;
                let rows = stmt.query_map(params![project_id], map_session)?
                    .collect::<Result<Vec<_>, _>>()?;
                rows
            }
        };
        Ok(sessions)
    }

    pub async fn get_session_by_id(&self, session_id: i64) -> Result<Option<Session>, AgentError> {
        let conn = self.conn.lock().unwrap();
        let session = conn
            .query_row(
                "SELECT id, project_id, agent_type, task_id, created_at
                 FROM agent_chat_session WHERE id = ?1 LIMIT 1",
                params![session_id],
                map_session,
            )
            .optional()?;
        Ok(session)
    }
}

fn map_session(row: &rusqlite::Row<'_>) -> rusqlite::Result<Session> {
    Ok(Session {
        id: Some(row.get(0)?),
        project_id: row.get(1)?,
        agent_type: row.get(2)?,
        task_id: row.get(3)?,
        created_at: row.get(4)?,
    })
}

#[async_trait]
impl AgentStorage for SqliteAgentStorage {
    async fn create_task_session(
        &self,
        project_id: i64,
        task_id: i64,
        agent_type: &str,
    ) -> Result<Session, AgentError> {
        let created_at = now();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO agent_chat_session (project_id, agent_type, task_id, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![project_id, agent_type, task_id, created_at],
        )?;
        let id = conn.last_insert_rowid();
        Ok(Session {
            id: Some(id),
            project_id,
            agent_type: agent_type.to_string(),
            task_id,
            created_at,
        })
    }

    async fn get_session_by_task(
        &self,
        task_id: i64,
        agent_type: &str,
    ) -> Result<Option<Session>, AgentError> {
        let conn = self.conn.lock().unwrap();
        let session = conn
            .query_row(
                "SELECT id, project_id, agent_type, task_id, created_at
                 FROM agent_chat_session
                 WHERE task_id = ?1 AND agent_type = ?2
                 LIMIT 1",
                params![task_id, agent_type],
                map_session,
            )
            .optional()?;
        Ok(session)
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
        let messages = stmt
            .query_map(params![session_id], |row| {
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
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(messages)
    }
}

// ---------------------------------------------------------------------------
// SqliteTaskStorage
// ---------------------------------------------------------------------------

pub struct SqliteTaskStorage {
    conn: Mutex<Connection>,
}

impl SqliteTaskStorage {
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

fn map_task(row: &rusqlite::Row<'_>) -> rusqlite::Result<Task> {
    Ok(Task {
        id: Some(row.get(0)?),
        project_id: row.get(1)?,
        epic_id: row.get(2)?,
        title: row.get(3)?,
        description: row.get(4)?,
        source_prompt: row.get(5)?,
        assigned_to_agent: row.get(6)?,
        status: TaskStatus::from_str(&row.get::<_, String>(7)?),
        depends_on_task_id: row.get(8)?,
        created_by_agent: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

fn map_epic(row: &rusqlite::Row<'_>) -> rusqlite::Result<Epic> {
    Ok(Epic {
        id: Some(row.get(0)?),
        project_id: row.get(1)?,
        title: row.get(2)?,
        description: row.get(3)?,
        source_prompt: row.get(4)?,
        status: EpicStatus::from_str(&row.get::<_, String>(5)?),
        created_by_agent: row.get(6)?,
        created_by_task_id: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

#[async_trait]
impl TaskStorage for SqliteTaskStorage {
    async fn create_task(&self, task: Task) -> Result<i64, AgentError> {
        let ts = now();
        let created_at = if task.created_at == 0 { ts } else { task.created_at };
        let updated_at = if task.updated_at == 0 { ts } else { task.updated_at };
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO task
                 (project_id, epic_id, title, description, source_prompt,
                  assigned_to_agent, status, depends_on_task_id, created_by_agent,
                  created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                task.project_id,
                task.epic_id,
                task.title,
                task.description,
                task.source_prompt,
                task.assigned_to_agent,
                task.status.as_str(),
                task.depends_on_task_id,
                task.created_by_agent,
                created_at,
                updated_at,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    async fn update_task_status(&self, task_id: i64, status: TaskStatus) -> Result<(), AgentError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE task SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![status.as_str(), now(), task_id],
        )?;
        Ok(())
    }

    async fn get_task(&self, task_id: i64) -> Result<Option<Task>, AgentError> {
        let conn = self.conn.lock().unwrap();
        let task = conn
            .query_row(
                "SELECT id, project_id, epic_id, title, description, source_prompt,
                        assigned_to_agent, status, depends_on_task_id, created_by_agent,
                        created_at, updated_at
                 FROM task WHERE id = ?1 LIMIT 1",
                params![task_id],
                map_task,
            )
            .optional()?;
        Ok(task)
    }

    async fn list_tasks_for_project(&self, project_id: i64) -> Result<Vec<Task>, AgentError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, epic_id, title, description, source_prompt,
                    assigned_to_agent, status, depends_on_task_id, created_by_agent,
                    created_at, updated_at
             FROM task WHERE project_id = ?1 ORDER BY id ASC",
        )?;
        let tasks = stmt
            .query_map(params![project_id], map_task)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(tasks)
    }

    async fn list_tasks_for_agent(
        &self,
        project_id: i64,
        agent_type: &str,
    ) -> Result<Vec<Task>, AgentError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, epic_id, title, description, source_prompt,
                    assigned_to_agent, status, depends_on_task_id, created_by_agent,
                    created_at, updated_at
             FROM task WHERE project_id = ?1 AND assigned_to_agent = ?2 ORDER BY id ASC",
        )?;
        let tasks = stmt
            .query_map(params![project_id, agent_type], map_task)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(tasks)
    }

    async fn list_pending_review_tasks(&self, project_id: i64) -> Result<Vec<Task>, AgentError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, epic_id, title, description, source_prompt,
                    assigned_to_agent, status, depends_on_task_id, created_by_agent,
                    created_at, updated_at
             FROM task WHERE project_id = ?1 AND status = 'review' ORDER BY id ASC",
        )?;
        let tasks = stmt
            .query_map(params![project_id], map_task)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(tasks)
    }

    async fn create_epic(&self, epic: Epic) -> Result<i64, AgentError> {
        let ts = now();
        let created_at = if epic.created_at == 0 { ts } else { epic.created_at };
        let updated_at = if epic.updated_at == 0 { ts } else { epic.updated_at };
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO epic
                 (project_id, title, description, source_prompt, status, created_by_agent,
                  created_by_task_id, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                epic.project_id,
                epic.title,
                epic.description,
                epic.source_prompt,
                epic.status.as_str(),
                epic.created_by_agent,
                epic.created_by_task_id,
                created_at,
                updated_at,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    async fn update_epic_status(&self, epic_id: i64, status: EpicStatus) -> Result<(), AgentError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE epic SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![status.as_str(), now(), epic_id],
        )?;
        Ok(())
    }

    async fn get_epic(&self, epic_id: i64) -> Result<Option<Epic>, AgentError> {
        let conn = self.conn.lock().unwrap();
        let epic = conn
            .query_row(
                "SELECT id, project_id, title, description, source_prompt, status,
                        created_by_agent, created_by_task_id, created_at, updated_at
                 FROM epic WHERE id = ?1 LIMIT 1",
                params![epic_id],
                map_epic,
            )
            .optional()?;
        Ok(epic)
    }

    async fn list_epics(&self, project_id: i64) -> Result<Vec<Epic>, AgentError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, title, description, source_prompt, status,
                    created_by_agent, created_by_task_id, created_at, updated_at
             FROM epic WHERE project_id = ?1 ORDER BY id ASC",
        )?;
        let epics = stmt
            .query_map(params![project_id], map_epic)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(epics)
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
                     WHERE session_id = ?1 AND version = ?2 LIMIT 1",
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
            project_id, session_id, version, row_id
        );
        Ok(row_id)
    }
}
