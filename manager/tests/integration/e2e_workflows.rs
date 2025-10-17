use actix_web::{test, web};
use serde_json::json;
use std::fs;

use nocodo_manager::models::{
    CreateProjectRequest, CreateWorkRequest, CreateAiSessionRequest,
    CreateLlmAgentSessionRequest, FileCreateRequest, FileUpdateRequest,
    MessageAuthorType, MessageContentType,
};

use crate::common::{TestApp, TestDataGenerator};

#[actix_rt::test]
async fn test_complete_project_creation_workflow() {
    let test_app = TestApp::new().await;

    // 1. Create a project with template
    let project_temp_dir = test_app.test_config().projects_dir().join("e2e-project");
    let create_project_req = CreateProjectRequest {
        name: "e2e-project".to_string(),
        path: Some(project_temp_dir.to_string_lossy().to_string()),
        description: None,
        parent_id: None,
        template: Some("rust-web-api".to_string()),
    };

    let req = test::TestRequest::post()
        .uri("/api/projects")
        .set_json(&create_project_req)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let project_id = body["project"]["id"].as_str().unwrap();

    // 2. Verify project was created with correct structure
    assert!(project_temp_dir.join("Cargo.toml").exists());
    assert!(project_temp_dir.join("src").join("main.rs").exists());
    assert!(project_temp_dir.join("src").join("lib.rs").exists());

    // 3. List projects to verify it's in the list
    let list_req = test::TestRequest::get().uri("/api/projects").to_request();
    let list_resp = test::call_service(&test_app.service(), list_req).await;
    assert!(list_resp.status().is_success());

    let list_body: serde_json::Value = test::read_body_json(list_resp).await;
    let projects = list_body["projects"].as_array().unwrap();
    assert!(projects.iter().any(|p| p["id"] == project_id));

    // 4. Get specific project details
    let get_uri = format!("/api/projects/{}", project_id);
    let get_req = test::TestRequest::get().uri(&get_uri).to_request();
    let get_resp = test::call_service(&test_app.service(), get_req).await;
    assert!(get_resp.status().is_success());

    let get_body: serde_json::Value = test::read_body_json(get_resp).await;
    let project = &get_body["project"];

    assert_eq!(project["name"], "e2e-project");
    assert_eq!(project["language"], "rust");
    assert_eq!(project["framework"], "actix-web");
    assert_eq!(project["status"], "initialized");
}

