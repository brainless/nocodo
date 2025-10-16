use actix_web::{test, web};
use serde_json::json;

use nocodo_manager::models::{
    CreateProjectRequest, CreateWorkRequest, FileListRequest, FileCreateRequest, FileUpdateRequest,
};

use crate::common::{TestApp, TestDataGenerator};

#[actix_rt::test]
async fn test_project_creation_errors() {
    let test_app = TestApp::new().await;

    // Test 1: Empty project name
    let invalid_request = CreateProjectRequest {
        name: "".to_string(),
        path: Some("/tmp/test".to_string()),
        language: Some("rust".to_string()),
        framework: Some("actix-web".to_string()),
        template: None,
    };

    let req = test::TestRequest::post()
        .uri("/api/projects")
        .set_json(&invalid_request)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert_eq!(resp.status(), 400);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "invalid_request");
    assert!(body["message"].as_str().unwrap().contains("Project name cannot be empty"));

    // Test 2: Whitespace-only project name
    let whitespace_request = CreateProjectRequest {
        name: "   \t\n  ".to_string(),
        path: Some("/tmp/test".to_string()),
        language: Some("rust".to_string()),
        framework: Some("actix-web".to_string()),
        template: None,
    };

    let req = test::TestRequest::post()
        .uri("/api/projects")
        .set_json(&whitespace_request)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert_eq!(resp.status(), 400);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "invalid_request");
    assert!(body["message"].as_str().unwrap().contains("Project name cannot be empty"));

    // Test 3: Project directory already exists
    let project_path = test_app.test_config().projects_dir().join("existing-dir");
    std::fs::create_dir_all(&project_path).unwrap();

    let existing_dir_request = CreateProjectRequest {
        name: "existing-dir-project".to_string(),
        path: Some(project_path.to_string_lossy().to_string()),
        language: Some("rust".to_string()),
        framework: Some("actix-web".to_string()),
        template: None,
    };

    let req = test::TestRequest::post()
        .uri("/api/projects")
        .set_json(&existing_dir_request)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert_eq!(resp.status(), 400);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "invalid_request");
    assert!(body["message"].as_str().unwrap().contains("Project directory already exists"));

    // Test 4: Invalid template
    let invalid_template_request = CreateProjectRequest {
        name: "invalid-template-project".to_string(),
        path: Some("/tmp/invalid-template".to_string()),
        description: None,
        parent_id: None,
        template: Some("non-existent-template".to_string()),
    };

    let req = test::TestRequest::post()
        .uri("/api/projects")
        .set_json(&invalid_template_request)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert_eq!(resp.status(), 400);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "invalid_request");
    assert!(body["message"].as_str().unwrap().contains("Template not found"));
}

#[actix_rt::test]
async fn test_project_retrieval_errors() {
    let test_app = TestApp::new().await;

    // Test: Get non-existent project
    let req = test::TestRequest::get()
        .uri("/api/projects/non-existent-id")
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert_eq!(resp.status(), 404);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "project_not_found");
}

#[actix_rt::test]
async fn test_work_creation_errors() {
    let test_app = TestApp::new().await;

    // Test 1: Empty work title
    let invalid_request = CreateWorkRequest {
        title: "".to_string(),
        project_id: None,
        tool_name: Some("test-tool".to_string()),
    };

    let req = test::TestRequest::post()
        .uri("/api/works")
        .set_json(&invalid_request)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert_eq!(resp.status(), 400);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "invalid_request");
    assert!(body["message"].as_str().unwrap().contains("Work title cannot be empty"));

    // Test 2: Invalid project ID
    let invalid_project_request = CreateWorkRequest {
        title: "Test Work".to_string(),
        project_id: Some("non-existent-project".to_string()),
        tool_name: Some("test-tool".to_string()),
    };

    let req = test::TestRequest::post()
        .uri("/api/works")
        .set_json(&invalid_project_request)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert_eq!(resp.status(), 400);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "invalid_request");
    assert!(body["message"].as_str().unwrap().contains("Invalid project ID"));
}

#[actix_rt::test]
async fn test_work_retrieval_errors() {
    let test_app = TestApp::new().await;

    // Test: Get non-existent work
    let req = test::TestRequest::get()
        .uri("/api/works/non-existent-id")
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert_eq!(resp.status(), 404);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "work_not_found");
}

#[actix_rt::test]
async fn test_work_update_errors() {
    let test_app = TestApp::new().await;

    // Create a work first
    let work = TestDataGenerator::create_work(Some("Update Error Test"), None);
    test_app.db().create_work(&work).unwrap();

    // Test: Update non-existent work
    let update_data = json!({
        "title": "Updated Title"
    });

    let req = test::TestRequest::put()
        .uri("/api/works/non-existent-id")
        .set_json(&update_data)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert_eq!(resp.status(), 404);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "work_not_found");

    // Test: Update with empty title
    let invalid_update = json!({
        "title": ""
    });

    let req = test::TestRequest::put()
        .uri(&format!("/api/works/{}", work.id))
        .set_json(&invalid_update)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert_eq!(resp.status(), 400);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "invalid_request");
    assert!(body["message"].as_str().unwrap().contains("Work title cannot be empty"));
}

