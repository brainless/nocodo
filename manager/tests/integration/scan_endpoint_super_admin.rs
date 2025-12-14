//! Test to verify Super Admins can access the /api/projects/scan endpoint

use actix_web::{test, web, App};
use nocodo_manager::handlers::{user_handlers, project_handlers};
use nocodo_manager::middleware::{AuthenticationMiddleware, PermissionMiddleware, PermissionRequirement};
use serde_json::json;

use crate::common::TestApp;

#[actix_rt::test]
async fn test_super_admin_can_scan_projects() {
    // Create a test app with REAL authentication (jwt_secret set)
    let test_app = TestApp::new().await;

    // Configure JWT secret for authentication and projects path
    {
        let mut config = test_app.app_state.config.write().unwrap();
        config.auth = Some(nocodo_manager::config::AuthConfig {
            jwt_secret: Some("test_jwt_secret_for_scan_test".to_string()),
        });
        config.projects = Some(nocodo_manager::config::ProjectsConfig {
            default_path: Some(test_app.config.temp_dir.path().to_string_lossy().to_string()),
        });
    }

    // Create service with authentication and permission middleware
    let service = test::init_service(
        App::new()
            .app_data(test_app.app_state.clone())
            .wrap(AuthenticationMiddleware)
            // Auth endpoints (no auth required)
            .route("/api/auth/register", web::post().to(user_handlers::register))
            .route("/api/auth/login", web::post().to(user_handlers::login))
            // Projects scope with nested permission middlewares (matching actual routes.rs)
            .service(
                web::scope("/api/projects")
                    .wrap(PermissionMiddleware::new(PermissionRequirement::new("project", "read")))
                    .service(
                        web::resource("/scan")
                            .wrap(PermissionMiddleware::new(
                                PermissionRequirement::new("project", "write"),
                            ))
                            .route(web::post().to(project_handlers::scan_projects)),
                    )
            )
    )
    .await;

    // Step 1: Register first user (should become Super Admin)
    let register_req = json!({
        "username": "admin",
        "email": "admin@example.com",
        "password": "password123"
    });

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&register_req)
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert_eq!(resp.status(), 201, "User registration should succeed");

    // Step 2: Login to get JWT token
    let login_req = json!({
        "username": "admin",
        "password": "password123"
    });

    let req = test::TestRequest::post()
        .uri("/api/auth/login")
        .set_json(&login_req)
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert_eq!(resp.status(), 200, "Login should succeed");

    let body: serde_json::Value = test::read_body_json(resp).await;
    let token = body["token"].as_str().expect("Token should be present");

    println!("\n=== Testing scan endpoint as Super Admin ===");
    println!("Token: {}", token);

    // Step 3: Try to access scan endpoint
    let req = test::TestRequest::post()
        .uri("/api/projects/scan")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&service, req).await;

    let status = resp.status();
    println!("Response status: {}", status);
    println!("Response headers: {:?}", resp.headers());

    // If error, try to get error message
    if !status.is_success() {
        let body_bytes = test::read_body(resp).await;
        let body_str = String::from_utf8_lossy(&body_bytes);
        println!("Error response body: {}", body_str);
        panic!(
            "Super Admin should be able to scan projects. Got {} status. Body: {}",
            status,
            body_str
        );
    }

    // This should succeed because Super Admins have implicit all permissions
    assert_eq!(
        status,
        200,
        "Super Admin should be able to scan projects"
    );

    let body: serde_json::Value = test::read_body_json(resp).await;
    println!("Response body: {}", serde_json::to_string_pretty(&body).unwrap());

    assert!(
        body.get("results").is_some(),
        "Response should contain 'results' field"
    );

    println!("\n✓ Super Admin can successfully scan projects");
}