#[actix_rt::test]
async fn test_ai_powered_development_workflow() {
    let test_app = TestApp::new().await;

    // 1. Create project
    let project = TestDataGenerator::create_project(Some("ai-dev-project"), Some("/tmp/ai-dev-project"));
    test_app.db().create_project(&project).unwrap();
    fs::create_dir_all(&project.path).unwrap();

    // 2. Create work session
    let work = TestDataGenerator::create_work(Some("AI Development Session"), Some(&project.id));
    test_app.db().create_work(&work).unwrap();

    // 3. Add user message
    let user_message = TestDataGenerator::create_work_message(
        &work.id,
        "Help me create a simple Rust web server with error handling",
        MessageAuthorType::User,
        0,
    );
    test_app.db().create_work_message(&user_message).unwrap();

    // 4. Create AI session
    let ai_session = TestDataGenerator::create_ai_session(&work.id, &user_message.id, "llm_agent");
    test_app.db().create_ai_session(&ai_session).unwrap();

    // 5. Create LLM agent session
    let llm_session = TestDataGenerator::create_llm_agent_session(&work.id, "openai", "gpt-4");
    test_app.db().create_llm_agent_session(&llm_session).unwrap();

    // 6. Simulate AI response with tool calls
    let ai_response_message = TestDataGenerator::create_work_message(
        &work.id,
        "I'll help you create a Rust web server. Let me first check the current project structure and then create the necessary files.",
        MessageAuthorType::Ai,
        1,
    );
    test_app.db().create_work_message(&ai_response_message).unwrap();

    // 7. Create tool calls for file operations
    let tool_calls = vec![
        TestDataGenerator::create_llm_agent_tool_call(
            &llm_session.id,
            "list_files",
            json!({"project_id": project.id, "path": "."})
        ),
        TestDataGenerator::create_llm_agent_tool_call(
            &llm_session.id,
            "create_file",
            json!({
                "project_id": project.id,
                "path": "src/main.rs",
                "content": "use actix_web::{web, App, HttpServer, Result};\n\nasync fn hello() -> Result<String> {\n    Ok(\"Hello from Rust server!\".to_string())\n}\n\n#[actix_web::main]\nasync fn main() -> std::io::Result<()> {\n    println!(\"Starting server on http://localhost:8080\");\n    \n    HttpServer::new(|| {\n        App::new()\n            .route(\"/\", web::get().to(hello))\n    })\n    .bind(\"127.0.0.1:8080\")?\n    .run()\n    .await\n}",
                "is_directory": false
            })
        ),
        TestDataGenerator::create_llm_agent_tool_call(
            &llm_session.id,
            "create_file",
            json!({
                "project_id": project.id,
                "path": "Cargo.toml",
                "content": "[package]\nname = \"ai-dev-project\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nactix-web = \"4.0\"\nactix-rt = \"2.7\"",
                "is_directory": false
            })
        ),
    ];

    for tool_call in &tool_calls {
        test_app.db().create_llm_agent_tool_call(tool_call).unwrap();
    }

    // 8. Update tool calls with results
    for tool_call in &tool_calls {
        let update_data = json!({
            "status": "completed",
            "execution_time_ms": 100,
            "response": {"result": "success"}
        });

        let uri = format!("/api/llm-agent/sessions/{}/tool-calls/{}", llm_session.id, tool_call.id);
        let req = test::TestRequest::put()
            .uri(&uri)
            .set_json(&update_data)
            .to_request();

        let resp = test::call_service(&test_app.service(), req).await;
        assert!(resp.status().is_success());
    }

    // 9. Verify the complete workflow
    // Check work messages
    let messages = test_app.db().get_work_messages(&work.id).unwrap();
    assert_eq!(messages.len(), 2);

    // Check AI sessions
    let ai_sessions = test_app.db().get_ai_sessions_by_work_id(&work.id).unwrap();
    assert_eq!(ai_sessions.len(), 1);

    // Check LLM agent sessions
    let llm_sessions = test_app.db().get_llm_agent_sessions_by_work(&work.id).unwrap();
    assert_eq!(llm_sessions.len(), 1);

    // Check tool calls
    let tool_calls_db = test_app.db().get_llm_agent_tool_calls(&llm_session.id).unwrap();
    assert_eq!(tool_calls_db.len(), 3);

    // Verify files were created
    assert!(std::path::Path::new(&project.path).join("src").join("main.rs").exists());
    assert!(std::path::Path::new(&project.path).join("Cargo.toml").exists());

    // Verify file contents
    let main_rs_content = fs::read_to_string(std::path::Path::new(&project.path).join("src").join("main.rs")).unwrap();
    assert!(main_rs_content.contains("actix_web"));
    assert!(main_rs_content.contains("hello"));

    let cargo_toml_content = fs::read_to_string(std::path::Path::new(&project.path).join("Cargo.toml")).unwrap();
    assert!(cargo_toml_content.contains("actix-web"));
    assert!(cargo_toml_content.contains("ai-dev-project"));
}

