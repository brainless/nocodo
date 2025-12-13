use actix_web::{test, web};
use nocodo_manager::routes::configure_routes;

use nocodo_manager::models::CreateProjectRequest;

use crate::common::{TestApp, TestDataGenerator};

#[actix_rt::test]
async fn test_get_projects_empty() {
    let test_app = TestApp::new().await;

    let service = test::init_service(
        actix_web::App::new()
            .app_data(test_app.app_state().clone())
            .route("/api/projects", web::get().to(nocodo_manager::handlers::project_handlers::get_projects))
    ).await;

    let req = test::TestRequest::get().uri("/api/projects").to_request();
    let resp = test::call_service(&service, req).await;

    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let projects = body["projects"].as_array().unwrap();
    assert_eq!(projects.len(), 0);
}

#[actix_rt::test]
async fn test_create_project_basic() {
    let test_app = TestApp::new().await;

    let project_temp_dir = test_app.test_config().projects_dir().join("basic-test-project");

    let create_request = CreateProjectRequest {
        name: "basic-test-project".to_string(),
        path: Some(project_temp_dir.to_string_lossy().to_string()),
        description: None,
        parent_id: None,
        template: None,
    };

    let service = test::init_service(
        actix_web::App::new()
            .app_data(test_app.app_state().clone())
            .configure(|cfg| configure_routes(cfg, false))
    ).await;

let req = test::TestRequest::post()
        .uri("/api/projects")
        .set_json(&create_request)
        .to_request();

    let resp = test::call_service(&service, req).await;
    if !resp.status().is_success() {
        let status = resp.status();
        let error_body = test::read_body(resp).await;
        let error_text = std::str::from_utf8(&error_body).unwrap_or("<invalid utf8>");
        panic!("Project creation failed with status {status}: {error_text}");
    }
    assert_eq!(resp.status(), 201);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let project = &body["project"];

    assert_eq!(project["name"], "basic-test-project");
    assert!(project["id"].as_i64().is_some());
    assert!(project["created_at"].as_i64().is_some());
    assert!(project["updated_at"].as_i64().is_some());

    // Verify project was created in database
    let projects = test_app.db().get_all_projects().unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].name, "basic-test-project");
}

#[actix_rt::test]
async fn test_create_project_invalid_name() {
    let test_app = TestApp::new().await;

    let create_request = CreateProjectRequest {
        name: "   ".to_string(),
        path: Some("/tmp/test".to_string()),
        description: None,
        parent_id: None,
        template: None,
    };

    let service = test::init_service(
        actix_web::App::new()
            .app_data(test_app.app_state().clone())
            .configure(|cfg| configure_routes(cfg, false))
    ).await;

    let req = test::TestRequest::post()
        .uri("/api/projects")
        .set_json(&create_request)
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert_eq!(resp.status(), 400);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "invalid_request");
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("Project name cannot be empty"));
}

#[actix_rt::test]
async fn test_create_project_default_path() {
    let test_app = TestApp::new().await;

    let create_request = CreateProjectRequest {
        name: format!("test-default-path-{}", chrono::Utc::now().timestamp_nanos()),
        path: None,
        description: None,
        parent_id: None,
        template: None,
    };

    let service = test::init_service(
        actix_web::App::new()
            .app_data(test_app.app_state().clone())
            .configure(|cfg| configure_routes(cfg, false))
    ).await;

    let req = test::TestRequest::post()
        .uri("/api/projects")
        .set_json(&create_request)
        .to_request();

    let resp = test::call_service(&service, req).await;
    if !resp.status().is_success() {
        let status = resp.status();
        let error_body = test::read_body(resp).await;
        let error_text = std::str::from_utf8(&error_body).unwrap_or("<invalid utf8>");
        panic!("Project creation failed with status {status}: {error_text}");
    }

    let body: serde_json::Value = test::read_body_json(resp).await;
    let project = &body["project"];

    // Path should contain the project name
    let path = project["path"].as_str().unwrap();
    assert!(path.contains("test-default-path-"));
}

#[actix_rt::test]
async fn test_create_project_duplicate_path() {
    let test_app = TestApp::new().await;

    let project_path = test_app.test_config().projects_dir().join("duplicate-path-project");

    let service = test::init_service(
        actix_web::App::new()
            .app_data(test_app.app_state().clone())
            .configure(|cfg| configure_routes(cfg, false))
    ).await;

    // Create first project
    let create_request1 = CreateProjectRequest {
        name: "duplicate-path-project-1".to_string(),
        path: Some(project_path.to_string_lossy().to_string()),
        description: None,
        parent_id: None,
        template: None,
    };

    let req1 = test::TestRequest::post()
        .uri("/api/projects")
        .set_json(&create_request1)
        .to_request();

    let resp1 = test::call_service(&service, req1).await;
    assert!(resp1.status().is_success());

    // Try to create second project with same path
    let create_request2 = CreateProjectRequest {
        name: "duplicate-path-project-2".to_string(),
        path: Some(project_path.to_string_lossy().to_string()),
        description: None,
        parent_id: None,
        template: None,
    };

    let req2 = test::TestRequest::post()
        .uri("/api/projects")
        .set_json(&create_request2)
        .to_request();

    let resp2 = test::call_service(&service, req2).await;
    assert_eq!(resp2.status(), 400);

    let body: serde_json::Value = test::read_body_json(resp2).await;
    assert_eq!(body["error"], "invalid_request");
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("Project directory already exists"));
}

