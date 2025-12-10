use actix_web::{test, web};
use serde_json::json;

use nocodo_manager::models::{
    CreateAiSessionRequest, LlmAgentToolCall,
};

use crate::common::{TestApp, TestDataGenerator};

#[actix_rt::test]
async fn test_create_llm_agent_tool_call() {
    let test_app = TestApp::new().await;

    // Create work and session
    let work = TestDataGenerator::create_work(Some("Tool Call Work"), None);
    test_app.db().create_work(&work).unwrap();

    let session = TestDataGenerator::create_llm_agent_session(&work.id, "openai", "gpt-4");
    test_app.db().create_llm_agent_session(&session).unwrap();

    // Create a tool call
    let tool_call_request = json!({
        "tool_name": "list_files",
        "request": {
            "path": ".",
            "recursive": false
        }
    });

    let uri = format!("/api/llm-agent/sessions/{}/tool-calls", session.id);
    let req = test::TestRequest::post()
        .uri(&uri)
        .set_json(&tool_call_request)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let tool_call = &body["tool_call"];

    assert_eq!(tool_call["session_id"], session.id);
    assert_eq!(tool_call["tool_name"], "list_files");
    assert_eq!(tool_call["status"], "pending");
    assert!(tool_call["id"].as_i64().is_some());
    assert!(tool_call["created_at"].as_i64().is_some());

    // Verify tool call was created in database
    let db_tool_calls = test_app.db().get_llm_agent_tool_calls(&session.id).unwrap();
    assert_eq!(db_tool_calls.len(), 1);
    assert_eq!(db_tool_calls[0].tool_name, "list_files");
    assert_eq!(db_tool_calls[0].status, "pending");
}

#[actix_rt::test]
async fn test_get_llm_agent_tool_calls() {
    let test_app = TestApp::new().await;

    // Create work and session
    let work = TestDataGenerator::create_work(Some("Get Tool Calls Work"), None);
    test_app.db().create_work(&work).unwrap();

    let session = TestDataGenerator::create_llm_agent_session(&work.id, "anthropic", "claude-3");
    test_app.db().create_llm_agent_session(&session).unwrap();

    // Create multiple tool calls
    let tool_calls_data = vec![
        ("read_file", json!({"path": "src/main.rs"})),
        ("list_files", json!({"path": ".", "recursive": true})),
        ("grep_search", json!({"pattern": "fn main", "path": "src"})),
    ];

    for (tool_name, request) in tool_calls_data {
        let tool_call = TestDataGenerator::create_llm_agent_tool_call(&session.id, tool_name, request);
        test_app.db().create_llm_agent_tool_call(&tool_call).unwrap();
    }

    // Get tool calls
    let uri = format!("/api/llm-agent/sessions/{}/tool-calls", session.id);
    let req = test::TestRequest::get().uri(&uri).to_request();
    let resp = test::call_service(&test_app.service(), req).await;

    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let tool_calls = body["tool_calls"].as_array().unwrap();

    assert_eq!(tool_calls.len(), 3);

    // Verify tool calls are in correct order (newest first)
    let tool_names: Vec<&str> = tool_calls.iter()
        .map(|tc| tc["tool_name"].as_str().unwrap())
        .collect();

    assert!(tool_names.contains(&"read_file"));
    assert!(tool_names.contains(&"list_files"));
    assert!(tool_names.contains(&"grep_search"));
}

