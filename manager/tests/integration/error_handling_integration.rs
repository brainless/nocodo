use actix_web::{test, web};
use serde_json::json;

use nocodo_manager::models::{
    CreateProjectRequest, CreateWorkRequest, CreateAiSessionRequest,
    CreateLlmAgentSessionRequest, FileCreateRequest, FileUpdateRequest,
};

use crate::common::{TestApp, TestDataGenerator};

#[actix_rt::test]
async fn test_integrated_error_handling_workflow() {
    let test_app = TestApp::new().await;

    // Test 1: Invalid project creation followed by valid creation
    let invalid_project_req = CreateProjectRequest {
        name: "".to_string(), // Invalid: empty name
        path: Some("/tmp/test".to_string()),
        language: Some("rust".to_string()),
        framework: Some("actix-web".to_string()),
        template: None,
    };

    let req = test::TestRequest::post()
        .uri("/api/projects")
        .set_json(&invalid_project_req)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert_eq!(resp.status(), 400);

    // Now create a valid project
    let valid_project_req = CreateProjectRequest {
        name: "recovery-project".to_string(),
        path: Some("/tmp/recovery-project".to_string()),
        language: Some("rust".to_string()),
        framework: Some("actix-web".to_string()),
        template: None,
    };

    let req = test::TestRequest::post()
        .uri("/api/projects")
        .set_json(&valid_project_req)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let project_id = body["project"]["id"].as_str().unwrap();

    // Test 2: Invalid work creation for valid project
    let invalid_work_req = CreateWorkRequest {
        title: "   \t\n  ".to_string(), // Invalid: whitespace only
        project_id: Some(project_id.to_string()),
        tool_name: Some("test-tool".to_string()),
    };

    let req = test::TestRequest::post()
        .uri("/api/works")
        .set_json(&invalid_work_req)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert_eq!(resp.status(), 400);

    // Create valid work
    let valid_work_req = CreateWorkRequest {
        title: "Recovery Work Session".to_string(),
        project_id: Some(project_id.to_string()),
        tool_name: Some("llm_agent".to_string()),
    };

    let req = test::TestRequest::post()
        .uri("/api/works")
        .set_json(&valid_work_req)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let work_id = body["work"]["id"].as_str().unwrap();

    // Test 3: Invalid AI session creation
    let invalid_ai_req = CreateAiSessionRequest {
        message_id: "non-existent-message".to_string(),
        tool_name: "test-tool".to_string(),
    };

    let req = test::TestRequest::post()
        .uri("/api/ai-sessions")
        .set_json(&invalid_ai_req)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert_eq!(resp.status(), 400);

    // Test 4: Invalid LLM agent session
    let invalid_llm_req = CreateLlmAgentSessionRequest {
        work_id: "non-existent-work".to_string(),
        provider: "openai".to_string(),
        model: "gpt-4".to_string(),
        system_prompt: None,
    };

    let req = test::TestRequest::post()
        .uri("/api/llm-agent/sessions")
        .set_json(&invalid_llm_req)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert_eq!(resp.status(), 400);

    // Test 5: File operations with invalid project
    let invalid_file_req = FileCreateRequest {
        project_id: "non-existent-project".to_string(),
        path: "test.txt".to_string(),
        content: Some("test content".to_string()),
        is_directory: false,
    };

    let req = test::TestRequest::post()
        .uri("/api/files/create")
        .set_json(&invalid_file_req)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert_eq!(resp.status(), 404);

    // Verify that valid operations still work after errors
    let projects_req = test::TestRequest::get().uri("/api/projects").to_request();
    let projects_resp = test::call_service(&test_app.service(), projects_req).await;
    assert!(projects_resp.status().is_success());

    let works_req = test::TestRequest::get().uri("/api/works").to_request();
    let works_resp = test::call_service(&test_app.service(), works_req).await;
    assert!(works_resp.status().is_success());
}