#[actix_rt::test]
async fn test_multi_user_collaboration_workflow() {
    let test_app = TestApp::new().await;

    // 1. Create shared project
    let project = TestDataGenerator::create_project(Some("collaboration-project"), Some("/tmp/collaboration-project"));
    test_app.db().create_project(&project).unwrap();
    fs::create_dir_all(&project.path).unwrap();

    // 2. Simulate multiple users working on the same project
    let user_workflows = vec![
        ("user1", "Add user authentication"),
        ("user2", "Implement database models"),
        ("user3", "Create API endpoints"),
    ];

    for (user_id, task) in user_workflows {
        // Create work session for each user
        let work = TestDataGenerator::create_work(
            Some(&format!("{} - {}", user_id, task)),
            Some(&project.id)
        );
        test_app.db().create_work(&work).unwrap();

        // Add user message
        let user_message = TestDataGenerator::create_work_message(
            &work.id,
            &format!("Please help me with: {}", task),
            MessageAuthorType::User,
            0,
        );
        test_app.db().create_work_message(&user_message).unwrap();

        // Create AI session
        let ai_session = TestDataGenerator::create_ai_session(&work.id, &user_message.id, "llm_agent");
        test_app.db().create_ai_session(&ai_session).unwrap();

        // Create LLM agent session
        let llm_session = TestDataGenerator::create_llm_agent_session(&work.id, "anthropic", "claude-3");
        test_app.db().create_llm_agent_session(&llm_session).unwrap();

        // Simulate AI response
        let ai_message = TestDataGenerator::create_work_message(
            &work.id,
            &format!("I'll help you implement {} for the project.", task),
            MessageAuthorType::Ai,
            1,
        );
        test_app.db().create_work_message(&ai_message).unwrap();
    }

    // 3. Verify all workflows are properly isolated
    let all_works = test_app.db().get_all_works().unwrap();
    assert_eq!(all_works.len(), 3);

    let all_ai_sessions = test_app.db().get_all_ai_sessions().unwrap();
    assert_eq!(all_ai_sessions.len(), 3);

    let all_llm_sessions = test_app.db().get_llm_agent_sessions_by_work(&project.id).unwrap();
    assert_eq!(all_llm_sessions.len(), 3);

    // 4. Verify project relationships
    for work in &all_works {
        assert_eq!(work.project_id, Some(project.id.clone()));
    }

    // 5. Verify work isolation
    for work in &all_works {
        let messages = test_app.db().get_work_messages(&work.id).unwrap();
        assert_eq!(messages.len(), 2); // One user message + one AI response

        let work_ai_sessions = test_app.db().get_ai_sessions_by_work_id(&work.id).unwrap();
        assert_eq!(work_ai_sessions.len(), 1);

        let work_llm_sessions = test_app.db().get_llm_agent_sessions_by_work(&work.id).unwrap();
        assert_eq!(work_llm_sessions.len(), 1);
    }
}