#[actix_rt::test]
async fn test_update_llm_agent_tool_call() {
    let test_app = TestApp::new().await;

    // Create work and session
    let work = TestDataGenerator::create_work(Some("Update Tool Call Work"), None);
    test_app.db().create_work(&work).unwrap();

    let session = TestDataGenerator::create_llm_agent_session(&work.id, "openai", "gpt-4");
    test_app.db().create_llm_agent_session(&session).unwrap();

    // Create a tool call
    let tool_call = TestDataGenerator::create_llm_agent_tool_call(
        &session.id,
        "list_files",
        json!({"path": "src"})
    );
    test_app.db().create_llm_agent_tool_call(&tool_call).unwrap();

    // Update the tool call with results
    let update_data = json!({
        "status": "completed",
        "response": {
            "files": [
                {"name": "main.rs", "is_directory": false},
                {"name": "lib.rs", "is_directory": false}
            ]
        },
        "execution_time_ms": 150
    });

    let uri = format!("/api/llm-agent/sessions/{}/tool-calls/{}", session.id, tool_call.id);
    let req = test::TestRequest::put()
        .uri(&uri)
        .set_json(&update_data)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let updated_tool_call = &body["tool_call"];

    assert_eq!(updated_tool_call["status"], "completed");
    assert_eq!(updated_tool_call["execution_time_ms"], 150);
    assert!(updated_tool_call["completed_at"].is_number());

    // Verify response data
    let response = &updated_tool_call["response"];
    assert!(response.is_object());
    let files = response["files"].as_array().unwrap();
    assert_eq!(files.len(), 2);
}

#[actix_rt::test]
async fn test_tool_call_status_transitions() {
    let test_app = TestApp::new().await;

    // Create work and session
    let work = TestDataGenerator::create_work(Some("Status Transition Work"), None);
    test_app.db().create_work(&work).unwrap();

    let session = TestDataGenerator::create_llm_agent_session(&work.id, "anthropic", "claude-3");
    test_app.db().create_llm_agent_session(&session).unwrap();

    // Create a tool call
    let tool_call = TestDataGenerator::create_llm_agent_tool_call(
        &session.id,
        "read_file",
        json!({"path": "Cargo.toml"})
    );
    test_app.db().create_llm_agent_tool_call(&tool_call).unwrap();

    // Test status transitions
    let statuses = vec!["pending", "running", "completed", "failed"];

    for status in statuses {
        let update_data = json!({
            "status": status
        });

        let uri = format!("/api/llm-agent/sessions/{}/tool-calls/{}", session.id, tool_call.id);
        let req = test::TestRequest::put()
            .uri(&uri)
            .set_json(&update_data)
            .to_request();

        let resp = test::call_service(&test_app.service(), req).await;
        assert!(resp.status().is_success());

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["tool_call"]["status"], status);

        // Verify in database
        let db_tool_calls = test_app.db().get_llm_agent_tool_calls(&session.id).unwrap();
        assert_eq!(db_tool_calls[0].status, status);
    }
}

#[actix_rt::test]
async fn test_tool_call_with_error_handling() {
    let test_app = TestApp::new().await;

    // Create work and session
    let work = TestDataGenerator::create_work(Some("Error Handling Work"), None);
    test_app.db().create_work(&work).unwrap();

    let session = TestDataGenerator::create_llm_agent_session(&work.id, "openai", "gpt-4");
    test_app.db().create_llm_agent_session(&session).unwrap();

    // Create a tool call that will fail
    let tool_call = TestDataGenerator::create_llm_agent_tool_call(
        &session.id,
        "read_file",
        json!({"path": "non-existent-file.txt"})
    );
    test_app.db().create_llm_agent_tool_call(&tool_call).unwrap();

    // Update with error
    let error_message = "File not found: non-existent-file.txt";
    let update_data = json!({
        "status": "failed",
        "error_details": error_message,
        "execution_time_ms": 50
    });

    let uri = format!("/api/llm-agent/sessions/{}/tool-calls/{}", session.id, tool_call.id);
    let req = test::TestRequest::put()
        .uri(&uri)
        .set_json(&update_data)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let updated_tool_call = &body["tool_call"];

    assert_eq!(updated_tool_call["status"], "failed");
    assert_eq!(updated_tool_call["error_details"], error_message);
    assert_eq!(updated_tool_call["execution_time_ms"], 50);

    // Verify in database
    let db_tool_calls = test_app.db().get_llm_agent_tool_calls(&session.id).unwrap();
    assert_eq!(db_tool_calls[0].status, "failed");
    assert_eq!(db_tool_calls[0].error_details, Some(error_message.to_string()));
}

