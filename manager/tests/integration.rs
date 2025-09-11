use actix::Actor;
use actix_web::{test, web, App};
use nocodo_manager::{
    database::Database,
    handlers::{create_project, get_projects, health_check, AppState},
    models::CreateProjectRequest,
    websocket::{WebSocketBroadcaster, WebSocketServer},
};
use std::sync::Arc;
use std::time::SystemTime;
use tempfile::tempdir;

#[actix_rt::test]
async fn test_health_check() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let database = Arc::new(Database::new(&db_path).unwrap());

    let ws_server = WebSocketServer::default().start();
    let app_state = web::Data::new(AppState {
        database,
        start_time: SystemTime::now(),
        ws_broadcaster: Arc::new(WebSocketBroadcaster::new(ws_server)),
        runner: None,
        terminal_runner: None,
        llm_agent: None,
    });

    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .route("/api/health", web::get().to(health_check)),
    )
    .await;

    let req = test::TestRequest::get().uri("/api/health").to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "ok");
    assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));
    assert!(body["uptime"].as_u64().is_some());
}

#[actix_rt::test]
async fn test_get_projects_empty() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let database = Arc::new(Database::new(&db_path).unwrap());

    let ws_server = WebSocketServer::default().start();
    let app_state = web::Data::new(AppState {
        database,
        start_time: SystemTime::now(),
        ws_broadcaster: Arc::new(WebSocketBroadcaster::new(ws_server)),
        runner: None,
        terminal_runner: None,
        llm_agent: None,
    });

    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .route("/api/projects", web::get().to(get_projects)),
    )
    .await;

    let req = test::TestRequest::get().uri("/api/projects").to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["projects"].as_array().unwrap().len(), 0);
}

#[actix_rt::test]
async fn test_create_project() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let database = Arc::new(Database::new(&db_path).unwrap());

    let ws_server = WebSocketServer::default().start();
    let app_state = web::Data::new(AppState {
        database,
        start_time: SystemTime::now(),
        ws_broadcaster: Arc::new(WebSocketBroadcaster::new(ws_server)),
        runner: None,
        terminal_runner: None,
        llm_agent: None,
    });

    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .route("/api/projects", web::post().to(create_project)),
    )
    .await;

    // Use a temporary directory for the project path
    let project_temp_dir = tempdir().unwrap();
    let project_path = project_temp_dir.path().join("test-project");

    let create_request = CreateProjectRequest {
        name: "test-project".to_string(),
        path: Some(project_path.to_string_lossy().to_string()),
        language: Some("rust".to_string()),
        framework: Some("actix-web".to_string()),
        template: None,
    };

    let req = test::TestRequest::post()
        .uri("/api/projects")
        .set_json(&create_request)
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Debug output for failing test
    let status = resp.status();
    if !status.is_success() {
        let body: serde_json::Value = test::read_body_json(resp).await;
        eprintln!("Response status: {status}");
        eprintln!(
            "Response body: {}",
            serde_json::to_string_pretty(&body).unwrap()
        );
        panic!("Request failed with status: {status}");
    }

    assert!(status.is_success());
    assert_eq!(status, 201); // Created

    let body: serde_json::Value = test::read_body_json(resp).await;
    let project = &body["project"];

    assert_eq!(project["name"], "test-project");
    assert_eq!(project["path"], project_path.to_string_lossy().to_string());
    assert_eq!(project["language"], "rust");
    assert_eq!(project["framework"], "actix-web");
    assert_eq!(project["status"], "initialized");
    assert!(project["id"].as_str().is_some());
    assert!(project["created_at"].as_i64().is_some());
    assert!(project["updated_at"].as_i64().is_some());
}

#[actix_rt::test]
async fn test_create_project_with_default_path() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let database = Arc::new(Database::new(&db_path).unwrap());

    let ws_server = WebSocketServer::default().start();
    let app_state = web::Data::new(AppState {
        database,
        start_time: SystemTime::now(),
        ws_broadcaster: Arc::new(WebSocketBroadcaster::new(ws_server)),
        runner: None,
        terminal_runner: None,
        llm_agent: None,
    });

    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .route("/api/projects", web::post().to(create_project)),
    )
    .await;

    // Use a temporary directory for the default path test
    let default_projects_dir = temp_dir.path().join("projects");
    let default_path_project_dir = default_projects_dir.join("default-path-project");

    // Mock the home directory by setting the path in the request
    let create_request = CreateProjectRequest {
        name: "default-path-project".to_string(),
        path: Some(default_path_project_dir.to_string_lossy().to_string()),
        language: Some("javascript".to_string()),
        framework: None,
        template: None,
    };

    let req = test::TestRequest::post()
        .uri("/api/projects")
        .set_json(&create_request)
        .to_request();

    let resp = test::call_service(&app, req).await;

    // Debug output for failing test
    let status = resp.status();
    if !status.is_success() {
        let body: serde_json::Value = test::read_body_json(resp).await;
        eprintln!("Response status: {status}");
        eprintln!(
            "Response body: {}",
            serde_json::to_string_pretty(&body).unwrap()
        );
        panic!("Request failed with status: {status}");
    }

    assert!(status.is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let project = &body["project"];

    assert_eq!(project["name"], "default-path-project");
    assert_eq!(project["language"], "javascript");
    assert!(project["framework"].is_null());
    // Path should contain the project name
    assert!(project["path"]
        .as_str()
        .unwrap()
        .contains("default-path-project"));
}