#[actix_rt::test]
async fn test_error_recovery_and_retry_workflow() {
    let test_app = TestApp::new().await;

    // 1. Create project
    let project = TestDataGenerator::create_project(Some("error-recovery-project"), Some("/tmp/error-recovery-project"));
    test_app.db().create_project(&project).unwrap();
    fs::create_dir_all(&project.path).unwrap();

    // 2. Create work session
    let work = TestDataGenerator::create_work(Some("Error Recovery Session"), Some(&project.id));
    test_app.db().create_work(&work).unwrap();

    // 3. Add user message
    let user_message = TestDataGenerator::create_work_message(
        &work.id,
        "Create a complex Rust application with multiple modules",
        MessageAuthorType::User,
        0,
    );
    test_app.db().create_work_message(&user_message).unwrap();

    // 4. Create AI session
    let ai_session = TestDataGenerator::create_ai_session(&work.id, &user_message.id, "llm_agent");
    test_app.db().create_ai_session(&ai_session).unwrap();

    // 5. Create LLM agent session
    let llm_session = TestDataGenerator::create_llm_agent_session(&work.id, "openai", "gpt-4");
    test_app.db().create_llm_agent_session(&llm_session).unwrap();

    // 6. Simulate failed tool calls and recovery
    let failed_tool_call = TestDataGenerator::create_llm_agent_tool_call(
        &llm_session.id,
        "create_file",
        json!({
            "project_id": project.id,
            "path": "src/complex_module.rs",
            "content": "invalid rust code {{{",
            "is_directory": false
        })
    );
    test_app.db().create_llm_agent_tool_call(&failed_tool_call).unwrap();

    // Update with failure
    let fail_update = json!({
        "status": "failed",
        "error_details": "Syntax error in generated code",
        "execution_time_ms": 500
    });

    let uri = format!("/api/llm-agent/sessions/{}/tool-calls/{}", llm_session.id, failed_tool_call.id);
    let req = test::TestRequest::put()
        .uri(&uri)
        .set_json(&fail_update)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    // 7. Create retry tool call with corrected code
    let retry_tool_call = TestDataGenerator::create_llm_agent_tool_call(
        &llm_session.id,
        "create_file",
        json!({
            "project_id": project.id,
            "path": "src/complex_module.rs",
            "content": "pub mod complex_module {\n    pub fn example_function() -> String {\n        \"Hello from complex module!\".to_string()\n    }\n\n    pub struct ComplexStruct {\n        pub value: i32,\n    }\n\n    impl ComplexStruct {\n        pub fn new(value: i32) -> Self {\n            Self { value }\n        }\n    }\n}",
            "is_directory": false
        })
    );
    test_app.db().create_llm_agent_tool_call(&retry_tool_call).unwrap();

    // Update retry with success
    let success_update = json!({
        "status": "completed",
        "execution_time_ms": 300,
        "response": {"result": "file_created_successfully"}
    });

    let uri = format!("/api/llm-agent/sessions/{}/tool-calls/{}", llm_session.id, retry_tool_call.id);
    let req = test::TestRequest::put()
        .uri(&uri)
        .set_json(&success_update)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    // 8. Verify error recovery
    let tool_calls = test_app.db().get_llm_agent_tool_calls(&llm_session.id).unwrap();
    assert_eq!(tool_calls.len(), 2);

    // One failed, one successful
    let failed_count = tool_calls.iter().filter(|tc| tc.status == "failed").count();
    let success_count = tool_calls.iter().filter(|tc| tc.status == "completed").count();

    assert_eq!(failed_count, 1);
    assert_eq!(success_count, 1);

    // Verify file was created with correct content
    let file_path = std::path::Path::new(&project.path).join("src").join("complex_module.rs");
    assert!(file_path.exists());

    let content = fs::read_to_string(&file_path).unwrap();
    assert!(content.contains("pub mod complex_module"));
    assert!(content.contains("example_function"));
    assert!(content.contains("ComplexStruct"));
    assert!(!content.contains("invalid rust code")); // Should not contain the failed attempt
}

