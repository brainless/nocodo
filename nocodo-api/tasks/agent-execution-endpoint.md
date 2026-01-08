# API Endpoint for Agent Execution with Database Persistence

## Problem
Need an API endpoint to execute agents (starting with SQLite agent) that accepts user prompts and persists conversation history (system prompt, user prompt, model responses) to SQLite database for later retrieval.

## Task
Create `POST /agents/{agent_id}/execute` endpoint that executes agents and stores all conversation data in a local SQLite database.

## Requirements

### 1. Database Setup
- Database location: `<OS user data path>/nocodo/nocodo-api.db`
  - macOS: `~/Library/Application Support/nocodo/nocodo-api.db`
  - Linux: `~/.local/share/nocodo/nocodo-api.db`
  - Windows: `{FOLDERPATH}\nocodo\nocodo-api.db`
- Reuse existing `nocodo-agents::database::Database` struct
- Initialize once in `main.rs` and share via Actix app state

### 2. Dependencies
Add to `nocodo-api/Cargo.toml`:
```toml
nocodo-agents = { path = "../nocodo-agents" }
manager-tools = { path = "../manager-tools" }
rusqlite = { workspace = true }
chrono = { workspace = true }
anyhow = { workspace = true }
home = "0.5"  # For user data path resolution
```

### 3. Agent Execution Endpoint

**Route**: `POST /agents/{agent_id}/execute`

**Request Body**:
```json
{
  "user_prompt": "Show me all tables in the database",
  "db_path": "/absolute/path/to/database.db"  // Required for sqlite agent
}
```

**Response**:
```json
{
  "session_id": 123,
  "agent_name": "sqlite",
  "status": "completed",
  "result": "Tables: users, posts, comments"
}
```

**Implementation Steps**:
1. Extract `agent_id` from path parameter
2. Validate `agent_id` (only "sqlite" supported in MVP)
3. Parse request body for `user_prompt` and `db_path`
4. Instantiate agent using factory pattern (create helper in `helpers/agents.rs`)
5. Call `agent.execute(user_prompt).await`
6. Return session info and result

### 4. Session Retrieval Endpoint

**Route**: `GET /agents/sessions/{session_id}`

**Response**:
```json
{
  "id": 123,
  "agent_name": "sqlite",
  "provider": "anthropic",
  "model": "claude-sonnet-4",
  "system_prompt": "You are a database analysis expert...",
  "user_prompt": "Show me all tables",
  "status": "completed",
  "result": "Tables: users, posts, comments",
  "messages": [
    {
      "role": "user",
      "content": "Show me all tables",
      "created_at": 1704196800
    },
    {
      "role": "assistant",
      "content": "I'll query the database...",
      "created_at": 1704196801
    }
  ],
  "tool_calls": [
    {
      "tool_name": "sqlite3_reader",
      "request": {"query": "SELECT name FROM sqlite_master WHERE type='table'"},
      "response": {"rows": [...]},
      "status": "completed",
      "execution_time_ms": 45
    }
  ]
}
```

## Data Flow
1. API receives user prompt → creates agent → executes
2. Agent automatically stores to DB via existing `nocodo-agents::database::Database`
3. Storage includes: session metadata, all messages (user/assistant/tool), tool calls with timing
4. API returns session_id for later retrieval

## Files to Create/Modify

**New Files**:
- `nocodo-api/src/models.rs` - Request/response structs
- `nocodo-api/src/handlers/agent_execution.rs` - Execution endpoint handler
- `nocodo-api/src/handlers/sessions.rs` - Session retrieval endpoint
- `nocodo-api/tasks/agent-execution-endpoint.md` - This task file

**Modified Files**:
- `nocodo-api/Cargo.toml` - Add dependencies
- `nocodo-api/src/main.rs` - Initialize Database, register routes
- `nocodo-api/src/handlers/mod.rs` - Export new handlers
- `nocodo-api/src/helpers/agents.rs` - Add agent factory function

## Acceptance Criteria
- `POST /agents/sqlite/execute` accepts user prompt and db_path
- Agent execution stores system_prompt, user_prompt, all messages, tool calls to database
- Database file created at OS-specific user data path
- `GET /agents/sessions/{id}` returns complete conversation history
- All existing agents endpoints (`GET /agents`) continue to work
- Generic architecture allows adding more agent types easily

## Out of Scope (Future Work)
- Authentication/authorization
- Tests (will add later)
- `GET /agents/sessions` list endpoint
- Streaming responses
- Agent configuration options
