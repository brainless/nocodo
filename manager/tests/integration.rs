use actix_web::{test, web, App};
use nocodo_manager::{
    database::Database,
    handlers::{AppState, create_project, get_projects, health_check},
    models::CreateProjectRequest,
};
use std::sync::Arc;
use std::time::SystemTime;
use tempfile::tempdir;

#[actix_rt::test]
async fn test_health_check() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let database = Arc::new(Database::new(&db_path).unwrap());
    
    let app_state = web::Data::new(AppState {
        database,
        start_time: SystemTime::now(),
    });

    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .route("/api/health", web::get().to(health_check))
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/health")
        .to_request();

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
    
    let app_state = web::Data::new(AppState {
        database,
        start_time: SystemTime::now(),
    });

    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .route("/api/projects", web::get().to(get_projects))
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/projects")
        .to_request();

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
    
    let app_state = web::Data::new(AppState {
        database,
        start_time: SystemTime::now(),
    });

    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .route("/api/projects", web::post().to(create_project))
    )
    .await;

    let create_request = CreateProjectRequest {
        name: "test-project".to_string(),
        path: Some("/tmp/test-project".to_string()),
        language: Some("rust".to_string()),
        framework: Some("actix-web".to_string()),
    };

    let req = test::TestRequest::post()
        .uri("/api/projects")
        .set_json(&create_request)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
    assert_eq!(resp.status(), 201); // Created

    let body: serde_json::Value = test::read_body_json(resp).await;
    let project = &body["project"];
    
    assert_eq!(project["name"], "test-project");
    assert_eq!(project["path"], "/tmp/test-project");
    assert_eq!(project["language"], "rust");
    assert_eq!(project["framework"], "actix-web");
    assert_eq!(project["status"], "created");
    assert!(project["id"].as_str().is_some());
    assert!(project["created_at"].as_i64().is_some());
    assert!(project["updated_at"].as_i64().is_some());
}

#[actix_rt::test]
async fn test_create_project_with_default_path() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let database = Arc::new(Database::new(&db_path).unwrap());
    
    let app_state = web::Data::new(AppState {
        database,
        start_time: SystemTime::now(),
    });

    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .route("/api/projects", web::post().to(create_project))
    )
    .await;

    let create_request = CreateProjectRequest {
        name: "default-path-project".to_string(),
        path: None, // Should use default path generation
        language: Some("javascript".to_string()),
        framework: None,
    };

    let req = test::TestRequest::post()
        .uri("/api/projects")
        .set_json(&create_request)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let project = &body["project"];
    
    assert_eq!(project["name"], "default-path-project");
    assert_eq!(project["language"], "javascript");
    assert!(project["framework"].is_null());
    // Path should contain the project name
    assert!(project["path"].as_str().unwrap().contains("default-path-project"));
}

#[actix_rt::test]
async fn test_create_project_invalid_name() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let database = Arc::new(Database::new(&db_path).unwrap());
    
    let app_state = web::Data::new(AppState {
        database,
        start_time: SystemTime::now(),
    });

    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .route("/api/projects", web::post().to(create_project))
    )
    .await;

    let create_request = CreateProjectRequest {
        name: "   ".to_string(), // Empty/whitespace name
        path: None,
        language: None,
        framework: None,
    };

    let req = test::TestRequest::post()
        .uri("/api/projects")
        .set_json(&create_request)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400); // Bad Request

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["error"], "invalid_request");
    assert!(body["message"].as_str().unwrap().contains("Project name cannot be empty"));
}

#[actix_rt::test]
async fn test_get_projects_after_creation() {
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let database = Arc::new(Database::new(&db_path).unwrap());
    
    let app_state = web::Data::new(AppState {
        database,
        start_time: SystemTime::now(),
    });

    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .route("/api/projects", web::get().to(get_projects))
            .route("/api/projects", web::post().to(create_project))
    )
    .await;

    // Create a project first
    let create_request = CreateProjectRequest {
        name: "list-test-project".to_string(),
        path: Some("/tmp/list-test".to_string()),
        language: Some("python".to_string()),
        framework: Some("django".to_string()),
    };

    let req = test::TestRequest::post()
        .uri("/api/projects")
        .set_json(&create_request)
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    // Now test getting projects
    let req = test::TestRequest::get()
        .uri("/api/projects")
        .to_request();

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