#[actix_rt::test]
async fn test_performance_monitoring_workflow() {
    let test_app = TestApp::new().await;

    // 1. Create project
    let project = TestDataGenerator::create_project(Some("perf-monitoring-project"), Some("/tmp/perf-monitoring-project"));
    test_app.db().create_project(&project).unwrap();
    fs::create_dir_all(&project.path).unwrap();

    // 2. Create work session
    let work = TestDataGenerator::create_work(Some("Performance Monitoring Session"), Some(&project.id));
    test_app.db().create_work(&work).unwrap();

    // 3. Create LLM agent session
    let llm_session = TestDataGenerator::create_llm_agent_session(&work.id, "anthropic", "claude-3");
    test_app.db().create_llm_agent_session(&llm_session).unwrap();

    // 4. Simulate multiple tool calls with performance tracking
    let operations = vec![
        ("list_files", 50),
        ("read_file", 75),
        ("grep_search", 200),
        ("create_file", 100),
        ("update_file", 80),
        ("run_command", 500),
    ];

    let mut total_time = 0i64;

    for (tool_name, exec_time) in operations {
        let tool_call = TestDataGenerator::create_llm_agent_tool_call(
            &llm_session.id,
            tool_name,
            json!({"param": "performance_test"})
        );
        test_app.db().create_llm_agent_tool_call(&tool_call).unwrap();

        // Update with performance data
        let update_data = json!({
            "status": "completed",
            "execution_time_ms": exec_time,
            "response": {"result": "success"}
        });

        let uri = format!("/api/llm-agent/sessions/{}/tool-calls/{}", llm_session.id, tool_call.id);
        let req = test::TestRequest::put()
            .uri(&uri)
            .set_json(&update_data)
            .to_request();

        let resp = test::call_service(&test_app.service(), req).await;
        assert!(resp.status().is_success());

        total_time += exec_time;
    }

    // 5. Verify performance metrics
    let tool_calls = test_app.db().get_llm_agent_tool_calls(&llm_session.id).unwrap();
    assert_eq!(tool_calls.len(), 6);

    // All should be completed
    for tool_call in &tool_calls {
        assert_eq!(tool_call.status, "completed");
        assert!(tool_call.execution_time_ms.is_some());
        assert!(tool_call.completed_at.is_some());
    }

    // Calculate average execution time
    let exec_times: Vec<i64> = tool_calls.iter()
        .map(|tc| tc.execution_time_ms.unwrap())
        .collect();

    let sum: i64 = exec_times.iter().sum();
    let avg_time = sum / exec_times.len() as i64;

    // Expected average: (50 + 75 + 200 + 100 + 80 + 500) / 6 = 1005 / 6 = 167.5
    assert_eq!(avg_time, 167); // Integer division

    // Verify total time matches
    assert_eq!(sum, total_time);

    // Check that operations completed in expected order
    let fastest_operation = tool_calls.iter()
        .min_by_key(|tc| tc.execution_time_ms.unwrap())
        .unwrap();

    let slowest_operation = tool_calls.iter()
        .max_by_key(|tc| tc.execution_time_ms.unwrap())
        .unwrap();

    assert_eq!(fastest_operation.tool_name, "list_files");
    assert_eq!(fastest_operation.execution_time_ms, Some(50));

    assert_eq!(slowest_operation.tool_name, "run_command");
    assert_eq!(slowest_operation.execution_time_ms, Some(500));
}

#[actix_rt::test]
async fn test_concurrent_user_sessions_workflow() {
    let test_app = TestApp::new().await;

    // 1. Create shared project
    let project = TestDataGenerator::create_project(Some("concurrent-sessions-project"), Some("/tmp/concurrent-sessions-project"));
    test_app.db().create_project(&project).unwrap();
    fs::create_dir_all(&project.path).unwrap();

    // 2. Simulate concurrent user sessions
    let session_count = 5;
    let mut session_ids = Vec::new();

    for i in 0..session_count {
        // Create work session
        let work = TestDataGenerator::create_work(
            Some(&format!("Concurrent Session {}", i)),
            Some(&project.id)
        );
        test_app.db().create_work(&work).unwrap();

        // Create LLM agent session
        let llm_session = TestDataGenerator::create_llm_agent_session(&work.id, "openai", "gpt-4");
        test_app.db().create_llm_agent_session(&llm_session).unwrap();
        session_ids.push(llm_session.id);

        // Add some tool calls for each session
        for j in 0..3 {
            let tool_call = TestDataGenerator::create_llm_agent_tool_call(
                &llm_session.id,
                &format!("operation_{}", j),
                json!({"session": i, "operation": j})
            );
            test_app.db().create_llm_agent_tool_call(&tool_call).unwrap();
        }
    }

    // 3. Verify all sessions are properly isolated
    let all_works = test_app.db().get_all_works().unwrap();
    assert_eq!(all_works.len(), session_count);

    let all_llm_sessions = test_app.db().get_llm_agent_sessions_by_work(&project.id).unwrap();
    assert_eq!(all_llm_sessions.len(), session_count);

    // 4. Verify tool call isolation
    let mut total_tool_calls = 0;
    for session_id in &session_ids {
        let tool_calls = test_app.db().get_llm_agent_tool_calls(session_id).unwrap();
        assert_eq!(tool_calls.len(), 3); // 3 tool calls per session
        total_tool_calls += tool_calls.len();

        // All tool calls should belong to the correct session
        for tool_call in &tool_calls {
            assert_eq!(tool_call.session_id, *session_id);
        }
    }

    assert_eq!(total_tool_calls, session_count * 3);

    // 5. Verify no cross-contamination between sessions
    for i in 0..session_ids.len() {
        for j in (i + 1)..session_ids.len() {
            let session_i_calls = test_app.db().get_llm_agent_tool_calls(&session_ids[i]).unwrap();
            let session_j_calls = test_app.db().get_llm_agent_tool_calls(&session_ids[j]).unwrap();

            // No tool call IDs should overlap
            let session_i_ids: std::collections::HashSet<_> = session_i_calls.iter().map(|tc| tc.id).collect();
            let session_j_ids: std::collections::HashSet<_> = session_j_calls.iter().map(|tc| tc.id).collect();

            let intersection: std::collections::HashSet<_> = session_i_ids.intersection(&session_j_ids).collect();
            assert_eq!(intersection.len(), 0, "Tool call IDs should not overlap between sessions");
        }
    }
}