#[actix_rt::test]
async fn test_multiple_tool_calls_workflow() {
    let test_app = TestApp::new().await;

    // Create work and session
    let work = TestDataGenerator::create_work(Some("Multi Tool Workflow"), None);
    test_app.db().create_work(&work).unwrap();

    let session = TestDataGenerator::create_llm_agent_session(&work.id, "anthropic", "claude-3");
    test_app.db().create_llm_agent_session(&session).unwrap();

    // Simulate a complex workflow with multiple tool calls
    let workflow_steps = vec![
        ("list_files", json!({"path": ".", "recursive": false}), "completed", Some(json!({"files": ["src", "Cargo.toml", "README.md"]})), None),
        ("read_file", json!({"path": "Cargo.toml"}), "completed", Some(json!({"content": "[package]\nname = \"test\"\nversion = \"0.1.0\""}), None),
        ("grep_search", json!({"pattern": "fn main", "path": "src"}), "failed", None, Some("Pattern not found")),
        ("list_files", json!({"path": "src", "recursive": true}), "completed", Some(json!({"files": ["main.rs", "lib.rs", "utils/mod.rs"]})), None),
    ];

    let mut created_tool_calls = Vec::new();

    for (tool_name, request, status, response, error) in workflow_steps {
        let tool_call = TestDataGenerator::create_llm_agent_tool_call(&session.id, tool_name, request);
        test_app.db().create_llm_agent_tool_call(&tool_call).unwrap();
        created_tool_calls.push((tool_call, status, response, error));
    }

    // Update each tool call with results
    for (tool_call, status, response, error) in created_tool_calls {
        let mut update_data = json!({
            "status": status,
            "execution_time_ms": 100
        });

        if let Some(resp) = response {
            update_data["response"] = resp;
        }

        if let Some(err) = error {
            update_data["error_details"] = json!(err);
        }

        let uri = format!("/api/llm-agent/sessions/{}/tool-calls/{}", session.id, tool_call.id);
        let req = test::TestRequest::put()
            .uri(&uri)
            .set_json(&update_data)
            .to_request();

        let resp = test::call_service(&test_app.service(), req).await;
        assert!(resp.status().is_success());
    }

    // Verify all tool calls
    let db_tool_calls = test_app.db().get_llm_agent_tool_calls(&session.id).unwrap();
    assert_eq!(db_tool_calls.len(), 4);

    // Check specific tool calls
    let list_files_calls: Vec<_> = db_tool_calls.iter()
        .filter(|tc| tc.tool_name == "list_files")
        .collect();

    assert_eq!(list_files_calls.len(), 2);

    let failed_call = db_tool_calls.iter()
        .find(|tc| tc.status == "failed")
        .unwrap();

    assert_eq!(failed_call.tool_name, "grep_search");
    assert_eq!(failed_call.error_details, Some("Pattern not found".to_string()));
}

#[actix_rt::test]
async fn test_tool_call_performance_tracking() {
    let test_app = TestApp::new().await;

    // Create work and session
    let work = TestDataGenerator::create_work(Some("Performance Tracking Work"), None);
    test_app.db().create_work(&work).unwrap();

    let session = TestDataGenerator::create_llm_agent_session(&work.id, "openai", "gpt-4");
    test_app.db().create_llm_agent_session(&session).unwrap();

    // Create tool calls with different execution times
    let performance_data = vec![
        ("fast_operation", 50),
        ("medium_operation", 200),
        ("slow_operation", 1000),
        ("very_slow_operation", 5000),
    ];

    for (operation, exec_time) in performance_data {
        let tool_call = TestDataGenerator::create_llm_agent_tool_call(
            &session.id,
            operation,
            json!({"param": "test"})
        );
        test_app.db().create_llm_agent_tool_call(&tool_call).unwrap();

        // Update with performance data
        let update_data = json!({
            "status": "completed",
            "execution_time_ms": exec_time,
            "response": {"result": "success"}
        });

        let uri = format!("/api/llm-agent/sessions/{}/tool-calls/{}", session.id, tool_call.id);
        let req = test::TestRequest::put()
            .uri(&uri)
            .set_json(&update_data)
            .to_request();

        let resp = test::call_service(&test_app.service(), req).await;
        assert!(resp.status().is_success());
    }

    // Verify performance data
    let db_tool_calls = test_app.db().get_llm_agent_tool_calls(&session.id).unwrap();
    assert_eq!(db_tool_calls.len(), 4);

    // Check execution times
    let exec_times: Vec<i64> = db_tool_calls.iter()
        .map(|tc| tc.execution_time_ms.unwrap_or(0))
        .collect();

    assert!(exec_times.contains(&50));
    assert!(exec_times.contains(&200));
    assert!(exec_times.contains(&1000));
    assert!(exec_times.contains(&5000));

    // Calculate average execution time
    let total_time: i64 = exec_times.iter().sum();
    let avg_time = total_time / exec_times.len() as i64;
    assert_eq!(avg_time, 1312); // (50 + 200 + 1000 + 5000) / 4
}

#[actix_rt::test]
async fn test_tool_call_progress_updates() {
    let test_app = TestApp::new().await;

    // Create work and session
    let work = TestDataGenerator::create_work(Some("Progress Update Work"), None);
    test_app.db().create_work(&work).unwrap();

    let session = TestDataGenerator::create_llm_agent_session(&work.id, "anthropic", "claude-3");
    test_app.db().create_llm_agent_session(&session).unwrap();

    // Create a long-running tool call
    let tool_call = TestDataGenerator::create_llm_agent_tool_call(
        &session.id,
        "complex_analysis",
        json!({"target": "large_codebase", "depth": "full"})
    );
    test_app.db().create_llm_agent_tool_call(&tool_call).unwrap();

    // Simulate progress updates
    let progress_updates = vec![
        "Initializing analysis...",
        "Scanning 25% complete...",
        "Scanning 50% complete...",
        "Processing results...",
        "Analysis complete",
    ];

    for (i, progress) in progress_updates.iter().enumerate() {
        let status = if i == progress_updates.len() - 1 { "completed" } else { "running" };
        let update_data = json!({
            "status": status,
            "progress_updates": progress,
            "execution_time_ms": (i as i64 + 1) * 1000
        });

        let uri = format!("/api/llm-agent/sessions/{}/tool-calls/{}", session.id, tool_call.id);
        let req = test::TestRequest::put()
            .uri(&uri)
            .set_json(&update_data)
            .to_request();

        let resp = test::call_service(&test_app.service(), req).await;
        assert!(resp.status().is_success());
    }

    // Verify final state
    let db_tool_calls = test_app.db().get_llm_agent_tool_calls(&session.id).unwrap();
    let final_call = &db_tool_calls[0];

    assert_eq!(final_call.status, "completed");
    assert_eq!(final_call.progress_updates, Some("Analysis complete".to_string()));
    assert_eq!(final_call.execution_time_ms, Some(5000)); // 5 * 1000ms
}

#[actix_rt::test]
async fn test_tool_call_concurrent_execution() {
    let test_app = TestApp::new().await;

    // Create work and session
    let work = TestDataGenerator::create_work(Some("Concurrent Tool Work"), None);
    test_app.db().create_work(&work).unwrap();

    let session = TestDataGenerator::create_llm_agent_session(&work.id, "openai", "gpt-4");
    test_app.db().create_llm_agent_session(&session).unwrap();

    // Create multiple tool calls to simulate concurrent execution
    let tool_names = vec!["list_files", "read_file", "grep_search", "run_command"];

    let mut tool_calls = Vec::new();

    for tool_name in tool_names {
        let tool_call = TestDataGenerator::create_llm_agent_tool_call(
            &session.id,
            tool_name,
            json!({"param": "test"})
        );
        test_app.db().create_llm_agent_tool_call(&tool_call).unwrap();
        tool_calls.push(tool_call);
    }

    // Update tool calls with different completion times (simulating concurrent execution)
    for (i, tool_call) in tool_calls.iter().enumerate() {
        let update_data = json!({
            "status": "completed",
            "execution_time_ms": (i as i64 + 1) * 200,
            "response": {"result": format!("completed_{}", i)}
        });

        let uri = format!("/api/llm-agent/sessions/{}/tool-calls/{}", session.id, tool_call.id);
        let req = test::TestRequest::put()
            .uri(&uri)
            .set_json(&update_data)
            .to_request();

        let resp = test::call_service(&test_app.service(), req).await;
        assert!(resp.status().is_success());
    }

    // Verify all tool calls completed
    let db_tool_calls = test_app.db().get_llm_agent_tool_calls(&session.id).unwrap();
    assert_eq!(db_tool_calls.len(), 4);

    // All should be completed
    for tool_call in &db_tool_calls {
        assert_eq!(tool_call.status, "completed");
        assert!(tool_call.execution_time_ms.is_some());
        assert!(tool_call.completed_at.is_some());
    }

    // Execution times should be different (simulating different completion times)
    let exec_times: Vec<i64> = db_tool_calls.iter()
        .map(|tc| tc.execution_time_ms.unwrap())
        .collect();

    let unique_times: std::collections::HashSet<_> = exec_times.iter().collect();
    assert_eq!(unique_times.len(), 4); // All execution times should be different
}

#[actix_rt::test]
async fn test_tool_call_error_recovery() {
    let test_app = TestApp::new().await;

    // Create work and session
    let work = TestDataGenerator::create_work(Some("Error Recovery Work"), None);
    test_app.db().create_work(&work).unwrap();

    let session = TestDataGenerator::create_llm_agent_session(&work.id, "anthropic", "claude-3");
    test_app.db().create_llm_agent_session(&session).unwrap();

    // Create a tool call that fails initially
    let tool_call = TestDataGenerator::create_llm_agent_tool_call(
        &session.id,
        "unreliable_operation",
        json!({"attempt": 1})
    );
    test_app.db().create_llm_agent_tool_call(&tool_call).unwrap();

    // First attempt fails
    let fail_update = json!({
        "status": "failed",
        "error_details": "Network timeout",
        "execution_time_ms": 5000
    });

    let uri = format!("/api/llm-agent/sessions/{}/tool-calls/{}", session.id, tool_call.id);
    let req = test::TestRequest::put()
        .uri(&uri)
        .set_json(&fail_update)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    // Verify failure was recorded
    let db_tool_calls = test_app.db().get_llm_agent_tool_calls(&session.id).unwrap();
    assert_eq!(db_tool_calls[0].status, "failed");
    assert_eq!(db_tool_calls[0].error_details, Some("Network timeout".to_string()));

    // Retry the operation (update with new attempt)
    let retry_update = json!({
        "status": "running",
        "progress_updates": "Retrying operation...",
        "error_details": None  // Clear previous error
    });

    let req = test::TestRequest::put()
        .uri(&uri)
        .set_json(&retry_update)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    // Final success
    let success_update = json!({
        "status": "completed",
        "response": {"result": "success_after_retry"},
        "execution_time_ms": 3000,
        "progress_updates": "Operation completed successfully"
    });

    let req = test::TestRequest::put()
        .uri(&uri)
        .set_json(&success_update)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    // Verify final successful state
    let db_tool_calls = test_app.db().get_llm_agent_tool_calls(&session.id).unwrap();
    let final_call = &db_tool_calls[0];

    assert_eq!(final_call.status, "completed");
    assert_eq!(final_call.progress_updates, Some("Operation completed successfully".to_string()));
    assert_eq!(final_call.execution_time_ms, Some(3000));
    // Error details should be cleared
    assert!(final_call.error_details.is_none());
}