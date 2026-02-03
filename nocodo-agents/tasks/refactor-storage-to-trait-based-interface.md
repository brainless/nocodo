# Refactor Storage to Trait-Based Interface

**Status**: 🔄 In Progress (~60% complete)
**Priority**: High
**Created**: 2026-02-03

## Summary

Refactor nocodo-agents to use trait-based storage abstraction, removing direct SQLite implementation. This allows consuming applications to provide their own storage implementations (PostgreSQL, files, memory, etc.) while keeping nocodo-agents database-agnostic.

## Problem Statement

Currently, nocodo-agents is tightly coupled to SQLite through:
- Direct `rusqlite` usage in `database/mod.rs`
- SQLite-specific migrations via `refinery`
- `Arc<Mutex<Connection>>` pattern throughout the codebase

This prevents:
- Using nocodo-agents with different databases (PostgreSQL, MySQL, etc.)
- Using alternative storage backends (files, memory, cloud storage)
- Integrating nocodo into projects with existing database infrastructure

## Goals

1. **Define storage trait interface**: Create `AgentStorage` trait with all storage operations
2. **Define data structures**: Create shared types for Session, Message, ToolCall
3. **Remove SQLite implementation**: Move concrete SQLite code out of nocodo-agents
4. **Update agents**: Refactor all agents to use trait instead of concrete Database struct
5. **Keep migrations separate**: Remove migration code from nocodo-agents core

## Architecture

### Storage Trait Interface

```rust
// nocodo-agents/src/storage/mod.rs

use async_trait::async_trait;
use crate::types::{Session, Message, ToolCall};

#[async_trait]
pub trait AgentStorage: Send + Sync {
    // Session management
    async fn create_session(&self, session: Session) -> Result<String, StorageError>;
    async fn get_session(&self, session_id: &str) -> Result<Option<Session>, StorageError>;
    async fn update_session(&self, session: Session) -> Result<(), StorageError>;

    // Message management
    async fn create_message(&self, message: Message) -> Result<String, StorageError>;
    async fn get_messages(&self, session_id: &str) -> Result<Vec<Message>, StorageError>;

    // Tool call management
    async fn create_tool_call(&self, tool_call: ToolCall) -> Result<String, StorageError>;
    async fn update_tool_call(&self, tool_call: ToolCall) -> Result<(), StorageError>;
    async fn get_tool_calls(&self, session_id: &str) -> Result<Vec<ToolCall>, StorageError>;
    async fn get_pending_tool_calls(&self, session_id: &str) -> Result<Vec<ToolCall>, StorageError>;
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Storage operation failed: {0}")]
    OperationFailed(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Other error: {0}")]
    Other(String),
}
```

### Data Structure Types

```rust
// nocodo-agents/src/types/session.rs

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: Option<String>,
    pub agent_name: String,
    pub provider: String,
    pub model: String,
    pub system_prompt: Option<String>,
    pub user_prompt: String,
    pub config: serde_json::Value,
    pub status: SessionStatus,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub result: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionStatus {
    Running,
    Completed,
    Failed,
    WaitingForUserInput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Option<String>,
    pub session_id: String,
    pub role: MessageRole,
    pub content: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: Option<String>,
    pub session_id: String,
    pub message_id: Option<String>,
    pub tool_call_id: String,
    pub tool_name: String,
    pub request: serde_json::Value,
    pub response: Option<serde_json::Value>,
    pub status: ToolCallStatus,
    pub execution_time_ms: Option<i64>,
    pub created_at: i64,
    pub completed_at: Option<i64>,
    pub error_details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ToolCallStatus {
    Pending,
    Executing,
    Completed,
    Failed,
}
```

## Implementation Plan

### Phase 1: Create Storage Types Module

#### 1.1 Create Types Module Structure

Create new modules:
```
nocodo-agents/src/
  types/
    mod.rs          # Module exports
    session.rs      # Session, SessionStatus
    message.rs      # Message, MessageRole
    tool_call.rs    # ToolCall, ToolCallStatus
```

#### 1.2 Implement Type Definitions

**File**: `nocodo-agents/src/types/session.rs`

- Define `Session` struct with all fields from current database schema
- Define `SessionStatus` enum
- Derive Serialize, Deserialize, Debug, Clone

**File**: `nocodo-agents/src/types/message.rs`

