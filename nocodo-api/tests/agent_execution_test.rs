mod common;

use actix_web::test;
use actix_web::test::TestRequest;
use common::{setup_test_app, setup_test_sqlite_db_with_data};
use shared_types::{AgentConfig, AgentExecutionRequest, SqliteAgentConfig};

#[actix_rt::test]
async fn test_sqlite_agent_execution_and_persistence() -> anyhow::Result<()> {
    let test_app = setup_test_app().await?;
    let _db = &test_app.db;

    let (_temp_file, db_path) = setup_test_sqlite_db_with_data()?;
    println!("Created test SQLite database at: {}", db_path);

    let request_body = AgentExecutionRequest {
        user_prompt: "How many users are in database?".to_string(),
        config: AgentConfig::Sqlite(SqliteAgentConfig { db_path }),
    };

    let req = TestRequest::post()
        .uri("/agents/sqlite/execute")
        .set_json(&request_body)
        .to_request();

    let resp = test::call_service(&test_app.app, req).await;
    let status = resp.status();
    println!("Response status: {}", status);

    let body_bytes = test::read_body(resp).await;
    let body_str = String::from_utf8_lossy(&body_bytes);
    println!("Response body: {}", body_str);

    if !status.is_success() {
        eprintln!("Error response body: {}", body_str);
        panic!("Request failed with status: {}", status);
    }

    let resp_value: serde_json::Value = serde_json::from_slice(&body_bytes)
        .expect("Failed to parse response as JSON");

    assert_eq!(
        resp_value["status"], "completed",
        "Expected status to be completed"
    );
    assert!(
        resp_value["session_id"].is_number(),
        "Expected session_id to be a number"
    );
    assert_eq!(
        resp_value["agent_name"], "sqlite",
        "Expected agent_name to be sqlite"
    );
    assert!(resp_value["result"].is_string(), "Expected result to be a string");
    assert!(
        !resp_value["result"].as_str().unwrap().is_empty(),
        "Expected result to not be empty"
    );

    let session_id = resp_value["session_id"].as_i64().unwrap();

    let session_req = TestRequest::get()
        .uri(format!("/agents/sessions/{}", session_id).as_str())
        .to_request();

    let session_resp = test::call_service(&test_app.app, session_req).await;
    let session_status = session_resp.status();
    println!("Session response status: {}", session_status);

    let session_body_bytes = test::read_body(session_resp).await;
    let session_body_str = String::from_utf8_lossy(&session_body_bytes);
    println!("Session response body: {}", session_body_str);

    if !session_status.is_success() {
        eprintln!("Error session response body: {}", session_body_str);
        panic!("Session request failed with status: {}", session_status);
    }

    let session_resp: serde_json::Value = serde_json::from_slice(&session_body_bytes)
        .expect("Failed to parse session response as JSON");

    assert_eq!(
        session_resp["id"], session_id,
        "Expected matching session_id"
    );
    assert_eq!(
        session_resp["agent_name"], "sqlite",
        "Expected agent_name to be sqlite"
    );
    assert_eq!(
        session_resp["provider"], "mock",
        "Expected provider to be mock"
    );
    assert_eq!(
        session_resp["model"], "mock-model",
        "Expected model to be mock-model"
    );
    assert_eq!(
        session_resp["status"], "completed",
        "Expected status to be completed"
    );

    assert!(
        session_resp["result"].is_string(),
        "Expected result to be a string"
    );
    assert!(
        !session_resp["result"].as_str().unwrap().is_empty(),
        "Expected result to not be empty"
    );

    // NOTE: Message and tool call persistence is not yet implemented in agents.
    // The agents don't currently save individual messages and tool calls to the database.
    // This will need to be implemented in the future by updating the Agent trait to accept
    // a session_id and having agents call database.save_message() and database.save_tool_call()
    // during execution.

    let messages = session_resp["messages"].as_array().unwrap();
    let tool_calls = session_resp["tool_calls"].as_array().unwrap();

    println!("Messages count: {}", messages.len());
    println!("Tool calls count: {}", tool_calls.len());

    // TODO: Once message/tool call persistence is implemented, uncomment these assertions:
    // assert!(!messages.is_empty(), "Expected at least one message in the session");
    // let user_msg = messages.iter().find(|m| m["role"] == "user").expect("Expected user message");
    // assert_eq!(user_msg["content"], "How many users are in database?");
    // assert!(!assistant_msgs.is_empty(), "Expected at least one assistant message");

    println!("✓ Integration test passed!");
    println!("✓ Verified agent execution completes successfully");
    println!("✓ Verified session is created and persisted");
    println!("✓ Verified session can be retrieved via API");
    println!("✓ Verified session contains correct metadata");
    Ok(())
}