#[actix_rt::test]
async fn test_cascading_error_recovery() {
    let test_app = TestApp::new().await;

    // Create a valid project first
    let project = TestDataGenerator::create_project(Some("cascade-project"), Some("/tmp/cascade-project"));
    test_app.db().create_project(&project).unwrap();
    std::fs::create_dir_all(&project.path).unwrap();

    // Chain of operations with potential errors
    let operations = vec![
        // 1. Try to create work with invalid project (should fail)
        ("create_work_invalid_project", json!({
            "title": "Invalid Project Work",
            "project_id": "non-existent-project",
            "tool_name": "test-tool"
        }), "/api/works", "POST", 400),

        // 2. Create valid work (should succeed)
        ("create_work_valid", json!({
            "title": "Valid Work",
            "project_id": project.id,
            "tool_name": "llm_agent"
        }), "/api/works", "POST", 201),

        // 3. Try to add message to non-existent work (should fail)
        ("add_message_invalid_work", json!({
            "content": "Test message",
            "author_type": "user"
        }), "/api/works/non-existent-work/messages", "POST", 404),

        // 4. Get the work we just created (should succeed)
        ("get_work", serde_json::Value::Null, &format!("/api/works/{}", "work-id-from-previous"), "GET", 200),

        // 5. Try to create file in non-existent project (should fail)
        ("create_file_invalid_project", json!({
            "project_id": "non-existent-project",
            "path": "test.txt",
            "content": "test content",
            "is_directory": false
        }), "/api/files/create", "POST", 404),
    ];

    let mut work_id: Option<String> = None;

    for (operation_name, payload, endpoint, method, expected_status) in operations {
        println!("Testing operation: {}", operation_name);

        let req = match method {
            "GET" => {
                if operation_name == "get_work" {
                    // Special case: we need to get the work ID from the previous operation
                    let works = test_app.db().get_all_works().unwrap();
                    let work = works.last().unwrap();
                    work_id = Some(work.id.clone());
                    test::TestRequest::get().uri(&format!("/api/works/{}", work.id))
                } else {
                    test::TestRequest::get().uri(endpoint)
                }
            },
            "POST" => {
                let mut req = test::TestRequest::post().uri(endpoint);
                if !payload.is_null() {
                    req = req.set_json(&payload);
                }
                req
            },
            _ => panic!("Unsupported method: {}", method),
        }.to_request();

        let resp = test::call_service(&test_app.service(), req).await;
        assert_eq!(resp.status(), expected_status, "Operation {} failed with unexpected status", operation_name);

        // If this was a successful work creation, store the work ID
        if operation_name == "create_work_valid" && resp.status().is_success() {
            let body: serde_json::Value = test::read_body_json(resp).await;
            work_id = Some(body["work"]["id"].as_str().unwrap().to_string());
        }
    }

    // Verify final state
    assert!(work_id.is_some(), "Should have created a valid work session");

    let works = test_app.db().get_all_works().unwrap();
    assert_eq!(works.len(), 1);
    assert_eq!(works[0].id, work_id.unwrap());
}

#[actix_rt::test]
async fn test_concurrent_error_handling() {
    let test_app = TestApp::new().await;

    let concurrent_operations = 10;
    let mut handles = Vec::new();

    // Launch concurrent operations, some valid, some invalid
    for i in 0..concurrent_operations {
        let service = test_app.service().clone();

        let handle = tokio::spawn(async move {
            let operation_result = if i % 2 == 0 {
                // Even operations: valid project creation
                let project_req = CreateProjectRequest {
                    name: format!("concurrent-project-{}", i),
                    path: Some(format!("/tmp/concurrent-project-{}", i)),
                    language: Some("rust".to_string()),
                    framework: Some("actix-web".to_string()),
                    template: None,
                };

                let req = test::TestRequest::post()
                    .uri("/api/projects")
                    .set_json(&project_req)
                    .to_request();

                let resp = test::call_service(&service, req).await;
                (i, resp.status().is_success(), "create_project".to_string())
            } else {
                // Odd operations: invalid project creation
                let invalid_project_req = CreateProjectRequest {
                    name: "".to_string(), // Invalid: empty name
                    path: Some(format!("/tmp/invalid-project-{}", i)),
                    language: Some("rust".to_string()),
                    framework: Some("actix-web".to_string()),
                    template: None,
                };

                let req = test::TestRequest::post()
                    .uri("/api/projects")
                    .set_json(&invalid_project_req)
                    .to_request();

                let resp = test::call_service(&service, req).await;
                (i, resp.status() == 400, "create_project_invalid".to_string())
            };

            operation_result
        });

        handles.push(handle);
    }

    // Collect results
    let mut valid_operations = 0;
    let mut invalid_operations = 0;

    for handle in handles {
        let (operation_id, success, operation_type) = handle.await.unwrap();

        match operation_type.as_str() {
            "create_project" => {
                if success {
                    valid_operations += 1;
                } else {
                    panic!("Valid operation {} should have succeeded", operation_id);
                }
            },
            "create_project_invalid" => {
                if success {
                    invalid_operations += 1;
                } else {
                    panic!("Invalid operation {} should have returned 400", operation_id);
                }
            },
            _ => panic!("Unknown operation type"),
        }
    }

    // Verify results
    assert_eq!(valid_operations, concurrent_operations / 2, "Should have {} valid operations", concurrent_operations / 2);
    assert_eq!(invalid_operations, concurrent_operations / 2, "Should have {} invalid operations", concurrent_operations / 2);

    // Verify database state
    let projects = test_app.db().get_all_projects().unwrap();
    assert_eq!(projects.len(), valid_operations as usize, "Should have created {} projects", valid_operations);
}

