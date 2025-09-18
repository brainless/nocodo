use actix_web::{test, web};
use serde_json::json;

use nocodo_manager::models::{CreateWorkRequest, Work, WorkMessage, MessageAuthorType, MessageContentType};

use crate::common::{TestApp, TestDataGenerator};

#[actix_rt::test]
async fn test_get_works_empty() {
    let test_app = TestApp::new().await;

    let req = test::TestRequest::get().uri("/api/works").to_request();
    let resp = test::call_service(&test_app.service(), req).await;

    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let works = body["works"].as_array().unwrap();
    assert_eq!(works.len(), 0);
}

#[actix_rt::test]
async fn test_create_work_basic() {
    let test_app = TestApp::new().await;

    let create_request = CreateWorkRequest {
        title: "Basic Work Session".to_string(),
        project_id: None,
        tool_name: Some("test-tool".to_string()),
    };

    let req = test::TestRequest::post()
        .uri("/api/works")
        .set_json(&create_request)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());
    assert_eq!(resp.status(), 201);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let work = &body["work"];

    assert_eq!(work["title"], "Basic Work Session");
    assert_eq!(work["status"], "active");
    assert_eq!(work["tool_name"], "test-tool");
    assert!(work["project_id"].is_null());
    assert!(work["id"].as_str().is_some());
    assert!(work["created_at"].as_i64().is_some());
    assert!(work["updated_at"].as_i64().is_some());

    // Verify work was created in database
    let works = test_app.db().get_all_works().unwrap();
    assert_eq!(works.len(), 1);
    assert_eq!(works[0].title, "Basic Work Session");
}

#[actix_rt::test]
async fn test_create_work_with_project() {
    let test_app = TestApp::new().await;

    // Create a project first
    let project = TestDataGenerator::create_project(Some("work-project"), Some("/tmp/work-project"));
    test_app.db().create_project(&project).unwrap();

    let create_request = CreateWorkRequest {
        title: "Work with Project".to_string(),
        project_id: Some(project.id.clone()),
        tool_name: Some("llm-agent".to_string()),
    };

    let req = test::TestRequest::post()
        .uri("/api/works")
        .set_json(&create_request)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let work = &body["work"];

    assert_eq!(work["title"], "Work with Project");
    assert_eq!(work["project_id"], project.id);
    assert_eq!(work["tool_name"], "llm-agent");
    assert_eq!(work["status"], "active");

    // Verify work-project relationship in database
    let db_work = test_app.db().get_work_by_id(work["id"].as_str().unwrap()).unwrap();
    assert_eq!(db_work.project_id, Some(project.id));
}

#[actix_rt::test]
async fn test_get_work_by_id() {
    let test_app = TestApp::new().await;

    // Create a work session
    let work = TestDataGenerator::create_work(Some("Get Work Test"), None);
    test_app.db().create_work(&work).unwrap();

    // Get the work by ID
    let uri = format!("/api/works/{}", work.id);
    let req = test::TestRequest::get().uri(&uri).to_request();
    let resp = test::call_service(&test_app.service(), req).await;

    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let retrieved_work = &body["work"];

    assert_eq!(retrieved_work["id"], work.id);
    assert_eq!(retrieved_work["title"], "Get Work Test");
    assert_eq!(retrieved_work["status"], "active");
}

#[actix_rt::test]
async fn test_get_work_not_found() {
    let test_app = TestApp::new().await;

    let req = test::TestRequest::get()
        .uri("/api/works/non-existent-id")
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert_eq!(resp.status(), 404);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "work_not_found");
}

#[actix_rt::test]
async fn test_get_works_after_creation() {
    let test_app = TestApp::new().await;

    // Create multiple work sessions
    let work1 = TestDataGenerator::create_work(Some("Work 1"), None);
    let work2 = TestDataGenerator::create_work(Some("Work 2"), None);
    let work3 = TestDataGenerator::create_work(Some("Work 3"), None);

    test_app.db().create_work(&work1).unwrap();
    test_app.db().create_work(&work2).unwrap();
    test_app.db().create_work(&work3).unwrap();

    // Get all works
    let req = test::TestRequest::get().uri("/api/works").to_request();
    let resp = test::call_service(&test_app.service(), req).await;

    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let works = body["works"].as_array().unwrap();

    assert_eq!(works.len(), 3);

    // Verify works are returned in correct order (newest first)
    let titles: Vec<&str> = works
        .iter()
        .map(|w| w["title"].as_str().unwrap())
        .collect();

    assert!(titles.contains(&"Work 1"));
    assert!(titles.contains(&"Work 2"));
    assert!(titles.contains(&"Work 3"));
}

