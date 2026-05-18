use std::sync::Mutex;

use async_trait::async_trait;
use rusqlite::{params, Connection, OptionalExtension};

use super::{
    AgentStorage, AgentType, ChatMessage, CommentStorage, ContextStorage, Epic, EpicCommentRow,
    EpicStatus, MessageContent, SchemaStorage, Session, Task, TaskCommentRow, TaskStatus,
    TaskStorage, UiFormStorage, UserChatMessageRow, UserChatSessionRow, UserChatStorage, UserRow,
    UserStorage,
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
                let rows = stmt
                    .query_map(params![project_id, at], map_session)?
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
                let rows = stmt
                    .query_map(params![project_id], map_session)?
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
    async fn rename_project(&self, project_id: i64, name: &str) -> Result<(), AgentError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE project SET name = ?1 WHERE id = ?2",
            params![name, project_id],
        )?;
        Ok(())
    }

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
        let created_at = if msg.created_at == 0 {
            now()
        } else {
            msg.created_at
        };
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
            return Err(AgentError::Other(
                "create_turn requires at least one message".into(),
            ));
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
        let created_at = if task.created_at == 0 {
            ts
        } else {
            task.created_at
        };
        let updated_at = if task.updated_at == 0 {
            ts
        } else {
            task.updated_at
        };
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

    async fn list_open_dispatchable_tasks(&self) -> Result<Vec<Task>, AgentError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT t.id, t.project_id, t.epic_id, t.title, t.description, t.source_prompt,
                    t.assigned_to_agent, t.status, t.depends_on_task_id, t.created_by_agent,
                    t.created_at, t.updated_at
             FROM task t
             LEFT JOIN agent_chat_session s
                    ON s.task_id = t.id AND s.agent_type = t.assigned_to_agent
             WHERE t.status = 'ready'
               AND t.assigned_to_agent != 'project_manager'
               AND s.id IS NULL
             ORDER BY t.id ASC",
        )?;
        let tasks = stmt
            .query_map([], map_task)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(tasks)
    }

    async fn create_epic(&self, epic: Epic) -> Result<i64, AgentError> {
        let ts = now();
        let created_at = if epic.created_at == 0 {
            ts
        } else {
            epic.created_at
        };
        let updated_at = if epic.updated_at == 0 {
            ts
        } else {
            epic.updated_at
        };
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
            project_id,
            session_id,
            version,
            row_id
        );
        Ok(row_id)
    }

    async fn get_latest_schema_for_project(
        &self,
        project_id: i64,
    ) -> Result<Option<String>, AgentError> {
        let conn = self.conn.lock().unwrap();
        let result = conn
            .query_row(
                "SELECT schema_json FROM project_schema
                 WHERE project_id = ?1
                 ORDER BY version DESC, id DESC LIMIT 1",
                params![project_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        Ok(result)
    }
}

// ---------------------------------------------------------------------------
// SqliteUiFormStorage
// ---------------------------------------------------------------------------

pub struct SqliteUiFormStorage {
    conn: Mutex<Connection>,
}

impl SqliteUiFormStorage {
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
impl UiFormStorage for SqliteUiFormStorage {
    async fn save_form_layout(
        &self,
        project_id: i64,
        entity_name: &str,
        layout_json: &str,
    ) -> Result<(), AgentError> {
        let ts = now();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO ui_form_layout (project_id, entity_name, layout_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?4)
             ON CONFLICT(project_id, entity_name) DO UPDATE SET
               layout_json = excluded.layout_json,
               updated_at  = excluded.updated_at",
            params![project_id, entity_name, layout_json, ts],
        )?;
        Ok(())
    }

    async fn get_form_layout(
        &self,
        project_id: i64,
        entity_name: &str,
    ) -> Result<Option<String>, AgentError> {
        let conn = self.conn.lock().unwrap();
        let result = conn
            .query_row(
                "SELECT layout_json FROM ui_form_layout
                 WHERE project_id = ?1 AND entity_name = ?2 LIMIT 1",
                params![project_id, entity_name],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        Ok(result)
    }

    async fn list_form_layouts(
        &self,
        project_id: i64,
    ) -> Result<Vec<(String, String)>, AgentError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT entity_name, layout_json FROM ui_form_layout
             WHERE project_id = ?1 ORDER BY entity_name ASC",
        )?;
        let rows = stmt
            .query_map(params![project_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}

// ---------------------------------------------------------------------------
// SqliteContextStorage
// ---------------------------------------------------------------------------

pub struct SqliteContextStorage {
    conn: Mutex<Connection>,
}

impl SqliteContextStorage {
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
impl ContextStorage for SqliteContextStorage {
    async fn save_context(
        &self,
        project_id: i64,
        context_type: &str,
        context: &str,
    ) -> Result<(), AgentError> {
        let ts = now();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO project_context (project_id, context_type, context, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?4)
             ON CONFLICT(project_id, context_type) DO UPDATE SET
               context    = excluded.context,
               updated_at = excluded.updated_at",
            params![project_id, context_type, context, ts],
        )?;
        Ok(())
    }

    async fn get_context(
        &self,
        project_id: i64,
        context_type: &str,
    ) -> Result<Option<String>, AgentError> {
        let conn = self.conn.lock().unwrap();
        let result = conn
            .query_row(
                "SELECT context FROM project_context
                 WHERE project_id = ?1 AND context_type = ?2 LIMIT 1",
                params![project_id, context_type],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        Ok(result)
    }
}

// ---------------------------------------------------------------------------
// SqliteUserStorage
// ---------------------------------------------------------------------------

pub struct SqliteUserStorage {
    conn: Mutex<Connection>,
}

impl SqliteUserStorage {
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

fn map_user_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<UserRow> {
    Ok(UserRow {
        id: row.get(0)?,
        display_name: row.get(1)?,
        email: row.get(2)?,
        is_guest: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

#[async_trait]
impl UserStorage for SqliteUserStorage {
    async fn create_guest_user(&self, display_name: String) -> Result<i64, AgentError> {
        let ts = now();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO user (display_name, is_guest, created_at, updated_at)
             VALUES (?1, 1, ?2, ?3)",
            params![display_name, ts, ts],
        )?;
        Ok(conn.last_insert_rowid())
    }

    async fn get_user(&self, user_id: i64) -> Result<Option<UserRow>, AgentError> {
        let conn = self.conn.lock().unwrap();
        let user = conn
            .query_row(
                "SELECT id, display_name, email, is_guest, created_at, updated_at
                 FROM user WHERE id = ?1 LIMIT 1",
                params![user_id],
                map_user_row,
            )
            .optional()?;
        Ok(user)
    }

    async fn update_display_name(
        &self,
        user_id: i64,
        display_name: String,
    ) -> Result<(), AgentError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE user SET display_name = ?1, updated_at = ?2 WHERE id = ?3",
            params![display_name, now(), user_id],
        )?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// SqliteUserChatStorage
// ---------------------------------------------------------------------------

pub struct SqliteUserChatStorage {
    conn: Mutex<Connection>,
}

impl SqliteUserChatStorage {
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

fn map_user_chat_session_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<UserChatSessionRow> {
    Ok(UserChatSessionRow {
        id: row.get(0)?,
        project_id: row.get(1)?,
        created_by_user_id: row.get(2)?,
        status: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
        completed_at: row.get(6)?,
    })
}

fn map_user_chat_message_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<UserChatMessageRow> {
    Ok(UserChatMessageRow {
        id: row.get(0)?,
        session_id: row.get(1)?,
        author_type: row.get(2)?,
        author_user_id: row.get(3)?,
        agent_type: row.get(4)?,
        turn_id: row.get(5)?,
        content_type: row.get(6)?,
        content: row.get(7)?,
        created_at: row.get(8)?,
    })
}

#[async_trait]
impl UserChatStorage for SqliteUserChatStorage {
    async fn create_session(&self, project_id: i64, user_id: i64) -> Result<i64, AgentError> {
        let ts = now();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO user_chat_session (project_id, created_by_user_id, status, created_at, updated_at)
             VALUES (?1, ?2, 'open', ?3, ?4)",
            params![project_id, user_id, ts, ts],
        )?;
        Ok(conn.last_insert_rowid())
    }

    async fn get_session(&self, session_id: i64) -> Result<Option<UserChatSessionRow>, AgentError> {
        let conn = self.conn.lock().unwrap();
        let session = conn
            .query_row(
                "SELECT id, project_id, created_by_user_id, status, created_at, updated_at, completed_at
                 FROM user_chat_session WHERE id = ?1 LIMIT 1",
                params![session_id],
                map_user_chat_session_row,
            )
            .optional()?;
        Ok(session)
    }

    async fn append_message(
        &self,
        session_id: i64,
        author_type: &str,
        author_user_id: Option<i64>,
        agent_type: Option<AgentType>,
        turn_id: Option<i64>,
        content: MessageContent,
    ) -> Result<i64, AgentError> {
        let ts = now();
        let agent_type_str = agent_type.map(|a| a.as_str().to_string());
        let content_type = content.content_type_str();
        let content_str = content.to_storage_content();
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO user_chat_message
                 (session_id, author_type, author_user_id, agent_type, turn_id, content_type, content, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![session_id, author_type, author_user_id, agent_type_str, turn_id, content_type, content_str, ts],
        )?;
        Ok(conn.last_insert_rowid())
    }

    async fn get_messages(&self, session_id: i64) -> Result<Vec<UserChatMessageRow>, AgentError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, author_type, author_user_id, agent_type, turn_id, content_type, content, created_at
             FROM user_chat_message
             WHERE session_id = ?1
             ORDER BY id ASC",
        )?;
        let messages = stmt
            .query_map(params![session_id], map_user_chat_message_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(messages)
    }

    async fn complete_session(&self, session_id: i64) -> Result<(), AgentError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE user_chat_session SET status = 'completed', completed_at = ?1, updated_at = ?1 WHERE id = ?2",
            params![now(), session_id],
        )?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// SqliteCommentStorage
// ---------------------------------------------------------------------------

pub struct SqliteCommentStorage {
    conn: Mutex<Connection>,
}

impl SqliteCommentStorage {
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

fn map_epic_comment_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<EpicCommentRow> {
    Ok(EpicCommentRow {
        id: row.get(0)?,
        epic_id: row.get(1)?,
        author_type: row.get(2)?,
        author_user_id: row.get(3)?,
        agent_type: row.get(4)?,
        content: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

fn map_task_comment_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TaskCommentRow> {
    Ok(TaskCommentRow {
        id: row.get(0)?,
        task_id: row.get(1)?,
        author_type: row.get(2)?,
        author_user_id: row.get(3)?,
        agent_type: row.get(4)?,
        content: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

#[async_trait]
impl CommentStorage for SqliteCommentStorage {
    async fn add_epic_comment(
        &self,
        epic_id: i64,
        author_type: &str,
        author_user_id: Option<i64>,
        agent_type: Option<AgentType>,
        content: String,
    ) -> Result<i64, AgentError> {
        let ts = now();
        let agent_type_str = agent_type.map(|a| a.as_str().to_string());
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO epic_comment (epic_id, author_type, author_user_id, agent_type, content, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![epic_id, author_type, author_user_id, agent_type_str, content, ts, ts],
        )?;
        Ok(conn.last_insert_rowid())
    }

    async fn get_epic_comments(&self, epic_id: i64) -> Result<Vec<EpicCommentRow>, AgentError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, epic_id, author_type, author_user_id, agent_type, content, created_at, updated_at
             FROM epic_comment
             WHERE epic_id = ?1
             ORDER BY id ASC",
        )?;
        let comments = stmt
            .query_map(params![epic_id], map_epic_comment_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(comments)
    }

    async fn add_task_comment(
        &self,
        task_id: i64,
        author_type: &str,
        author_user_id: Option<i64>,
        agent_type: Option<AgentType>,
        content: String,
    ) -> Result<i64, AgentError> {
        let ts = now();
        let agent_type_str = agent_type.map(|a| a.as_str().to_string());
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO task_comment (task_id, author_type, author_user_id, agent_type, content, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![task_id, author_type, author_user_id, agent_type_str, content, ts, ts],
        )?;
        Ok(conn.last_insert_rowid())
    }

    async fn get_task_comments(&self, task_id: i64) -> Result<Vec<TaskCommentRow>, AgentError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, task_id, author_type, author_user_id, agent_type, content, created_at, updated_at
             FROM task_comment
             WHERE task_id = ?1
             ORDER BY id ASC",
        )?;
        let comments = stmt
            .query_map(params![task_id], map_task_comment_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(comments)
    }
}