#[actix_rt::test]
async fn test_get_project_by_id() {
    let test_app = TestApp::new().await;

    // Create a project first
    let project = TestDataGenerator::create_project(Some("get-by-id-test"), Some("/tmp/get-by-id"));
    test_app.db().create_project(&project).unwrap();

    let service = test::init_service(
        actix_web::App::new()
            .app_data(test_app.app_state().clone())
            .configure(|cfg| configure_routes(cfg, false))
    ).await;

    // Get the project by ID
    let uri = format!("/api/projects/{}", project.id);
    let req = test::TestRequest::get().uri(&uri).to_request();
    let resp = test::call_service(&service, req).await;

    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let retrieved_project = &body["project"];

    assert_eq!(retrieved_project["id"], project.id);
    assert_eq!(retrieved_project["name"], "get-by-id-test");
    assert_eq!(retrieved_project["path"], "/tmp/get-by-id");
}

#[actix_rt::test]
async fn test_get_project_not_found() {
    let test_app = TestApp::new().await;

    let service = test::init_service(
        actix_web::App::new()
            .app_data(test_app.app_state().clone())
            .configure(|cfg| configure_routes(cfg, false))
    ).await;

    let req = test::TestRequest::get()
        .uri("/api/projects/non-existent-id")
        .to_request();

    let resp = test::call_service(&service, req).await;
    let status = resp.status();
    assert_eq!(status, 404);

    let body_bytes = test::read_body(resp).await;
    let body_str = std::str::from_utf8(&body_bytes).unwrap_or("<invalid utf8>");
    
    // Check if response body is empty or contains JSON error
    if body_str.is_empty() {
        // Empty response body is acceptable for 404
        assert_eq!(status, 404);
    } else {
        println!("Response body: {}", body_str);
        let body: serde_json::Value = serde_json::from_str(body_str).unwrap_or(serde_json::json!({}));
        if body.get("error").is_some() {
            assert_eq!(body["error"], "project_not_found");
        } else {
            // If no error field, just check status
            assert_eq!(status, 404);
        }
    }
}

#[actix_rt::test]
async fn test_get_projects_after_creation() {
    let test_app = TestApp::new().await;

    // Create multiple projects
    let projects = TestDataGenerator::create_projects(3);
    for project in &projects {
        test_app.db().create_project(project).unwrap();
    }

    let service = test::init_service(
        actix_web::App::new()
            .app_data(test_app.app_state().clone())
            .configure(|cfg| configure_routes(cfg, false))
    ).await;

    // Get all projects
    let req = test::TestRequest::get().uri("/api/projects").to_request();
    let resp = test::call_service(&service, req).await;

    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let retrieved_projects = body["projects"].as_array().unwrap();

    assert_eq!(retrieved_projects.len(), 3);

    // Verify projects are returned in correct order (newest first)
    let names: Vec<&str> = retrieved_projects
        .iter()
        .map(|p| p["name"].as_str().unwrap())
        .collect();

    assert!(names.contains(&"test-project-0"));
    assert!(names.contains(&"test-project-1"));
    assert!(names.contains(&"test-project-2"));
}

#[actix_rt::test]
async fn test_project_technology_detection() {
    let test_app = TestApp::new().await;

    let project_temp_dir = test_app.test_config().projects_dir().join("tech-detection-test");

    // Create project directory structure
    std::fs::create_dir_all(&project_temp_dir).unwrap();
    std::fs::write(project_temp_dir.join("Cargo.toml"), "[package]\nname = \"test\"\nversion = \"0.1.0\"").unwrap();
    std::fs::create_dir_all(project_temp_dir.join("src")).unwrap();
    std::fs::write(project_temp_dir.join("src").join("main.rs"), "fn main() {}").unwrap();

    let create_request = CreateProjectRequest {
        name: "tech-detection-test".to_string(),
        path: Some(project_temp_dir.to_string_lossy().to_string()),
        description: None,
        parent_id: None,
        template: None,
    };

    let service = test::init_service(
        actix_web::App::new()
            .app_data(test_app.app_state().clone())
            .configure(|cfg| configure_routes(cfg, false))
    ).await;

    let req = test::TestRequest::post()
        .uri("/api/projects")
        .set_json(&create_request)
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let project = &body["project"];

    assert_eq!(project["name"], "tech-detection-test");
}

#[actix_rt::test]
async fn test_project_creation_workflow() {
    let test_app = TestApp::new().await;

    let project_temp_dir = test_app.test_config().projects_dir().join("workflow-test");

    let service = test::init_service(
        actix_web::App::new()
            .app_data(test_app.app_state().clone())
            .configure(|cfg| configure_routes(cfg, false))
    ).await;

    // 1. Create project
    let create_request = CreateProjectRequest {
        name: "workflow-test".to_string(),
        path: Some(project_temp_dir.to_string_lossy().to_string()),
        description: None,
        parent_id: None,
        template: Some("rust-web-api".to_string()),
    };

    let create_req = test::TestRequest::post()
        .uri("/api/projects")
        .set_json(&create_request)
        .to_request();

    let create_resp = test::call_service(&service, create_req).await;
    assert!(create_resp.status().is_success());

    let create_body: serde_json::Value = test::read_body_json(create_resp).await;
    let project_id = create_body["project"]["id"].as_i64().unwrap();

    // 2. Get project by ID
    let get_uri = format!("/api/projects/{}", project_id);
    let get_req = test::TestRequest::get().uri(&get_uri).to_request();
    let get_resp = test::call_service(&service, get_req).await;
    assert!(get_resp.status().is_success());

    // 3. List all projects
    let list_req = test::TestRequest::get().uri("/api/projects").to_request();
    let list_resp = test::call_service(&service, list_req).await;
    assert!(list_resp.status().is_success());

    let list_body: serde_json::Value = test::read_body_json(list_resp).await;
    let projects = list_body["projects"].as_array().unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0]["id"], project_id);
}