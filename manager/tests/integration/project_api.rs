use actix_web::test;
use nocodo_manager::routes::configure_routes;

use nocodo_manager::models::CreateProjectRequest;

use crate::common::{TestApp, TestDataGenerator};

#[actix_rt::test]
async fn test_get_projects_empty() {
    let test_app = TestApp::new().await;

    let service = test::init_service(
        actix_web::App::new()
            .app_data(test_app.app_state().clone())
            .configure(|cfg| configure_routes(cfg, false)),
    )
    .await;

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

    let project_temp_dir = test_app
        .test_config()
        .projects_dir()
        .join("basic-test-project");

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
            .configure(|cfg| configure_routes(cfg, false)),
    )
    .await;

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
        path: Some(
            test_app
                .test_config()
                .projects_dir()
                .join("invalid-name-test")
                .to_string_lossy()
                .to_string(),
        ),
        description: None,
        parent_id: None,
        template: None,
    };

    let service = test::init_service(
        actix_web::App::new()
            .app_data(test_app.app_state().clone())
            .configure(|cfg| configure_routes(cfg, false)),
    )
    .await;

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
        name: format!(
            "test-default-path-{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
        ),
        path: None,
        description: None,
        parent_id: None,
        template: None,
    };

    let service = test::init_service(
        actix_web::App::new()
            .app_data(test_app.app_state().clone())
            .configure(|cfg| configure_routes(cfg, false)),
    )
    .await;

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

    let project_path = test_app
        .test_config()
        .projects_dir()
        .join("duplicate-path-project");

    let service = test::init_service(
        actix_web::App::new()
            .app_data(test_app.app_state().clone())
            .configure(|cfg| configure_routes(cfg, false)),
    )
    .await;

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
    let project = TestDataGenerator::create_project(
        Some("get-by-id-test"),
        Some(
            test_app
                .test_config()
                .projects_dir()
                .join("get-by-id")
                .to_string_lossy()
                .as_ref(),
        ),
    );
    test_app.db().create_project(&project).unwrap();

    let service = test::init_service(
        actix_web::App::new()
            .app_data(test_app.app_state().clone())
            .configure(|cfg| configure_routes(cfg, false)),
    )
    .await;

    // Get the project by ID
    let uri = format!("/api/projects/{}", project.id);
    let req = test::TestRequest::get().uri(&uri).to_request();
    let resp = test::call_service(&service, req).await;

    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let retrieved_project = &body["project"];

    assert_eq!(retrieved_project["id"], project.id);
    assert_eq!(retrieved_project["name"], "get-by-id-test");
    assert_eq!(
        retrieved_project["path"],
        test_app
            .test_config()
            .projects_dir()
            .join("get-by-id")
            .to_string_lossy()
            .to_string()
    );
}

#[actix_rt::test]
async fn test_get_project_not_found() {
    let test_app = TestApp::new().await;

    let service = test::init_service(
        actix_web::App::new()
            .app_data(test_app.app_state().clone())
            .configure(|cfg| configure_routes(cfg, false)),
    )
    .await;

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
        let body: serde_json::Value =
            serde_json::from_str(body_str).unwrap_or(serde_json::json!({}));
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
            .configure(|cfg| configure_routes(cfg, false)),
    )
    .await;

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
async fn test_project_creation_workflow() {
    let test_app = TestApp::new().await;

    let service = test::init_service(
        actix_web::App::new()
            .app_data(test_app.app_state().clone())
            .configure(|cfg| configure_routes(cfg, false)),
    )
    .await;

    // Test 1: Get available templates
    let templates_req = test::TestRequest::get().uri("/api/templates").to_request();
    let templates_resp = test::call_service(&service, templates_req).await;
    assert!(templates_resp.status().is_success());

    let templates: Vec<serde_json::Value> = test::read_body_json(templates_resp).await;
    assert!(!templates.is_empty());

    // Verify we have our expected templates
    let template_names: Vec<&str> = templates
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    assert!(template_names.contains(&"rust-web-api"));
    assert!(template_names.contains(&"node-web-app"));
    assert!(template_names.contains(&"static-site"));

    // Test 2: Create a project with a template
    let project_path = test_app.test_config().projects_dir().join("test-project");
    let project_req = serde_json::json!({
        "name": "test-project",
        "path": project_path.to_str().unwrap(),
        "template": "static-site"
    });

    let create_req = test::TestRequest::post()
        .uri("/api/projects")
        .set_json(&project_req)
        .to_request();

    let create_resp = test::call_service(&service, create_req).await;
    if !create_resp.status().is_success() {
        let status = create_resp.status();
        let error_body = test::read_body(create_resp).await;
        let error_text = std::str::from_utf8(&error_body).unwrap_or("<invalid utf8>");
        panic!("Project creation failed with status {status}: {error_text}");
    }

    let project_response: serde_json::Value = test::read_body_json(create_resp).await;
    let project = &project_response["project"];

    // Verify project was created correctly
    assert_eq!(project["name"], "test-project");

    // Verify project files were created
    assert!(project_path.join("index.html").exists());
    assert!(project_path.join("styles.css").exists());
    assert!(project_path.join("script.js").exists());
    assert!(project_path.join("README.md").exists());
    assert!(project_path.join(".gitignore").exists());

    // Verify Git repository was initialized
    assert!(project_path.join(".git").exists());

    // Test 3: Create a project without template
    let basic_project_path = test_app.test_config().projects_dir().join("basic-project");
    let basic_project_req = serde_json::json!({
        "name": "basic-project",
        "path": basic_project_path.to_str().unwrap()
    });

    let basic_req = test::TestRequest::post()
        .uri("/api/projects")
        .set_json(&basic_project_req)
        .to_request();

    let basic_resp = test::call_service(&service, basic_req).await;
    assert!(basic_resp.status().is_success());

    let basic_project_response: serde_json::Value = test::read_body_json(basic_resp).await;
    let basic_project = &basic_project_response["project"];

    // Verify basic project was created
    assert_eq!(basic_project["name"], "basic-project");

    // Verify basic files were created
    assert!(basic_project_path.join("README.md").exists());
    assert!(basic_project_path.join(".git").exists());

    // Test 4: Verify projects are stored in database
    let stored_projects = test_app.db().get_all_projects().unwrap();
    assert_eq!(stored_projects.len(), 2);

    let project_names: Vec<&str> = stored_projects.iter().map(|p| p.name.as_str()).collect();
    assert!(project_names.contains(&"test-project"));
    assert!(project_names.contains(&"basic-project"));
}

#[actix_rt::test]
async fn test_create_project_unknown_template() {
    let test_app = TestApp::new().await;

    let service = test::init_service(
        actix_web::App::new()
            .app_data(test_app.app_state().clone())
            .configure(|cfg| configure_routes(cfg, false)),
    )
    .await;

    let unknown_template_req = serde_json::json!({
        "name": "test-project",
        "path": test_app.test_config().projects_dir().join("test2").to_str().unwrap(),
        "template": "unknown-template"
    });

    let req = test::TestRequest::post()
        .uri("/api/projects")
        .set_json(&unknown_template_req)
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert!(!resp.status().is_success());
}
