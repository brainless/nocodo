# Integration Test for Chat History Endpoint

## Objective
Add an integration test that validates the GET `/agents/sessions/{session_id}` endpoint by inserting mock chat history data directly into the test database (without using the agent execution API).

## Context
The current integration test (`agent_execution_test.rs`) creates data through the agent execution API endpoint. This new test will:
- Insert mock data directly into the database tables
- Simulate a complete SQLite agent conversation with messages and tool calls
- Validate that the chat history endpoint correctly retrieves and formats the data

## Requirements

### 1. Create Test File
**Location**: `nocodo-api/tests/chat_history_test.rs`

### 2. Mock Data Structure
Insert a realistic SQLite agent conversation that includes:

**Session metadata**:
- agent_name: "sqlite"
- provider: "mock"
- model: "mock-model"
- user_prompt: "How many users are in the database?"
- status: "completed"
- result: "There are 5 users in the database."

**Messages** (in chronological order):
1. **User message**: "How many users are in the database?"
2. **Assistant message** (tool call): Empty content with tool_use stop reason
3. **Tool message**: Contains tool call result (e.g., "5")
4. **Assistant message** (final response): "There are 5 users in the database."

**Tool calls**:
1. **sqlite3_reader tool call**:
   - tool_name: "sqlite3_reader"
   - request: `{"query": "SELECT COUNT(*) FROM users"}`
   - response: `{"result": "5"}`
   - status: "completed"
   - execution_time_ms: 15

### 3. Test Implementation

**Test function signature**:
```rust
#[actix_rt::test]
async fn test_get_session_with_mock_chat_history() -> anyhow::Result<()>
```

**Test steps**:
1. Setup test database using `setup_test_app()` from common module
2. Insert mock data directly into database:
   - Create session using `db.create_session()`
   - Create user message using `db.create_message(session_id, "user", content)`
   - Create assistant message for tool call using `db.create_message(session_id, "assistant", "")`
   - Create tool call using `db.create_tool_call()`
   - Complete tool call using `db.complete_tool_call()`
   - Create tool result message using `db.create_message(session_id, "tool", result)`
   - Create final assistant message using `db.create_message(session_id, "assistant", final_answer)`
   - Complete session using `db.complete_session(session_id, result)`
3. Call GET `/agents/sessions/{session_id}` endpoint using Actix test utilities
4. Validate HTTP response:
   - Status: 200 OK
   - Response structure matches `SessionResponse` type
5. Validate session metadata:
   - `id` matches the created session_id
   - `agent_name` is "sqlite"
   - `provider` is "mock"
   - `model` is "mock-model"
   - `user_prompt` matches the inserted prompt
   - `status` is "completed"
   - `result` matches the inserted result
6. Validate messages array:
   - Contains exactly 4 messages
   - Messages are ordered by `created_at` ASC
   - Message 0: role="user", content="How many users are in the database?"
   - Message 1: role="assistant", content="" (tool call message)
   - Message 2: role="tool", content contains tool result
   - Message 3: role="assistant", content="There are 5 users in the database."
7. Validate tool_calls array:
   - Contains exactly 1 tool call
   - Tool call has:
     - `tool_name` = "sqlite3_reader"
     - `request` contains the SQL query
     - `response` contains the result
     - `status` = "completed"
     - `execution_time_ms` is present and > 0