#[actix_rt::test]
async fn test_error_boundary_isolation() {
    let test_app = TestApp::new().await;

    // Create multiple valid entities first
    let project = TestDataGenerator::create_project(Some("boundary-project"), Some("/tmp/boundary-project"));
    test_app.db().create_project(&project).unwrap();

    let work = TestDataGenerator::create_work(Some("Boundary Work"), Some(&project.id));
    test_app.db().create_work(&work).unwrap();

    // Test that errors in one operation don't affect others
    let test_cases = vec![
        // Invalid operations that should fail but not affect system state
        ("invalid_project_name", || async {
            let req = CreateProjectRequest {
                name: "".to_string(),
                path: Some("/tmp/invalid".to_string()),
                language: Some("rust".to_string()),
                framework: Some("actix-web".to_string()),
                template: None,
            };

            let test_req = test::TestRequest::post()
                .uri("/api/projects")
                .set_json(&req)
                .to_request();

            test::call_service(&test_app.service(), test_req).await
        }),

        ("invalid_work_title", || async {
            let req = CreateWorkRequest {
                title: "   \t\n  ".to_string(),
                project_id: Some(project.id.clone()),
                tool_name: Some("test-tool".to_string()),
            };

            let test_req = test::TestRequest::post()
                .uri("/api/works")
                .set_json(&req)
                .to_request();

            test::call_service(&test_app.service(), test_req).await
        }),

        ("non_existent_work", || async {
            let test_req = test::TestRequest::get()
                .uri("/api/works/non-existent-id")
                .to_request();

            test::call_service(&test_app.service(), test_req).await
        }),

        ("invalid_file_operation", || async {
            let req = FileCreateRequest {
                project_id: "non-existent-project".to_string(),
                path: "test.txt".to_string(),
                content: Some("test".to_string()),
                is_directory: false,
            };

            let test_req = test::TestRequest::post()
                .uri("/api/files/create")
                .set_json(&req)
                .to_request();

            test::call_service(&test_app.service(), test_req).await
        }),
    ];

    // Execute all error cases
    for (test_name, operation) in test_cases {
        println!("Testing error boundary: {}", test_name);
        let resp = operation().await;

        // These should all fail with appropriate error codes
        assert!(!resp.status().is_success(),
                "Error case '{}' should have failed but succeeded with status {}",
                test_name, resp.status());
    }

    // Verify that the system state is unchanged after all errors
    let projects_after = test_app.db().get_all_projects().unwrap();
    let works_after = test_app.db().get_all_works().unwrap();

    assert_eq!(projects_after.len(), 1, "Project count should remain unchanged after errors");
    assert_eq!(works_after.len(), 1, "Work count should remain unchanged after errors");
    assert_eq!(projects_after[0].id, project.id, "Original project should still exist");
    assert_eq!(works_after[0].id, work.id, "Original work should still exist");

    // Verify that valid operations still work
    let health_req = test::TestRequest::get().uri("/api/health").to_request();
    let health_resp = test::call_service(&test_app.service(), health_req).await;
    assert!(health_resp.status().is_success());

    let projects_req = test::TestRequest::get().uri("/api/projects").to_request();
    let projects_resp = test::call_service(&test_app.service(), projects_req).await;
    assert!(projects_resp.status().is_success());
}

#[actix_rt::test]
async fn test_error_message_consistency() {
    let test_app = TestApp::new().await;

    let error_cases = vec![
        // (endpoint, method, payload, expected_error_field, expected_status)
        ("/api/projects", "POST", json!({"name": "", "path": "/tmp/test"}), "invalid_request", 400),
        ("/api/projects/non-existent", "GET", serde_json::Value::Null, "project_not_found", 404),
        ("/api/works", "POST", json!({"title": "", "project_id": "non-existent"}), "invalid_request", 400),
        ("/api/works/non-existent", "GET", serde_json::Value::Null, "work_not_found", 404),
        ("/api/files/create", "POST", json!({"project_id": "non-existent", "path": "test.txt"}), "project_not_found", 404),
        ("/api/files/read", "POST", json!({"project_id": "non-existent", "path": "test.txt"}), "project_not_found", 404),
    ];

    for (endpoint, method, payload, expected_error, expected_status) in error_cases {
        let req = match method {
            "GET" => test::TestRequest::get().uri(endpoint),
            "POST" => {
                let mut req = test::TestRequest::post().uri(endpoint);
                if !payload.is_null() {
                    req = req.set_json(&payload);
                }
                req
            },
            _ => panic!("Unsupported method"),
        }.to_request();

        let resp = test::call_service(&test_app.service(), req).await;
        assert_eq!(resp.status(), expected_status,
                  "Expected status {} for {}, got {}", expected_status, endpoint, resp.status());

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["error"], expected_error,
                  "Expected error '{}' for {}, got '{}'", expected_error, endpoint, body["error"]);

        // Verify error response structure
        assert!(body["error"].is_string(), "Error field should be a string");
        assert!(body["message"].is_string() || body["message"].is_null(),
               "Message field should be a string or null");
    }
}