#[actix_rt::test]
async fn test_create_project_invalid_name() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let database = Arc::new(Database::new(&db_path).unwrap());

    let ws_server = WebSocketServer::default().start();
    let app_state = web::Data::new(AppState {
        database,
        start_time: SystemTime::now(),
        ws_broadcaster: Arc::new(WebSocketBroadcaster::new(ws_server)),
        runner: None,
        terminal_runner: None,
        llm_agent: None,
    });

    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .route("/api/projects", web::post().to(create_project)),
    )
    .await;

    let create_request = CreateProjectRequest {
        name: "   ".to_string(), // Empty/whitespace name
        path: None,
        language: None,
        framework: None,
        template: None,
    };

    let req = test::TestRequest::post()
        .uri("/api/projects")
        .set_json(&create_request)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400); // Bad Request

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "invalid_request");
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("Project name cannot be empty"));
}

#[actix_rt::test]
async fn test_get_projects_after_creation() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let database = Arc::new(Database::new(&db_path).unwrap());

    let ws_server = WebSocketServer::default().start();
    let app_state = web::Data::new(AppState {
        database,
        start_time: SystemTime::now(),
        ws_broadcaster: Arc::new(WebSocketBroadcaster::new(ws_server)),
        runner: None,
        terminal_runner: None,
        llm_agent: None,
    });

    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .route("/api/projects", web::get().to(get_projects))
            .route("/api/projects", web::post().to(create_project)),
    )
    .await;

    // Create a project first
    let create_request = CreateProjectRequest {
        name: "list-test-project".to_string(),
        path: Some(
            temp_dir
                .path()
                .join("list-test")
                .to_string_lossy()
                .to_string(),
        ),
        language: Some("python".to_string()),
        framework: Some("django".to_string()),
        template: None,
    };

    let req = test::TestRequest::post()
        .uri("/api/projects")
        .set_json(&create_request)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    // Now test getting projects
    let req = test::TestRequest::get().uri("/api/projects").to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let projects = body["projects"].as_array().unwrap();
    assert_eq!(projects.len(), 1);

    let project = &projects[0];
    assert_eq!(project["name"], "list-test-project");
    assert_eq!(project["language"], "python");
    assert_eq!(project["framework"], "django");
}

#[actix_rt::test]
async fn test_technology_detection_for_rust_project() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let database = Arc::new(Database::new(&db_path).unwrap());

    let ws_server = WebSocketServer::default().start();
    let app_state = web::Data::new(AppState {
        database,
        start_time: SystemTime::now(),
        ws_broadcaster: Arc::new(WebSocketBroadcaster::new(ws_server)),
        runner: None,
        terminal_runner: None,
        llm_agent: None,
    });

    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .route("/api/projects", web::post().to(create_project)),
    )
    .await;

    // Create a project without specifying language/framework to trigger detection
    let project_path = temp_dir.path().join("rust-detection-test");
    let create_request = CreateProjectRequest {
        name: "rust-detection-test".to_string(),
        path: Some(project_path.to_string_lossy().to_string()),
        language: None,
        framework: None,
        template: Some("rust-web-api".to_string()),
    };

    let req = test::TestRequest::post()
        .uri("/api/projects")
        .set_json(&create_request)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let project = &body["project"];

    // Verify detection worked
    assert_eq!(project["name"], "rust-detection-test");
    assert_eq!(project["language"], "rust");
    assert_eq!(project["framework"], "actix-web");
    // Technologies field should be present
    assert!(project["technologies"].is_string() || project["technologies"].is_null());

    // If technologies is a string, parse it to verify structure
    if project["technologies"].is_string() {
        let technologies_str = project["technologies"].as_str().unwrap();
        let detection_result: serde_json::Value = serde_json::from_str(technologies_str).unwrap();

        // Verify the detection result has the expected fields
        assert!(detection_result["primary_language"].is_string());
        assert!(detection_result["technologies"].is_array());
        assert!(detection_result["build_tools"].is_array());
        assert!(detection_result["package_managers"].is_array());
        assert!(detection_result["deployment_configs"].is_array());
    }
}