#[actix_rt::test]
async fn test_full_development_lifecycle_workflow() {
    let test_app = TestApp::new().await;

    let start_time = std::time::Instant::now();

    // Phase 1: Project Setup
    let project_temp_dir = test_app.test_config().projects_dir().join("lifecycle-project");
    let create_project_req = CreateProjectRequest {
        name: "lifecycle-project".to_string(),
        path: Some(project_temp_dir.to_string_lossy().to_string()),
        language: Some("rust".to_string()),
        framework: Some("actix-web".to_string()),
        template: None,
    };

    let req = test::TestRequest::post()
        .uri("/api/projects")
        .set_json(&create_project_req)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let project_id = body["project"]["id"].as_str().unwrap();

    // Phase 2: Development Session
    let work_req = CreateWorkRequest {
        title: "Full Development Lifecycle".to_string(),
        project_id: Some(project_id.to_string()),
        tool_name: Some("llm_agent".to_string()),
    };

    let req = test::TestRequest::post()
        .uri("/api/works")
        .set_json(&work_req)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let work_id = body["work"]["id"].as_str().unwrap();

    // Phase 3: AI Interaction
    let message_data = json!({
        "content": "Help me build a complete Rust web application with user management",
        "author_type": "user"
    });

    let msg_uri = format!("/api/works/{}/messages", work_id);
    let req = test::TestRequest::post()
        .uri(&msg_uri)
        .set_json(&message_data)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    // Phase 4: File Operations
    let create_file_req = FileCreateRequest {
        project_id: project_id.to_string(),
        path: "src/models.rs".to_string(),
        content: Some("pub struct User {\n    pub id: i32,\n    pub name: String,\n    pub email: String,\n}".to_string()),
        is_directory: false,
    };

    let req = test::TestRequest::post()
        .uri("/api/files/create")
        .set_json(&create_file_req)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    // Phase 5: Code Updates
    let update_file_req = FileUpdateRequest {
        project_id: project_id.to_string(),
        content: "pub struct User {\n    pub id: i32,\n    pub name: String,\n    pub email: String,\n}\n\nimpl User {\n    pub fn new(id: i32, name: String, email: String) -> Self {\n        Self { id, name, email }\n    }\n}".to_string(),
    };

    let req = test::TestRequest::put()
        .uri("/api/files/src/models.rs")
        .set_json(&update_file_req)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    // Phase 6: Verification
    let read_req = json!({
        "project_id": project_id,
        "path": "src/models.rs"
    });

    let req = test::TestRequest::post()
        .uri("/api/files/read")
        .set_json(&read_req)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let content = body["content"].as_str().unwrap();
    assert!(content.contains("impl User"));
    assert!(content.contains("new"));

    // Phase 7: List project contents
    let list_req = json!({
        "project_id": project_id,
        "path": "."
    });

    let req = test::TestRequest::post()
        .uri("/api/files/list")
        .set_json(&list_req)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let files = body["files"].as_array().unwrap();
    assert!(!files.is_empty());

    let duration = start_time.elapsed();
    println!("Full development lifecycle completed in {:?}", duration);

    // Should complete in reasonable time (less than 10 seconds for full workflow)
    assert!(duration < std::time::Duration::from_secs(10));
}