#[actix_rt::test]
async fn test_resource_cleanup_on_errors() {
    let test_app = TestApp::new().await;

    // Test that failed operations don't leave partial state
    let initial_projects = test_app.db().get_all_projects().unwrap().len();
    let initial_works = test_app.db().get_all_works().unwrap().len();

    // Attempt to create a project with invalid template (should fail)
    let invalid_template_req = CreateProjectRequest {
        name: "cleanup-test-project".to_string(),
        path: Some("/tmp/cleanup-test-project".to_string()),
        description: None,
        parent_id: None,
        template: Some("non-existent-template".to_string()),
    };

    let req = test::TestRequest::post()
        .uri("/api/projects")
        .set_json(&invalid_template_req)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert_eq!(resp.status(), 400);

    // Verify no partial state was created
    let final_projects = test_app.db().get_all_projects().unwrap().len();
    let final_works = test_app.db().get_all_works().unwrap().len();

    assert_eq!(final_projects, initial_projects, "No projects should be created on error");
    assert_eq!(final_works, initial_works, "No works should be created on error");

    // Verify file system is clean
    let project_path = std::path::Path::new("/tmp/cleanup-test-project");
    assert!(!project_path.exists(), "Project directory should not exist after failed creation");

    // Now create a valid project to ensure system still works
    let valid_req = CreateProjectRequest {
        name: "cleanup-valid-project".to_string(),
        path: Some("/tmp/cleanup-valid-project".to_string()),
        language: Some("rust".to_string()),
        framework: Some("actix-web".to_string()),
        template: None,
    };

    let req = test::TestRequest::post()
        .uri("/api/projects")
        .set_json(&valid_req)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    // Verify valid operation worked
    let final_projects_after_success = test_app.db().get_all_projects().unwrap().len();
    assert_eq!(final_projects_after_success, initial_projects + 1, "Valid project should be created");
}

#[actix_rt::test]
async fn test_error_rate_monitoring() {
    let test_app = TestApp::new().await;

    let mut error_count = 0;
    let mut success_count = 0;
    let total_operations = 100;

    // Mix of valid and invalid operations
    for i in 0..total_operations {
        let is_error_case = i % 3 == 0; // Every 3rd operation is an error

        let (req, should_succeed) = if is_error_case {
            // Error case: invalid project name
            let error_req = CreateProjectRequest {
                name: "".to_string(),
                path: Some(format!("/tmp/error-test-{}", i)),
                language: Some("rust".to_string()),
                framework: Some("actix-web".to_string()),
                template: None,
            };

            let test_req = test::TestRequest::post()
                .uri("/api/projects")
                .set_json(&error_req)
                .to_request();

            (test_req, false)
        } else {
            // Success case: valid project
            let success_req = CreateProjectRequest {
                name: format!("success-project-{}", i),
                path: Some(format!("/tmp/success-project-{}", i)),
                language: Some("rust".to_string()),
                framework: Some("actix-web".to_string()),
                template: None,
            };

            let test_req = test::TestRequest::post()
                .uri("/api/projects")
                .set_json(&success_req)
                .to_request();

            (test_req, true)
        };

        let resp = test::call_service(&test_app.service(), req).await;

        if should_succeed {
            assert!(resp.status().is_success(), "Valid operation {} should succeed", i);
            success_count += 1;
        } else {
            assert_eq!(resp.status(), 400, "Invalid operation {} should return 400", i);
            error_count += 1;
        }
    }

    // Verify error rate
    let expected_errors = total_operations / 3; // Every 3rd operation
    let expected_successes = total_operations - expected_errors;

    assert_eq!(error_count, expected_errors, "Should have {} errors", expected_errors);
    assert_eq!(success_count, expected_successes, "Should have {} successes", expected_successes);

    let error_rate = (error_count as f64) / (total_operations as f64);
    println!("Error rate: {:.2}% ({} errors out of {} operations)", error_rate * 100.0, error_count, total_operations);

    // Error rate should be approximately 33%
    assert!((error_rate - 0.33).abs() < 0.1, "Error rate should be approximately 33%, got {:.2}%", error_rate * 100.0);

    // Verify final state
    let projects = test_app.db().get_all_projects().unwrap();
    assert_eq!(projects.len(), success_count as usize, "Should have created {} projects", success_count);
}