### 4. Key Differences from Existing Test
- **No API endpoint for data creation**: Insert data directly using `db.create_session()`, `db.create_message()`, etc.
- **Full chat history**: Include messages and tool calls (which current agents don't persist yet)
- **Read-only API validation**: Only tests the GET endpoint, not POST
- **Realistic conversation flow**: Simulates the complete agent execution lifecycle

## Implementation Pattern

```rust
mod common;

use actix_web::test;
use actix_web::test::TestRequest;
use common::setup_test_app;

#[actix_rt::test]
async fn test_get_session_with_mock_chat_history() -> anyhow::Result<()> {
    let test_app = setup_test_app().await?;
    let db = &test_app.db;

    // 1. Create session
    let session_id = db.create_session(
        "sqlite",
        "mock",
        "mock-model",
        None,
        "How many users are in the database?",
        Some(serde_json::json!({
            "Sqlite": {
                "db_path": "/test/database.db"
            }
        })),
    )?;

    // 2. Create messages (with small time delays to ensure ordering)
    let base_time = chrono::Utc::now().timestamp();

    // User message
    db.create_message(session_id, "user", "How many users are in the database?")?;

    // Assistant message (tool call)
    db.create_message(session_id, "assistant", "")?;

    // Tool call
    let tool_call_id = db.create_tool_call(
        session_id,
        None,
        "call_123",
        "sqlite3_reader",
        serde_json::json!({
            "query": "SELECT COUNT(*) FROM users"
        }),
    )?;

    db.complete_tool_call(
        tool_call_id,
        serde_json::json!({"result": "5"}),
        15,
    )?;

    // Tool result message
    db.create_message(session_id, "tool", "5")?;

    // Final assistant message
    db.create_message(session_id, "assistant", "There are 5 users in the database.")?;

    // 3. Complete session
    db.complete_session(session_id, "There are 5 users in the database.")?;

    // 4. Call GET endpoint
    let req = TestRequest::get()
        .uri(format!("/agents/sessions/{}", session_id).as_str())
        .to_request();

    let resp = test::call_service(&test_app.app, req).await;

    // 5. Validate response
    assert!(resp.status().is_success());

    let body_bytes = test::read_body(resp).await;
    let session_resp: serde_json::Value = serde_json::from_slice(&body_bytes)?;

    // 6. Assert session metadata
    assert_eq!(session_resp["id"], session_id);
    assert_eq!(session_resp["agent_name"], "sqlite");
    assert_eq!(session_resp["provider"], "mock");
    assert_eq!(session_resp["model"], "mock-model");
    assert_eq!(session_resp["user_prompt"], "How many users are in the database?");
    assert_eq!(session_resp["status"], "completed");
    assert_eq!(session_resp["result"], "There are 5 users in the database.");

    // 7. Assert messages
    let messages = session_resp["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 4, "Expected 4 messages");

    assert_eq!(messages[0]["role"], "user");
    assert_eq!(messages[0]["content"], "How many users are in the database?");

    assert_eq!(messages[1]["role"], "assistant");
    assert_eq!(messages[1]["content"], "");

    assert_eq!(messages[2]["role"], "tool");
    assert_eq!(messages[2]["content"], "5");

    assert_eq!(messages[3]["role"], "assistant");
    assert_eq!(messages[3]["content"], "There are 5 users in the database.");

    // 8. Assert tool calls
    let tool_calls = session_resp["tool_calls"].as_array().unwrap();
    assert_eq!(tool_calls.len(), 1, "Expected 1 tool call");

    assert_eq!(tool_calls[0]["tool_name"], "sqlite3_reader");
    assert_eq!(tool_calls[0]["status"], "completed");
    assert!(tool_calls[0]["execution_time_ms"].as_i64().unwrap() > 0);

    let request = &tool_calls[0]["request"];
    assert!(request["query"].as_str().unwrap().contains("SELECT COUNT(*)"));

    let response = &tool_calls[0]["response"];
    assert_eq!(response["result"], "5");

    println!("✓ Chat history integration test passed!");
    Ok(())
}
```

## Success Criteria
- [x] Test file created at `nocodo-api/tests/chat_history_test.rs`
- [x] Test inserts data directly into database (not via API)
- [x] Test validates complete session metadata retrieval
- [x] Test validates messages array with correct order and content
- [x] Test validates tool_calls array with complete data
- [x] Test passes successfully
- [x] Test is self-contained and can run in isolation

## Benefits
1. **Validates read endpoint independently**: Tests the GET endpoint without coupling to the POST endpoint
2. **Tests message/tool persistence**: Even though agents don't save messages yet, this validates the database schema and retrieval logic
3. **Realistic conversation simulation**: Provides a reference for what a complete agent conversation should look like
4. **Database layer validation**: Tests that the database methods work correctly together
5. **Future-proof**: When agents start persisting messages/tools, this test ensures the endpoint returns them correctly

## Out of Scope
- Testing the POST `/agents/sqlite/execute` endpoint (already covered by existing test)
- Testing error scenarios (invalid session_id, etc.)
- Testing other agent types
- Testing streaming or real-time updates

## References
- Existing test: `nocodo-api/tests/agent_execution_test.rs`
- Database methods: `nocodo-agents/src/database/mod.rs:125-327`
- Session endpoint handler: `nocodo-api/src/handlers/sessions.rs:8-35`
- Session retrieval logic: `nocodo-api/src/handlers/sessions.rs:37-122`
