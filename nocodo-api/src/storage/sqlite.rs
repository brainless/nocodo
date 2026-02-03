use crate::DbConnection;
use async_trait::async_trait;
use nocodo_agents::{
    AgentStorage, Message, MessageRole, Session, SessionStatus, StorageError, ToolCall,
    ToolCallStatus,
};
use rusqlite::{params, OptionalExtension};

pub struct SqliteAgentStorage {
    connection: DbConnection,
}

impl SqliteAgentStorage {
    pub fn new(connection: DbConnection) -> Self {
        Self { connection }
    }
}

#[async_trait]
impl AgentStorage for SqliteAgentStorage {
    async fn create_session(&self, session: Session) -> Result<i64, StorageError> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| StorageError::OperationFailed(format!("Lock error: {}", e)))?;

        let status_str = match session.status {
            SessionStatus::Running => "running",
            SessionStatus::Completed => "completed",
            SessionStatus::Failed => "failed",
            SessionStatus::WaitingForUserInput => "waiting_for_user_input",
        };

        let config_json =
            serde_json::to_string(&session.config).map_err(StorageError::SerializationError)?;

        conn.execute(
            r#"
            INSERT INTO agent_sessions
                (agent_name, provider, model, system_prompt, user_prompt, config, status, started_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
            params![
                session.agent_name,
                session.provider,
                session.model,
                session.system_prompt,
                session.user_prompt,
                config_json,
                status_str,
                session.started_at,
            ],
        )
        .map_err(|e| StorageError::OperationFailed(e.to_string()))?;

        let id = conn.last_insert_rowid();
        Ok(id)
    }

    async fn get_session(&self, session_id: i64) -> Result<Option<Session>, StorageError> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| StorageError::OperationFailed(format!("Lock error: {}", e)))?;

        let mut stmt = conn
            .prepare(
                r#"
                SELECT id, agent_name, provider, model, system_prompt, user_prompt,
                       config, status, started_at, ended_at, result, error
                FROM agent_sessions
                WHERE id = ?1
                "#,
            )
            .map_err(|e| StorageError::OperationFailed(e.to_string()))?;

        let session = stmt
            .query_row(params![session_id], |row| {
                let status_str: String = row.get(7)?;
                let status = match status_str.as_str() {
                    "running" => SessionStatus::Running,
                    "completed" => SessionStatus::Completed,
                    "failed" => SessionStatus::Failed,
                    "waiting_for_user_input" => SessionStatus::WaitingForUserInput,
                    _ => SessionStatus::Running,
                };

                let config_json: String = row.get(6)?;
                let config: serde_json::Value =
                    serde_json::from_str(&config_json).unwrap_or(serde_json::json!({}));

                Ok(Session {
                    id: Some(row.get(0)?),
                    agent_name: row.get(1)?,
                    provider: row.get(2)?,
                    model: row.get(3)?,
                    system_prompt: row.get(4)?,
                    user_prompt: row.get(5)?,
                    config,
                    status,
                    started_at: row.get(8)?,
                    ended_at: row.get(9)?,
                    result: row.get(10)?,
                    error: row.get(11)?,
                })
            })
            .optional()
            .map_err(|e| StorageError::OperationFailed(e.to_string()))?;

        Ok(session)
    }

    async fn update_session(&self, session: Session) -> Result<(), StorageError> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| StorageError::OperationFailed(format!("Lock error: {}", e)))?;

        let id = session
            .id
            .ok_or_else(|| StorageError::NotFound("Session ID required for update".to_string()))?;

        let status_str = match session.status {
            SessionStatus::Running => "running",
            SessionStatus::Completed => "completed",
            SessionStatus::Failed => "failed",
            SessionStatus::WaitingForUserInput => "waiting_for_user_input",
        };

        conn.execute(
            r#"
            UPDATE agent_sessions
            SET status = ?1, ended_at = ?2, result = ?3, error = ?4
            WHERE id = ?5
            "#,
            params![
                status_str,
                session.ended_at,
                session.result,
                session.error,
                id
            ],
        )
        .map_err(|e| StorageError::OperationFailed(e.to_string()))?;

        Ok(())
    }

    async fn create_message(&self, message: Message) -> Result<i64, StorageError> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| StorageError::OperationFailed(format!("Lock error: {}", e)))?;

        let role_str = match message.role {
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::System => "system",
            MessageRole::Tool => "tool",
        };

        conn.execute(
            r#"
            INSERT INTO agent_messages (session_id, role, content, created_at)
            VALUES (?1, ?2, ?3, ?4)
            "#,
            params![message.session_id, role_str, message.content, message.created_at],
        )
        .map_err(|e| StorageError::OperationFailed(e.to_string()))?;

        let id = conn.last_insert_rowid();
        Ok(id)
    }

    async fn get_messages(&self, session_id: i64) -> Result<Vec<Message>, StorageError> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| StorageError::OperationFailed(format!("Lock error: {}", e)))?;

        let mut stmt = conn
            .prepare(
                r#"
                SELECT id, session_id, role, content, created_at
                FROM agent_messages
                WHERE session_id = ?1
                ORDER BY created_at ASC
                "#,
            )
            .map_err(|e| StorageError::OperationFailed(e.to_string()))?;

        let messages = stmt
            .query_map(params![session_id], |row| {
                let role_str: String = row.get(2)?;
                let role = match role_str.as_str() {
                    "user" => MessageRole::User,
                    "assistant" => MessageRole::Assistant,
                    "system" => MessageRole::System,
                    "tool" => MessageRole::Tool,
                    _ => MessageRole::User,
                };

                Ok(Message {
                    id: Some(row.get(0)?),
                    session_id: row.get(1)?,
                    role,
                    content: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })
            .map_err(|e| StorageError::OperationFailed(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| StorageError::OperationFailed(e.to_string()))?;

        Ok(messages)
    }

    async fn create_tool_call(&self, tool_call: ToolCall) -> Result<i64, StorageError> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| StorageError::OperationFailed(format!("Lock error: {}", e)))?;

        let status_str = match tool_call.status {
            ToolCallStatus::Pending => "pending",
            ToolCallStatus::Executing => "executing",
            ToolCallStatus::Completed => "completed",
            ToolCallStatus::Failed => "failed",
        };

        let request_json =
            serde_json::to_string(&tool_call.request).map_err(StorageError::SerializationError)?;

        conn.execute(
            r#"
            INSERT INTO agent_tool_calls
                (session_id, message_id, tool_call_id, tool_name, request, status, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            params![
                tool_call.session_id,
                tool_call.message_id,
                tool_call.tool_call_id,
                tool_call.tool_name,
                request_json,
                status_str,
                tool_call.created_at,
            ],
        )
        .map_err(|e| StorageError::OperationFailed(e.to_string()))?;

        let id = conn.last_insert_rowid();
        Ok(id)
    }

    async fn update_tool_call(&self, tool_call: ToolCall) -> Result<(), StorageError> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| StorageError::OperationFailed(format!("Lock error: {}", e)))?;

        let id = tool_call
            .id
            .ok_or_else(|| {
                StorageError::NotFound("Tool call ID required for update".to_string())
            })?;

        let status_str = match tool_call.status {
            ToolCallStatus::Pending => "pending",
            ToolCallStatus::Executing => "executing",
            ToolCallStatus::Completed => "completed",
            ToolCallStatus::Failed => "failed",
        };

        let response_json = tool_call
            .response
            .map(|r| serde_json::to_string(&r))
            .transpose()
            .map_err(StorageError::SerializationError)?;

        conn.execute(
            r#"
            UPDATE agent_tool_calls
            SET status = ?1, response = ?2, execution_time_ms = ?3,
                completed_at = ?4, error_details = ?5
            WHERE id = ?6
            "#,
            params![
                status_str,
                response_json,
                tool_call.execution_time_ms,
                tool_call.completed_at,
                tool_call.error_details,
                id
            ],
        )
        .map_err(|e| StorageError::OperationFailed(e.to_string()))?;

        Ok(())
    }

    async fn get_tool_calls(&self, session_id: i64) -> Result<Vec<ToolCall>, StorageError> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| StorageError::OperationFailed(format!("Lock error: {}", e)))?;

        let mut stmt = conn
            .prepare(
                r#"
                SELECT id, session_id, message_id, tool_call_id, tool_name, request,
                       response, status, execution_time_ms, created_at, completed_at, error_details
                FROM agent_tool_calls
                WHERE session_id = ?1
                ORDER BY created_at ASC
                "#,
            )
            .map_err(|e| StorageError::OperationFailed(e.to_string()))?;

        let tool_calls = stmt
            .query_map(params![session_id], |row| {
                let status_str: String = row.get(7)?;
                let status = match status_str.as_str() {
                    "pending" => ToolCallStatus::Pending,
                    "executing" => ToolCallStatus::Executing,
                    "completed" => ToolCallStatus::Completed,
                    "failed" => ToolCallStatus::Failed,
                    _ => ToolCallStatus::Pending,
                };

                let request_json: String = row.get(5)?;
                let request = serde_json::from_str(&request_json).unwrap_or(serde_json::json!({}));

                let response_json: Option<String> = row.get(6)?;
                let response = response_json.and_then(|json| serde_json::from_str(&json).ok());

                Ok(ToolCall {
                    id: Some(row.get(0)?),
                    session_id: row.get(1)?,
                    message_id: row.get(2)?,
                    tool_call_id: row.get(3)?,
                    tool_name: row.get(4)?,
                    request,
                    response,
                    status,
                    execution_time_ms: row.get(8)?,
                    created_at: row.get(9)?,
                    completed_at: row.get(10)?,
                    error_details: row.get(11)?,
                })
            })
            .map_err(|e| StorageError::OperationFailed(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| StorageError::OperationFailed(e.to_string()))?;

        Ok(tool_calls)
    }

    async fn get_pending_tool_calls(
        &self,
        session_id: i64,
    ) -> Result<Vec<ToolCall>, StorageError> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| StorageError::OperationFailed(format!("Lock error: {}", e)))?;

        let mut stmt = conn
            .prepare(
                r#"
                SELECT id, session_id, message_id, tool_call_id, tool_name, request,
                       response, status, execution_time_ms, created_at, completed_at, error_details
                FROM agent_tool_calls
                WHERE session_id = ?1 AND status = 'pending'
                ORDER BY created_at ASC
                "#,
            )
            .map_err(|e| StorageError::OperationFailed(e.to_string()))?;

        let tool_calls = stmt
            .query_map(params![session_id], |row| {
                let request_json: String = row.get(5)?;
                let request = serde_json::from_str(&request_json).unwrap_or(serde_json::json!({}));

                let response_json: Option<String> = row.get(6)?;
                let response = response_json.and_then(|json| serde_json::from_str(&json).ok());

                Ok(ToolCall {
                    id: Some(row.get(0)?),
                    session_id: row.get(1)?,
                    message_id: row.get(2)?,
                    tool_call_id: row.get(3)?,
                    tool_name: row.get(4)?,
                    request,
                    response,
                    status: ToolCallStatus::Pending,
                    execution_time_ms: row.get(8)?,
                    created_at: row.get(9)?,
                    completed_at: row.get(10)?,
                    error_details: row.get(11)?,
                })
            })
            .map_err(|e| StorageError::OperationFailed(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| StorageError::OperationFailed(e.to_string()))?;

        Ok(tool_calls)
    }
}
