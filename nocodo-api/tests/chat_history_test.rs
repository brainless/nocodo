mod common;

use actix_web::test;
use actix_web::test::TestRequest;
use common::setup_test_app;

#[actix_rt::test]
async fn test_get_session_with_mock_chat_history() -> anyhow::Result<()> {
    let test_app = setup_test_app().await?;
    let db = &test_app.db;

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

    db.create_message(session_id, "user", "How many users are in the database?")?;

    db.create_message(session_id, "assistant", "")?;

    let tool_call_id = db.create_tool_call(
        session_id,
        None,
        "call_123",
        "sqlite3_reader",
        serde_json::json!({
            "query": "SELECT COUNT(*) FROM users"
        }),
    )?;

    db.complete_tool_call(tool_call_id, serde_json::json!({"result": "5"}), 15)?;

    db.create_message(session_id, "tool", "5")?;

    db.create_message(
        session_id,
        "assistant",
        "There are 5 users in the database.",
    )?;

    db.complete_session(session_id, "There are 5 users in the database.")?;

    let req = TestRequest::get()
        .uri(format!("/agents/sessions/{}", session_id).as_str())
        .to_request();

    let resp = test::call_service(&test_app.app, req).await;

    assert!(resp.status().is_success());

    let body_bytes = test::read_body(resp).await;
    let session_resp: serde_json::Value = serde_json::from_slice(&body_bytes)?;

    assert_eq!(session_resp["id"], session_id);
    assert_eq!(session_resp["agent_name"], "sqlite");
    assert_eq!(session_resp["provider"], "mock");
    assert_eq!(session_resp["model"], "mock-model");
    assert_eq!(
        session_resp["user_prompt"],
        "How many users are in the database?"
    );
    assert_eq!(session_resp["status"], "completed");
    assert_eq!(session_resp["result"], "There are 5 users in the database.");

    let messages = session_resp["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 4, "Expected 4 messages");

    assert_eq!(messages[0]["role"], "user");
    assert_eq!(
        messages[0]["content"],
        "How many users are in the database?"
    );

    assert_eq!(messages[1]["role"], "assistant");
    assert_eq!(messages[1]["content"], "");

    assert_eq!(messages[2]["role"], "tool");
    assert_eq!(messages[2]["content"], "5");

    assert_eq!(messages[3]["role"], "assistant");
    assert_eq!(messages[3]["content"], "There are 5 users in the database.");

    let tool_calls = session_resp["tool_calls"].as_array().unwrap();
    assert_eq!(tool_calls.len(), 1, "Expected 1 tool call");

    assert_eq!(tool_calls[0]["tool_name"], "sqlite3_reader");
    assert_eq!(tool_calls[0]["status"], "completed");
    assert!(tool_calls[0]["execution_time_ms"].as_i64().unwrap() > 0);

    let request = &tool_calls[0]["request"];
    assert!(request["query"]
        .as_str()
        .unwrap()
        .contains("SELECT COUNT(*)"));

    let response = &tool_calls[0]["response"];
    assert_eq!(response["result"], "5");

    println!("✓ Chat history integration test passed!");
    Ok(())
}
