use actix_web::{test, web, App};
use nocodo_manager::database::Database;
use nocodo_manager::handlers::{create_project, get_templates, AppState};
use nocodo_manager::websocket::{WebSocketBroadcaster, WebSocketServer};
use actix::Actor;
use std::sync::Arc;
use std::time::SystemTime;
use tempfile::tempdir;

#[actix_rt::test]
async fn test_project_creation_workflow() {
    // Create a temporary database
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let database = Arc::new(Database::new(&db_path).unwrap());

    // Create app state
    let ws_server = WebSocketServer::default().start();
    let app_state = web::Data::new(AppState {
        database: database.clone(),
        start_time: SystemTime::now(),
        ws_broadcaster: Arc::new(WebSocketBroadcaster::new(ws_server)),
    });

    // Initialize the test app
    let app = test::init_service(
        App::new()
            .app_data(app_state.clone())
            .route("/api/projects", web::post().to(create_project))
            .route("/api/templates", web::get().to(get_templates)),
    )
    .await;

    // Test 1: Get available templates
    let templates_req = test::TestRequest::get().uri("/api/templates").to_request();

    let templates_resp = test::call_service(&app, templates_req).await;
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
    let temp_parent_dir = tempdir().unwrap();
    let project_path = temp_parent_dir.path().join("test-project");
    let project_req = serde_json::json!({
        "name": "test-project",
        "path": project_path.to_str().unwrap(),
        "template": "static-site"
    });

    let create_req = test::TestRequest::post()
        .uri("/api/projects")
        .set_json(&project_req)
        .to_request();

    let create_resp = test::call_service(&app, create_req).await;
    if !create_resp.status().is_success() {
        let status = create_resp.status();
        let error_body = test::read_body(create_resp).await;
        let error_text = std::str::from_utf8(&error_body).unwrap_or("<invalid utf8>");
        panic!(
            "Project creation failed with status {status}: {error_text}"
        );
    }

    let project_response: serde_json::Value = test::read_body_json(create_resp).await;
    let project = &project_response["project"];

    // Verify project was created correctly
    assert_eq!(project["name"], "test-project");
    assert_eq!(project["language"], "html");
    assert_eq!(project["status"], "initialized");

    // Verify project files were created
    assert!(project_path.join("index.html").exists());
    assert!(project_path.join("styles.css").exists());
    assert!(project_path.join("script.js").exists());
    assert!(project_path.join("README.md").exists());
    assert!(project_path.join(".gitignore").exists());

    // Verify Git repository was initialized
    assert!(project_path.join(".git").exists());

    // Test 3: Create a project without template
    let temp_parent_dir2 = tempdir().unwrap();
    let basic_project_path = temp_parent_dir2.path().join("basic-project");
    let basic_project_req = serde_json::json!({
        "name": "basic-project",
        "path": basic_project_path.to_str().unwrap()
    });

    let basic_req = test::TestRequest::post()
        .uri("/api/projects")
        .set_json(&basic_project_req)
        .to_request();

    let basic_resp = test::call_service(&app, basic_req).await;
    assert!(basic_resp.status().is_success());

    let basic_project_response: serde_json::Value = test::read_body_json(basic_resp).await;
    let basic_project = &basic_project_response["project"];

    // Verify basic project was created
    assert_eq!(basic_project["name"], "basic-project");
    assert_eq!(basic_project["status"], "initialized");

    // Verify basic files were created
    assert!(basic_project_path.join("README.md").exists());
    assert!(basic_project_path.join(".git").exists());

    // Test 4: Verify projects are stored in database
    let stored_projects = database.get_all_projects().unwrap();
    assert_eq!(stored_projects.len(), 2);

    let project_names: Vec<&str> = stored_projects.iter().map(|p| p.name.as_str()).collect();
    assert!(project_names.contains(&"test-project"));
    assert!(project_names.contains(&"basic-project"));
}

#[actix_rt::test]
async fn test_project_creation_error_handling() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let database = Arc::new(Database::new(&db_path).unwrap());

    let ws_server = WebSocketServer::default().start();
    let app_state = web::Data::new(AppState {
        database,
        start_time: SystemTime::now(),
        ws_broadcaster: Arc::new(WebSocketBroadcaster::new(ws_server)),
    });

    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .route("/api/projects", web::post().to(create_project)),
    )
    .await;

    // Test: Invalid request (empty name)
    let invalid_req = serde_json::json!({
        "name": "",
        "path": "/tmp/test"
    });

    let req = test::TestRequest::post()
        .uri("/api/projects")
        .set_json(&invalid_req)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(!resp.status().is_success());

    // Test: Unknown template
    let unknown_template_req = serde_json::json!({
        "name": "test-project",
        "path": "/tmp/test2",
        "template": "unknown-template"
    });

    let req = test::TestRequest::post()
        .uri("/api/projects")
        .set_json(&unknown_template_req)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(!resp.status().is_success());
}
