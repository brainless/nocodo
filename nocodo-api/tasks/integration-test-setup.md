# Integration Test Setup for nocodo-api

## Objective
Add the first integration test to nocodo-api that validates the agent execution API and database persistence.

## Requirements

### 1. Common Test Module
Create a common test utilities module that includes:
- **Test Database Setup**: Create in-memory or temporary SQLite database for each test
- **Test App Setup**: Initialize Actix test application with all routes and dependencies
- **Configuration**: Load test-specific configuration (or use defaults)

**Location**: `nocodo-api/src/test_utils.rs` or `nocodo-api/tests/common/mod.rs`

**Key utilities needed**:
```rust
// Create test database (in-memory)
async fn setup_test_db() -> Arc<Database>

// Create test Actix app with all routes
async fn setup_test_app() -> impl Service

// Create mock LLM client
fn create_mock_llm_client() -> Arc<dyn LlmClient>
```

### 2. Mock for Agent Crate Requests
Create a mock LLM client that simulates responses without making real API calls to Anthropic/OpenAI.

**Pattern reference**: `nocodo-agents/src/factory.rs:158-187`

**Requirements**:
- Mock `nocodo_llm_sdk::client::LlmClient` trait
- Return predefined responses for SQLite agent queries
- Support at least one tool call cycle (e.g., sqlite3_reader tool)

**Example mock behavior**:
1. User prompt: "How many users are in the database?"
2. Mock returns tool call for sqlite3_reader with query: "SELECT COUNT(*) FROM users"
3. Mock receives tool result and returns final answer: "There are 5 users"

### 3. Integration Test Implementation
Add integration test for agent execute API endpoint.

**Location**: `nocodo-api/tests/agent_execution_test.rs`

**Test scenario**:
```rust
#[actix_rt::test]
async fn test_sqlite_agent_execution_and_persistence()
```

**Steps**:
1. Setup test database (in-memory)
2. Create test SQLite database file with sample data (users table with 5 records)
3. Setup test Actix app with mock LLM client
4. Send POST request to `/agents/sqlite/execute` with:
   ```json
   {
     "user_prompt": "How many users are in the database?",
     "config": {
       "Sqlite": {
         "db_path": "/path/to/test.db"
       }
     }
   }
   ```
5. Assert HTTP response:
   - Status: 200 OK
   - Response contains `session_id`
   - Response contains `agent_name: "sqlite"`
   - Response contains `status: "completed"`
   - Response contains valid result

6. **Validate Database Persistence**:
   - Query `agent_sessions` table:
     - Verify session exists with correct session_id
     - Verify agent_name = "sqlite"
     - Verify status = "completed"
     - Verify result is non-empty
   - Query `agent_messages` table:
     - Verify user message exists with content matching user_prompt
     - Verify assistant messages exist
     - Verify messages are ordered by created_at
   - Query `agent_tool_calls` table (if mock includes tool calls):
     - Verify tool call exists for sqlite3_reader
     - Verify tool request and response are stored
     - Verify execution_time_ms is set

## Implementation Notes

### Dependencies to Add
Add to `nocodo-api/Cargo.toml` under `[dev-dependencies]`:
```toml
actix-rt = "2.0"
actix-web = { version = "4.0", features = ["test-utils"] }
tempfile = "3.0"
```

### Test Database Pattern
Follow the pattern from `nocodo-agents/src/sqlite_analysis/tests.rs`:
- Use `tempfile::NamedTempFile` for temporary SQLite databases
- Use in-memory database for nocodo database: `Database::new(&PathBuf::from(":memory:"))?`
- Clean up resources after test

### Actix Test Utilities
Use Actix's built-in test utilities:
```rust
use actix_web::{test, App};

let app = test::init_service(
    App::new()
        .app_data(/* inject test dependencies */)
        .configure(/* configure routes */)
).await;

let req = test::TestRequest::post()
    .uri("/agents/sqlite/execute")
    .set_json(&request_body)
    .to_request();

let resp = test::call_service(&app, req).await;
```

## Success Criteria
- [x] Test database and app setup utilities are implemented
- [x] Mock LLM client is created and integrated
- [x] Integration test passes successfully
- [x] Test validates HTTP response structure
- [~] Test validates all database tables (sessions, messages, tool_calls) - **Partial**: Session validation works, but message/tool call persistence not yet implemented in agents
- [x] Test is self-contained and can run in isolation
- [x] Test cleans up resources properly (using NamedTempFile which auto-cleans)

## Implementation Status

### ✅ Completed
1. **Common Test Module** (`nocodo-api/tests/common/mod.rs`):
   - Test database setup (in-memory SQLite for nocodo database)
   - Test app setup with Actix test utilities
   - Mock LLM client implementation
   - Helper functions for creating SQLite test databases with sample data

2. **Mock LLM Client**:
   - Implements `nocodo_llm_sdk::client::LlmClient` trait
   - Returns predefined responses (currently simple text, no tool calls)
   - Tracks call count for verification

3. **Integration Test** (`nocodo-api/tests/agent_execution_test.rs`):
   - Tests POST `/agents/sqlite/execute` endpoint
   - Validates HTTP 200 response
   - Validates response JSON structure (session_id, agent_name, status, result)
   - Tests GET `/agents/sessions/{id}` endpoint
   - Validates session retrieval and metadata

4. **Dependencies**:
   - Added `actix-rt` and `actix-http` to dev-dependencies

### ⚠️ Limitations/TODO
1. **Message Persistence**: Agents don't currently save individual messages to the `agent_messages` table during execution. The Agent trait would need to be updated to accept a session_id parameter and call `database.save_message()` during execution.

2. **Tool Call Persistence**: Similarly, tool calls are not being persisted to the `agent_tool_calls` table. This requires the same architectural changes as message persistence.

3. **Mock Tool Calls**: The current mock LLM client returns simple text responses without simulating tool calls. Future enhancement could add mock responses that include tool calls (e.g., sqlite3_reader).

### 📝 Notes
- The test currently validates the core agent execution flow and session persistence
- Session metadata (agent_name, status, result, config, etc.) is properly persisted and retrieved
- The test documents with TODOs where message and tool call validation should be added once persistence is implemented

## Out of Scope
- Other integration tests (only implement ONE test as specified)
- End-to-end tests with real LLM API calls
- Testing error scenarios or edge cases
- Testing other agents (codebase-analysis)
- Testing other API endpoints

## References
- Agent execution handler: `nocodo-api/src/handlers/agent_execution.rs`
- Database implementation: `nocodo-agents/src/database/mod.rs`
- Existing test pattern: `nocodo-agents/src/sqlite_analysis/tests.rs`
- Mock LLM pattern: `nocodo-agents/src/factory.rs:158-187`
