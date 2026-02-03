# Implement SQLite Agent Storage for nocodo-api

**Status**: 📋 Not Started
**Priority**: High
**Created**: 2026-02-03

## Summary

Implement SQLite-based storage for nocodo-agents by creating a `SqliteAgentStorage` struct that implements the `AgentStorage` trait. This allows nocodo-api to continue using SQLite for storing agent sessions, messages, and tool calls.

## Problem Statement

After refactoring nocodo-agents to use trait-based storage, nocodo-api needs a concrete SQLite implementation to store agent execution data. The storage must:
- Implement the `AgentStorage` trait from nocodo-agents
- Use SQLite for persistence
- Manage database migrations
- Support the existing nocodo-api database infrastructure

## Goals

1. **Implement AgentStorage trait**: Create `SqliteAgentStorage` struct with SQLite backend
2. **Database migrations**: Port migrations from nocodo-agents to nocodo-api
3. **Connection management**: Use Arc-wrapped connection for thread safety
4. **Integration**: Update nocodo-api to use new storage implementation
5. **Maintain compatibility**: Keep existing nocodo-api functionality working

## Architecture

### Storage Implementation Structure

```rust
// nocodo-api/src/storage/sqlite.rs

use nocodo_agents::{AgentStorage, Session, Message, ToolCall, StorageError};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use async_trait::async_trait;

pub type DbConnection = Arc<Mutex<Connection>>;

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
    async fn create_session(&self, session: Session) -> Result<String, StorageError> {
        // SQLite implementation
    }
    // ... other methods
}
```

## Implementation Plan

### Phase 1: Create Storage Module Structure

#### 1.1 Create Module Files

Create new directory and files:
```
nocodo-api/src/
  storage/
    mod.rs          # Module exports
    sqlite.rs       # SqliteAgentStorage implementation
    migrations/     # SQLite migrations
```

### Phase 2: Port Migrations

#### 2.1 Copy Migration Files

Copy migration files from nocodo-agents to nocodo-api:

**Source**: `nocodo-agents/src/database/migrations/`
**Destination**: `nocodo-api/src/storage/migrations/`

Migration files to copy:
- `V1__create_agent_sessions.rs`
- `V2__create_agent_messages.rs`
- `V3__create_agent_tool_calls.rs`
- `V4__create_project_requirements_qna.rs`
- `V5__create_project_settings.rs`

#### 2.2 Update Migration Module

**File**: `nocodo-api/src/storage/migrations/mod.rs`

```rust
use refinery::embed_migrations;

embed_migrations!("src/storage/migrations");

pub fn run_migrations(connection: &mut rusqlite::Connection) -> Result<(), refinery::Error> {
    migrations::runner().run(connection)
}
```

### Phase 3: Implement SqliteAgentStorage

#### 3.1 Implement Core Storage Methods

**File**: `nocodo-api/src/storage/sqlite.rs`