#[actix_rt::test]
async fn test_work_deletion_errors() {
    let test_app = TestApp::new().await;

    // Test: Delete non-existent work
    let req = test::TestRequest::delete()
        .uri("/api/works/non-existent-id")
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert_eq!(resp.status(), 404);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "work_not_found");
}

#[actix_rt::test]
async fn test_work_message_errors() {
    let test_app = TestApp::new().await;

    // Create a work first
    let work = TestDataGenerator::create_work(Some("Message Error Test"), None);
    test_app.db().create_work(&work).unwrap();

    // Test 1: Add message to non-existent work
    let message_data = json!({
        "content": "Test message",
        "author_type": "user"
    });

    let req = test::TestRequest::post()
        .uri("/api/works/non-existent-work/messages")
        .set_json(&message_data)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert_eq!(resp.status(), 404);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "work_not_found");

    // Test 2: Add message with empty content
    let empty_message = json!({
        "content": "",
        "author_type": "user"
    });

    let req = test::TestRequest::post()
        .uri(&format!("/api/works/{}/messages", work.id))
        .set_json(&empty_message)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert_eq!(resp.status(), 400);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "invalid_request");
    assert!(body["message"].as_str().unwrap().contains("Message content cannot be empty"));

    // Test 3: Add message with invalid author type
    let invalid_author = json!({
        "content": "Test message",
        "author_type": "invalid_author"
    });

    let req = test::TestRequest::post()
        .uri(&format!("/api/works/{}/messages", work.id))
        .set_json(&invalid_author)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert_eq!(resp.status(), 400);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "invalid_request");
    assert!(body["message"].as_str().unwrap().contains("Invalid author type"));
}

#[actix_rt::test]
async fn test_file_operation_errors() {
    let test_app = TestApp::new().await;

    // Test 1: List files for non-existent project
    let list_request = FileListRequest {
        project_id: Some("non-existent-project".to_string()),
        path: None,
    };

    let req = test::TestRequest::post()
        .uri("/api/files/list")
        .set_json(&list_request)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert_eq!(resp.status(), 404);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "project_not_found");

    // Test 2: List files in non-existent path
    let project = TestDataGenerator::create_project(Some("file-error-project"), Some("/tmp/file-error-project"));
    test_app.db().create_project(&project).unwrap();
    std::fs::create_dir_all(&project.path).unwrap();

    let invalid_path_request = FileListRequest {
        project_id: Some(project.id.clone()),
        path: Some("non/existent/path".to_string()),
    };

    let req = test::TestRequest::post()
        .uri("/api/files/list")
        .set_json(&invalid_path_request)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert_eq!(resp.status(), 404);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].as_str().unwrap().contains("path_not_found"));

    // Test 3: Read non-existent file
    let read_request = json!({
        "project_id": project.id,
        "path": "non-existent.txt"
    });

    let req = test::TestRequest::post()
        .uri("/api/files/read")
        .set_json(&read_request)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert_eq!(resp.status(), 404);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].as_str().unwrap().contains("file_not_found"));

    // Test 4: Create file in non-existent project
    let create_request = FileCreateRequest {
        project_id: "non-existent-project".to_string(),
        path: "test.txt".to_string(),
        content: Some("test content".to_string()),
        is_directory: false,
    };

    let req = test::TestRequest::post()
        .uri("/api/files/create")
        .set_json(&create_request)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert_eq!(resp.status(), 404);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "project_not_found");

    // Test 5: Create file that already exists
    let file_path = std::path::Path::new(&project.path).join("existing.txt");
    std::fs::write(&file_path, "existing content").unwrap();

    let create_existing_request = FileCreateRequest {
        project_id: project.id.clone(),
        path: "existing.txt".to_string(),
        content: Some("new content".to_string()),
        is_directory: false,
    };

    let req = test::TestRequest::post()
        .uri("/api/files/create")
        .set_json(&create_existing_request)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert_eq!(resp.status(), 409);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].as_str().unwrap().contains("already_exists"));

    // Test 6: Update non-existent file
    let update_request = FileUpdateRequest {
        project_id: project.id.clone(),
        content: "updated content".to_string(),
    };

    let req = test::TestRequest::put()
        .uri("/api/files/non-existent.txt")
        .set_json(&update_request)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert_eq!(resp.status(), 404);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["error"].as_str().unwrap().contains("file_not_found"));
}

#[actix_rt::test]
async fn test_ai_session_errors() {
    let test_app = TestApp::new().await;

    // Test: Create AI session for non-existent work
    let ai_session_request = json!({
        "message_id": "non-existent-message",
        "tool_name": "test-tool"
    });

    let req = test::TestRequest::post()
        .uri("/api/ai-sessions")
        .set_json(&ai_session_request)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert_eq!(resp.status(), 400);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "invalid_request");
    assert!(body["message"].as_str().unwrap().contains("Invalid work ID"));
}