#[actix_rt::test]
async fn test_update_work() {
    let test_app = TestApp::new().await;

    // Create a work session
    let work = TestDataGenerator::create_work(Some("Update Test"), None);
    test_app.db().create_work(&work).unwrap();

    // Update the work
    let update_data = json!({
        "title": "Updated Work Title",
        "status": "completed"
    });

    let uri = format!("/api/works/{}", work.id);
    let req = test::TestRequest::put()
        .uri(&uri)
        .set_json(&update_data)
        .to_request();

    let resp = test::call_service(&test_app.service(), req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let updated_work = &body["work"];

    assert_eq!(updated_work["id"], work.id);
    assert_eq!(updated_work["title"], "Updated Work Title");
    assert_eq!(updated_work["status"], "completed");

    // Verify update in database
    let db_work = test_app.db().get_work_by_id(&work.id).unwrap();
    assert_eq!(db_work.title, "Updated Work Title");
    assert_eq!(db_work.status, "completed");
}

#[actix_rt::test]
async fn test_delete_work() {
    let test_app = TestApp::new().await;

    // Create a work session
    let work = TestDataGenerator::create_work(Some("Delete Test"), None);
    test_app.db().create_work(&work).unwrap();

    // Verify it exists
    let works_before = test_app.db().get_all_works().unwrap();
    assert_eq!(works_before.len(), 1);

    // Delete the work
    let uri = format!("/api/works/{}", work.id);
    let req = test::TestRequest::delete().uri(&uri).to_request();
    let resp = test::call_service(&test_app.service(), req).await;

    assert!(resp.status().is_success());

    // Verify it was deleted from database
    let works_after = test_app.db().get_all_works().unwrap();
    assert_eq!(works_after.len(), 0);

    // Try to get the deleted work
    let get_req = test::TestRequest::get().uri(&uri).to_request();
    let get_resp = test::call_service(&test_app.service(), get_req).await;
    assert_eq!(get_resp.status(), 404);
}

#[actix_rt::test]
async fn test_work_with_messages_workflow() {
    let test_app = TestApp::new().await;

    // 1. Create work session
    let create_work_req = CreateWorkRequest {
        title: "Message Workflow Test".to_string(),
        project_id: None,
        tool_name: Some("test-tool".to_string()),
    };

    let work_req = test::TestRequest::post()
        .uri("/api/works")
        .set_json(&create_work_req)
        .to_request();

    let work_resp = test::call_service(&test_app.service(), work_req).await;
    assert!(work_resp.status().is_success());

    let work_body: serde_json::Value = test::read_body_json(work_resp).await;
    let work_id = work_body["work"]["id"].as_str().unwrap();

    // 2. Add messages to work
    let message_data = json!({
        "content": "Hello, I need help with coding",
        "author_type": "user"
    });

    let msg_uri = format!("/api/works/{}/messages", work_id);
    let msg_req = test::TestRequest::post()
        .uri(&msg_uri)
        .set_json(&message_data)
        .to_request();

    let msg_resp = test::call_service(&test_app.service(), msg_req).await;
    assert!(msg_resp.status().is_success());

    // 3. Get work messages
    let get_msg_req = test::TestRequest::get().uri(&msg_uri).to_request();
    let get_msg_resp = test::call_service(&test_app.service(), get_msg_req).await;
    assert!(get_msg_resp.status().is_success());

    let msg_body: serde_json::Value = test::read_body_json(get_msg_resp).await;
    let messages = msg_body["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);

    let message = &messages[0];
    assert_eq!(message["content"], "Hello, I need help with coding");
    assert_eq!(message["author_type"], "user");
    assert_eq!(message["sequence_order"], 0);

    // 4. Add another message
    let ai_message_data = json!({
        "content": "I'll help you with your coding task",
        "author_type": "ai"
    });

    let ai_msg_req = test::TestRequest::post()
        .uri(&msg_uri)
        .set_json(&ai_message_data)
        .to_request();

    let ai_msg_resp = test::call_service(&test_app.service(), ai_msg_req).await;
    assert!(ai_msg_resp.status().is_success());

    // 5. Verify both messages
    let final_msg_req = test::TestRequest::get().uri(&msg_uri).to_request();
    let final_msg_resp = test::call_service(&test_app.service(), final_msg_req).await;
    assert!(final_msg_resp.status().is_success());

    let final_body: serde_json::Value = test::read_body_json(final_msg_resp).await;
    let final_messages = final_body["messages"].as_array().unwrap();
    assert_eq!(final_messages.len(), 2);

    assert_eq!(final_messages[0]["sequence_order"], 0);
    assert_eq!(final_messages[1]["sequence_order"], 1);
    assert_eq!(final_messages[0]["author_type"], "user");
    assert_eq!(final_messages[1]["author_type"], "ai");
}

#[actix_rt::test]
async fn test_work_message_sequence_ordering() {
    let test_app = TestApp::new().await;

    // Create work session
    let work = TestDataGenerator::create_work(Some("Sequence Test"), None);
    test_app.db().create_work(&work).unwrap();

    // Add messages with specific sequence
    let messages = vec![
        ("First message", 0),
        ("Second message", 1),
        ("Third message", 2),
    ];

    for (content, expected_seq) in messages {
        let message = TestDataGenerator::create_work_message(
            &work.id,
            content,
            MessageAuthorType::User,
            expected_seq,
        );
        test_app.db().create_work_message(&message).unwrap();
    }

    // Get messages and verify sequence
    let db_messages = test_app.db().get_work_messages(&work.id).unwrap();
    assert_eq!(db_messages.len(), 3);

    for (i, message) in db_messages.iter().enumerate() {
        assert_eq!(message.sequence_order, i as i32);
    }

    // Verify through API
    let uri = format!("/api/works/{}/messages", work.id);
    let req = test::TestRequest::get().uri(&uri).to_request();
    let resp = test::call_service(&test_app.service(), req).await;

    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let api_messages = body["messages"].as_array().unwrap();

    for (i, message) in api_messages.iter().enumerate() {
        assert_eq!(message["sequence_order"], i as i32);
    }
}

#[actix_rt::test]
async fn test_work_with_project_relationship() {
    let test_app = TestApp::new().await;

    // Create project
    let project = TestDataGenerator::create_project(Some("Work Project"), Some("/tmp/work-project"));
    test_app.db().create_project(&project).unwrap();

    // Create work associated with project
    let work = TestDataGenerator::create_work(Some("Project Work"), Some(&project.id));
    test_app.db().create_work(&work).unwrap();

    // Get work and verify project relationship
    let uri = format!("/api/works/{}", work.id);
    let req = test::TestRequest::get().uri(&uri).to_request();
    let resp = test::call_service(&test_app.service(), req).await;

    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let retrieved_work = &body["work"];

    assert_eq!(retrieved_work["project_id"], project.id);
    assert_eq!(retrieved_work["title"], "Project Work");

    // Verify relationship in database
    let db_work = test_app.db().get_work_by_id(&work.id).unwrap();
    assert_eq!(db_work.project_id, Some(project.id));
}

#[actix_rt::test]
async fn test_work_status_transitions() {
    let test_app = TestApp::new().await;

    // Create work
    let work = TestDataGenerator::create_work(Some("Status Test"), None);
    test_app.db().create_work(&work).unwrap();

    // Test status transitions
    let statuses = vec!["active", "running", "completed", "failed"];

    for status in statuses {
        let update_data = json!({
            "status": status
        });

        let uri = format!("/api/works/{}", work.id);
        let req = test::TestRequest::put()
            .uri(&uri)
            .set_json(&update_data)
            .to_request();

        let resp = test::call_service(&test_app.service(), req).await;
        assert!(resp.status().is_success());

        let body: serde_json::Value = test::read_body_json(resp).await;
        assert_eq!(body["work"]["status"], status);

        // Verify in database
        let db_work = test_app.db().get_work_by_id(&work.id).unwrap();
        assert_eq!(db_work.status, status);
    }
}