```rust
use async_trait::async_trait;
use chrono::Utc;
use nocodo_agents::{
    AgentStorage, Message, MessageRole, Session, SessionStatus, StorageError, ToolCall,
    ToolCallStatus,
};
use rusqlite::{params, Connection};
use std::sync::{Arc, Mutex};

pub type DbConnection = Arc<Mutex<Connection>>;

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
    async fn create_session(&self, mut session: Session) -> Result<String, StorageError> {
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

        let config_json = serde_json::to_string(&session.config)
            .map_err(StorageError::SerializationError)?;

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

        let id = conn.last_insert_rowid().to_string();
        Ok(id)
    }

    async fn get_session(&self, session_id: &str) -> Result<Option<Session>, StorageError> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| StorageError::OperationFailed(format!("Lock error: {}", e)))?;

        let id: i64 = session_id
            .parse()
            .map_err(|_| StorageError::NotFound(format!("Invalid session ID: {}", session_id)))?;

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
            .query_row(params![id], |row| {
                let status_str: String = row.get(7)?;
                let status = match status_str.as_str() {
                    "running" => SessionStatus::Running,
                    "completed" => SessionStatus::Completed,
                    "failed" => SessionStatus::Failed,
                    "waiting_for_user_input" => SessionStatus::WaitingForUserInput,
                    _ => SessionStatus::Running,
                };

                let config_json: String = row.get(6)?;
                let config: serde_json::Value = serde_json::from_str(&config_json)
                    .unwrap_or(serde_json::json!({}));

                Ok(Session {
                    id: Some(row.get::<_, i64>(0)?.to_string()),
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

        let id: i64 = session
            .id
            .as_ref()
            .and_then(|id| id.parse().ok())
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
            params![status_str, session.ended_at, session.result, session.error, id],
        )
        .map_err(|e| StorageError::OperationFailed(e.to_string()))?;

        Ok(())
    }

    async fn create_message(&self, mut message: Message) -> Result<String, StorageError> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| StorageError::OperationFailed(format!("Lock error: {}", e)))?;

        let session_id: i64 = message
            .session_id
            .parse()
            .map_err(|_| StorageError::OperationFailed("Invalid session ID".to_string()))?;

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
            params![session_id, role_str, message.content, message.created_at],
        )
        .map_err(|e| StorageError::OperationFailed(e.to_string()))?;

        let id = conn.last_insert_rowid().to_string();
        Ok(id)
    }

    async fn get_messages(&self, session_id: &str) -> Result<Vec<Message>, StorageError> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| StorageError::OperationFailed(format!("Lock error: {}", e)))?;

        let id: i64 = session_id
            .parse()
            .map_err(|_| StorageError::NotFound(format!("Invalid session ID: {}", session_id)))?;

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
            .query_map(params![id], |row| {
                let role_str: String = row.get(2)?;
                let role = match role_str.as_str() {
                    "user" => MessageRole::User,
                    "assistant" => MessageRole::Assistant,
                    "system" => MessageRole::System,
                    "tool" => MessageRole::Tool,
                    _ => MessageRole::User,
                };

                Ok(Message {
                    id: Some(row.get::<_, i64>(0)?.to_string()),
                    session_id: row.get::<_, i64>(1)?.to_string(),
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

    async fn create_tool_call(&self, tool_call: ToolCall) -> Result<String, StorageError> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| StorageError::OperationFailed(format!("Lock error: {}", e)))?;

        let session_id: i64 = tool_call
            .session_id
            .parse()
            .map_err(|_| StorageError::OperationFailed("Invalid session ID".to_string()))?;

        let message_id: Option<i64> = tool_call
            .message_id
            .as_ref()
            .and_then(|id| id.parse().ok());

        let status_str = match tool_call.status {
            ToolCallStatus::Pending => "pending",
            ToolCallStatus::Executing => "executing",
            ToolCallStatus::Completed => "completed",
            ToolCallStatus::Failed => "failed",
        };

        let request_json = serde_json::to_string(&tool_call.request)
            .map_err(StorageError::SerializationError)?;

        conn.execute(
            r#"
            INSERT INTO agent_tool_calls
                (session_id, message_id, tool_call_id, tool_name, request, status, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            params![
                session_id,
                message_id,
                tool_call.tool_call_id,
                tool_call.tool_name,
                request_json,
                status_str,
                tool_call.created_at,
            ],
        )
        .map_err(|e| StorageError::OperationFailed(e.to_string()))?;

        let id = conn.last_insert_rowid().to_string();
        Ok(id)
    }

    async fn update_tool_call(&self, tool_call: ToolCall) -> Result<(), StorageError> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| StorageError::OperationFailed(format!("Lock error: {}", e)))?;

        let id: i64 = tool_call
            .id
            .as_ref()
            .and_then(|id| id.parse().ok())
            .ok_or_else(|| StorageError::NotFound("Tool call ID required for update".to_string()))?;

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

    async fn get_tool_calls(&self, session_id: &str) -> Result<Vec<ToolCall>, StorageError> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| StorageError::OperationFailed(format!("Lock error: {}", e)))?;

        let id: i64 = session_id
            .parse()
            .map_err(|_| StorageError::NotFound(format!("Invalid session ID: {}", session_id)))?;

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
            .query_map(params![id], |row| {
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
                let response = response_json
                    .and_then(|json| serde_json::from_str(&json).ok());

                Ok(ToolCall {
                    id: Some(row.get::<_, i64>(0)?.to_string()),
                    session_id: row.get::<_, i64>(1)?.to_string(),
                    message_id: row.get::<_, Option<i64>>(2)?.map(|id| id.to_string()),
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

    async fn get_pending_tool_calls(&self, session_id: &str) -> Result<Vec<ToolCall>, StorageError> {
        let conn = self
            .connection
            .lock()
            .map_err(|e| StorageError::OperationFailed(format!("Lock error: {}", e)))?;

        let id: i64 = session_id
            .parse()
            .map_err(|_| StorageError::NotFound(format!("Invalid session ID: {}", session_id)))?;

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
            .query_map(params![id], |row| {
                let request_json: String = row.get(5)?;
                let request = serde_json::from_str(&request_json).unwrap_or(serde_json::json!({}));

                let response_json: Option<String> = row.get(6)?;
                let response = response_json
                    .and_then(|json| serde_json::from_str(&json).ok());

                Ok(ToolCall {
                    id: Some(row.get::<_, i64>(0)?.to_string()),
                    session_id: row.get::<_, i64>(1)?.to_string(),
                    message_id: row.get::<_, Option<i64>>(2)?.map(|id| id.to_string()),
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
```