- Define `Message` struct
- Define `MessageRole` enum
- Derive Serialize, Deserialize, Debug, Clone

**File**: `nocodo-agents/src/types/tool_call.rs`

- Define `ToolCall` struct
- Define `ToolCallStatus` enum
- Derive Serialize, Deserialize, Debug, Clone

**File**: `nocodo-agents/src/types/mod.rs`

```rust
mod session;
mod message;
mod tool_call;

pub use session::{Session, SessionStatus};
pub use message::{Message, MessageRole};
pub use tool_call::{ToolCall, ToolCallStatus};
```

### Phase 2: Create Storage Trait Interface

#### 2.1 Create Storage Module

**File**: `nocodo-agents/src/storage/mod.rs`

- Define `AgentStorage` trait with async methods
- Define `StorageError` enum with thiserror
- Add comprehensive documentation for each method
- Include usage examples in doc comments

### Phase 3: Refactor Agents to Use Trait

#### 3.1 Update Agent Structures

**Files to modify**:
- `nocodo-agents/src/codebase_analysis/mod.rs`
- `nocodo-agents/src/sqlite_reader/mod.rs`
- `nocodo-agents/src/requirements_gathering/mod.rs`
- `nocodo-agents/src/settings_management/mod.rs`
- `nocodo-agents/src/imap_email/mod.rs`
- `nocodo-agents/src/structured_json/mod.rs`

**Changes**:

Replace:
```rust
pub struct SomeAgent {
    database: Arc<Database>,
    // ...
}
```

With:
```rust
pub struct SomeAgent<S: AgentStorage> {
    storage: Arc<S>,
    // ...
}
```

#### 3.2 Update Agent Methods

Replace all database method calls:

**Before**:
```rust
let session_id = self.database.create_session(
    "agent-name",
    provider,
    model,
    system_prompt,
    user_prompt,
)?;
```

**After**:
```rust
let session = Session {
    id: None,
    agent_name: "agent-name".to_string(),
    provider: provider.to_string(),
    model: model.to_string(),
    system_prompt: Some(system_prompt.to_string()),
    user_prompt: user_prompt.to_string(),
    config: serde_json::json!({}),
    status: SessionStatus::Running,
    started_at: chrono::Utc::now().timestamp(),
    ended_at: None,
    result: None,
    error: None,
};

let session_id = self.storage.create_session(session).await?;
```

**Replace all occurrences**:
- `database.create_message()` → `storage.create_message()`
- `database.get_messages()` → `storage.get_messages()`
- `database.create_tool_call()` → `storage.create_tool_call()`
- `database.complete_tool_call()` → `storage.update_tool_call()`
- `database.fail_tool_call()` → `storage.update_tool_call()`
- `database.complete_session()` → `storage.update_session()`
- `database.fail_session()` → `storage.update_session()`

#### 3.3 Update Agent Factory

**File**: `nocodo-agents/src/factory.rs`

Change factory methods to accept storage:

**Before**:
```rust
pub fn create_sqlite_reader_agent(
    &self,
    db_path: String,
) -> anyhow::Result<SqliteReaderAgent> {
    SqliteReaderAgent::new(
        self.llm_client.clone(),
        self.database.clone(),
        self.tool_executor.clone(),
        db_path,
    )
}
```

**After**:
```rust
pub fn create_sqlite_reader_agent<S: AgentStorage + 'static>(
    &self,
    storage: Arc<S>,
    db_path: String,
) -> anyhow::Result<SqliteReaderAgent<S>> {
    SqliteReaderAgent::new(
        self.llm_client.clone(),
        storage,
        self.tool_executor.clone(),
        db_path,
    )
}
```

### Phase 4: Remove SQLite Implementation

#### 4.1 Remove Database Module

Delete or move these files:
- `nocodo-agents/src/database/mod.rs`
- `nocodo-agents/src/database/migrations/`

These will be moved to consuming applications (like nocodo-api).

#### 4.2 Update Cargo.toml

**File**: `nocodo-agents/Cargo.toml`

Remove SQLite-specific dependencies:
```toml
# Remove these:
# rusqlite = { version = "0.37", features = ["bundled"] }
# refinery = { version = "0.9", features = ["rusqlite"] }

# Keep these:
chrono = { version = "0.4", features = ["serde"] }
async-trait = "0.1"
thiserror = { workspace = true }
```