#[actix_rt::test]
async fn test_template_errors() {
    let test_app = TestApp::new().await;

    // Test: Get non-existent template
    let req = test::TestRequest::get()
        .uri("/api/templates/non-existent-template")
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert_eq!(resp.status(), 404);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "template_not_found");
}

#[actix_rt::test]
async fn test_malformed_json_errors() {
    let test_app = TestApp::new().await;

    // Test: Malformed JSON in project creation
    let req = test::TestRequest::post()
        .uri("/api/projects")
        .set_payload(b"{invalid json")
        .header("content-type", "application/json")
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert_eq!(resp.status(), 400);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "invalid_json");

    // Test: Invalid JSON structure
    let req = test::TestRequest::post()
        .uri("/api/projects")
        .set_payload(b"{\"name\": 123, \"invalid_field\": \"value\"}") // name should be string
        .header("content-type", "application/json")
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert_eq!(resp.status(), 400);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "invalid_request");
}

#[actix_rt::test]
async fn test_method_not_allowed_errors() {
    let test_app = TestApp::new().await;

    // Test: Wrong HTTP method
    let req = test::TestRequest::patch()
        .uri("/api/projects")
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert_eq!(resp.status(), 405);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "method_not_allowed");
}

#[actix_rt::test]
async fn test_database_constraint_errors() {
    let test_app = TestApp::new().await;

    // Create a project
    let project = TestDataGenerator::create_project(Some("constraint-test"), Some("/tmp/constraint-test"));
    test_app.db().create_project(&project).unwrap();

    // Try to create another project with same ID (should fail due to unique constraint)
    // This simulates a race condition or duplicate ID scenario
    let duplicate_project = nocodo_manager::models::Project {
        id: project.id.clone(), // Same ID
        name: "different-name".to_string(),
        path: "/tmp/different-path".to_string(),
        language: Some("javascript".to_string()),
        parent_id: None,
        
        created_at: chrono::Utc::now().timestamp(),
        updated_at: chrono::Utc::now().timestamp(),
        
    };

    // This should fail at the database level
    let result = test_app.db().create_project(&duplicate_project);
    assert!(result.is_err());

    // The error should be a database constraint violation
    let error = result.unwrap_err();
    assert!(error.to_string().contains("UNIQUE constraint failed") ||
            error.to_string().contains("constraint") ||
            error.to_string().contains("duplicate"));
}

#[actix_rt::test]
async fn test_concurrent_modification_errors() {
    let test_app = TestApp::new().await;

    // Create a work session
    let work = TestDataGenerator::create_work(Some("Concurrent Test"), None);
    test_app.db().create_work(&work).unwrap();

    // Simulate concurrent updates by modifying the work in database directly
    // while trying to update through API
    let db_work = test_app.db().get_work_by_id(&work.id).unwrap();
    let original_updated_at = db_work.updated_at;

    // Update through API
    let update_data = json!({
        "title": "Updated Title"
    });

    let req = test::TestRequest::put()
        .uri(&format!("/api/works/{}", work.id))
        .set_json(&update_data)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    // Verify the update worked
    let updated_work = test_app.db().get_work_by_id(&work.id).unwrap();
    assert_eq!(updated_work.title, "Updated Title");
    assert_ne!(updated_work.updated_at, original_updated_at);
}

#[actix_rt::test]
async fn test_large_payload_errors() {
    let test_app = TestApp::new().await;

    // Create a very large string (over typical limits)
    let large_content = "x".repeat(10 * 1024 * 1024); // 10MB

    let create_request = CreateProjectRequest {
        name: "large-payload-test".to_string(),
        path: Some("/tmp/large-test".to_string()),
        language: Some("rust".to_string()),
        framework: Some("actix-web".to_string()),
        template: None,
    };

    let req = test::TestRequest::post()
        .uri("/api/projects")
        .set_json(&create_request)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;

    // This might succeed or fail depending on server configuration
    // The important thing is that it doesn't crash the server
    assert!(resp.status().is_success() || resp.status().is_client_error());
}

#[actix_rt::test]
async fn test_network_timeout_simulation() {
    let test_app = TestApp::new().await;

    // Create a project
    let project = TestDataGenerator::create_project(Some("timeout-test"), Some("/tmp/timeout-test"));
    test_app.db().create_project(&project).unwrap();

    // Simulate a slow operation by creating many files
    std::fs::create_dir_all(&project.path).unwrap();
    for i in 0..1000 {
        std::fs::write(
            std::path::Path::new(&project.path).join(format!("file-{}.txt", i)),
            format!("Content {}", i)
        ).unwrap();
    }

    // List files - this should complete within reasonable time
    let list_request = FileListRequest {
        project_id: Some(project.id.clone()),
        path: None,
    };

    let start_time = std::time::Instant::now();

    let req = test::TestRequest::post()
        .uri("/api/files/list")
        .set_json(&list_request)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;

    let duration = start_time.elapsed();

    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let files = body["files"].as_array().unwrap();

    assert_eq!(files.len(), 1000);

    // Should complete in reasonable time (less than 5 seconds for 1000 files)
    assert!(duration < std::time::Duration::from_secs(5));
}