### Phase 4: Update nocodo-api Integration

#### 4.1 Update Database Helper

**File**: `nocodo-api/src/helpers/database.rs`

Add migration runner:
```rust
use crate::storage::migrations;

pub fn initialize_database(db_path: &str) -> anyhow::Result<DbConnection> {
    // Create parent directories if needed
    if let Some(parent) = Path::new(db_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut conn = Connection::open(db_path)?;

    // Enable foreign keys
    conn.execute("PRAGMA foreign_keys = ON", [])?;

    // Run migrations
    migrations::run_migrations(&mut conn)?;

    Ok(Arc::new(Mutex::new(conn)))
}
```

#### 4.2 Update main.rs

**File**: `nocodo-api/src/main.rs`

Update to use new storage:
```rust
use nocodo_api::storage::SqliteAgentStorage;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // ... config loading

    // Initialize database
    let db_connection = helpers::database::initialize_database(&config.database.path)?;

    // Create storage
    let storage = Arc::new(SqliteAgentStorage::new(db_connection.clone()));

    // Create app state with storage
    let app_state = web::Data::new(AppState {
        storage,
        // ... other fields
    });

    // ... rest of main
}
```

#### 4.3 Update Cargo.toml

**File**: `nocodo-api/Cargo.toml`

Ensure dependencies:
```toml
[dependencies]
nocodo-agents = { path = "../nocodo-agents" }
nocodo-tools = { path = "../nocodo-tools" }
rusqlite = { version = "0.37", features = ["bundled"] }
refinery = { version = "0.9", features = ["rusqlite"] }
chrono = { version = "0.4", features = ["serde"] }
async-trait = "0.1"
```

### Phase 5: Update Handlers

#### 5.1 Update Agent Execution Handler

**File**: `nocodo-api/src/handlers/agent_execution.rs`

Update to use storage from app state:
```rust
pub async fn execute_agent(
    agent_id: web::Path<String>,
    request: web::Json<ExecuteAgentRequest>,
    data: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    let agent = match agent_id.as_str() {
        "sqlite" => {
            let agent = SqliteReaderAgent::new(
                data.llm_client.clone(),
                data.storage.clone(),  // Use storage from app state
                data.tool_executor.clone(),
                request.db_path.clone(),
            )?;
            Box::new(agent) as Box<dyn Agent<SqliteAgentStorage>>
        }
        // ... other agents
    };

    let result = agent.execute(&request.user_prompt).await?;
    Ok(HttpResponse::Ok().json(result))
}
```

## Files Changed

### New Files
- `nocodo-api/src/storage/mod.rs`
- `nocodo-api/src/storage/sqlite.rs`
- `nocodo-api/src/storage/migrations/mod.rs`
- `nocodo-api/src/storage/migrations/V1__create_agent_sessions.rs`
- `nocodo-api/src/storage/migrations/V2__create_agent_messages.rs`
- `nocodo-api/src/storage/migrations/V3__create_agent_tool_calls.rs`
- `nocodo-api/src/storage/migrations/V4__create_project_requirements_qna.rs`
- `nocodo-api/src/storage/migrations/V5__create_project_settings.rs`
- `nocodo-api/tasks/implement-sqlite-agent-storage.md`

### Modified Files
- `nocodo-api/Cargo.toml` - Add dependencies
- `nocodo-api/src/lib.rs` - Export storage module
- `nocodo-api/src/main.rs` - Initialize storage
- `nocodo-api/src/helpers/database.rs` - Add migration runner
- `nocodo-api/src/handlers/agent_execution.rs` - Use new storage

## Testing Strategy

### Compilation
```bash
cd nocodo-api
cargo check
```

### Manual Testing
```bash
cargo run

# In another terminal
curl -X POST http://localhost:8080/agents/sqlite/execute \
  -H "Content-Type: application/json" \
  -d '{
    "user_prompt": "List all tables",
    "db_path": "/path/to/test.db"
  }'
```

## Success Criteria

- [ ] `SqliteAgentStorage` struct implements `AgentStorage` trait
- [ ] All trait methods implemented with SQLite backend
- [ ] Migrations ported and working
- [ ] Database initialized at startup
- [ ] Agent execution uses new storage
- [ ] Existing API endpoints continue working
- [ ] Code compiles without errors
- [ ] No clippy warnings
- [ ] Manual testing successful

## Notes

- This implementation maintains backward compatibility with existing nocodo-api behavior
- SQLite remains the storage backend for nocodo-api
- The storage layer is now isolated and can be tested independently
- Future enhancements could include connection pooling if needed