#### 4.3 Update lib.rs

**File**: `nocodo-agents/src/lib.rs`

Remove:
```rust
pub mod database;
```

Add:
```rust
pub mod storage;
pub mod types;
```

Update exports:
```rust
pub use storage::{AgentStorage, StorageError};
pub use types::{Session, SessionStatus, Message, MessageRole, ToolCall, ToolCallStatus};
```

### Phase 5: Update Binary Runners

Update all binary runners to accept storage implementation:

**Files to update**:
- `nocodo-agents/bin/codebase_analysis_runner.rs`
- `nocodo-agents/bin/sqlite_reader_runner.rs`
- `nocodo-agents/bin/structured_json_runner.rs`
- `nocodo-agents/bin/requirements_gathering_runner.rs`
- `nocodo-agents/bin/settings_management_runner.rs`
- `nocodo-agents/bin/imap_email_runner.rs`

**Note**: Since these are standalone binaries, they will need their own storage implementation. Consider adding a temporary in-memory implementation for testing or updating them to use nocodo-api's SQLite implementation.

## Files Changed

### New Files
- `nocodo-agents/src/types/mod.rs`
- `nocodo-agents/src/types/session.rs`
- `nocodo-agents/src/types/message.rs`
- `nocodo-agents/src/types/tool_call.rs`
- `nocodo-agents/src/storage/mod.rs`
- `nocodo-agents/tasks/refactor-storage-to-trait-based-interface.md`

### Deleted Files
- `nocodo-agents/src/database/mod.rs`
- `nocodo-agents/src/database/migrations/` (entire directory)

### Modified Files
- `nocodo-agents/Cargo.toml` - Remove rusqlite, refinery
- `nocodo-agents/src/lib.rs` - Export new modules, remove database
- `nocodo-agents/src/factory.rs` - Update all factory methods
- `nocodo-agents/src/codebase_analysis/mod.rs` - Use trait
- `nocodo-agents/src/sqlite_reader/mod.rs` - Use trait
- `nocodo-agents/src/requirements_gathering/mod.rs` - Use trait
- `nocodo-agents/src/settings_management/mod.rs` - Use trait
- `nocodo-agents/src/imap_email/mod.rs` - Use trait
- `nocodo-agents/src/structured_json/mod.rs` - Use trait
- All binary runners in `nocodo-agents/bin/`

## Testing Strategy

### Compilation Check
```bash
cd nocodo-agents
cargo check
```

### Type Checking
Ensure all agents compile with generic storage parameter:
```bash
cargo build --lib
```

### Documentation
Generate and review trait documentation:
```bash
cargo doc --open
```

## Success Criteria

- [ ] `AgentStorage` trait defined with all required methods
- [ ] `Session`, `Message`, `ToolCall` types defined and exported
- [ ] `StorageError` enum defined with proper error handling
- [ ] All agents refactored to use `AgentStorage` trait
- [ ] All database method calls replaced with trait calls
- [ ] SQLite-specific code removed (rusqlite, refinery dependencies)
- [ ] `database` module removed from nocodo-agents
- [ ] Factory methods updated to accept storage parameter
- [ ] Code compiles without errors
- [ ] No clippy warnings
- [ ] Documentation complete for trait interface

## Migration Guide for Consuming Applications

After this refactor, consuming applications must:

1. **Implement the trait**:
```rust
use nocodo_agents::{AgentStorage, Session, Message, ToolCall, StorageError};
use async_trait::async_trait;

struct MyStorage {
    // Your storage implementation
}

#[async_trait]
impl AgentStorage for MyStorage {
    async fn create_session(&self, session: Session) -> Result<String, StorageError> {
        // Your implementation
    }
    // ... implement all other methods
}
```

2. **Pass storage to agents**:
```rust
let storage = Arc::new(MyStorage::new());
let agent = SqliteReaderAgent::new(
    llm_client,
    storage,
    tool_executor,
    db_path,
)?;
```

## Notes

- This is a breaking change for all consuming applications
- Agents become generic over storage type: `Agent<S: AgentStorage>`
- Storage implementations are fully owned by consuming applications
- nocodo-agents no longer has opinions about storage backend
- Migration path provided for existing nocodo-api consumer
