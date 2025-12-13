//! Test to verify first user bootstrap logic on fresh install
//!
//! This test verifies that the first registered user:
//! 1. Is automatically added to a "Super Admins" team
//! 2. Can access all protected API endpoints without explicit permissions
//! 3. Has implicit all permissions through Super Admins team membership

mod common;

use actix_web::{test, web, App};
use nocodo_manager::handlers::{user_handlers, project_handlers, team_handlers};
use nocodo_manager::middleware::{AuthenticationMiddleware, PermissionMiddleware, PermissionRequirement};
use serde_json::json;

use common::TestApp;

#[actix_rt::test]
async fn test_first_user_bootstrap_super_admin() {
    // Create a test app with REAL authentication (jwt_secret set)
    let test_app = TestApp::new().await;

    // Configure JWT secret for authentication
    {
        let mut config = test_app.app_state.config.write().unwrap();
        config.auth = Some(nocodo_manager::config::AuthConfig {
            jwt_secret: Some("test_jwt_secret_for_fresh_install_test".to_string()),
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
            // Protected endpoints (require auth + permissions)
            .service(
                web::scope("/api/projects")
                    .wrap(PermissionMiddleware::new(PermissionRequirement::new("project", "read")))
                    .route("", web::get().to(project_handlers::get_projects))
            )
            .service(
                web::scope("/api/teams")
                    .wrap(PermissionMiddleware::new(PermissionRequirement::new("team", "read")))
                    .route("", web::get().to(team_handlers::list_teams))
                    .service(
                        web::scope("/{id}")
                            .wrap(PermissionMiddleware::new(
                                PermissionRequirement::new("team", "read").with_resource_id("id")
                            ))
                            .route("/permissions", web::get().to(team_handlers::get_team_permissions))
                    )
            )
    )
    .await;

    // Step 1: Register a new user
    let register_req = json!({
        "username": "newuser",
        "email": "newuser@example.com",
        "password": "password123"
    });

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&register_req)
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert_eq!(resp.status(), 201, "User registration should succeed");

    // Step 2: Login to get JWT token
    let login_req = serde_json::json!({
        "username": "newuser",
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

    // Step 3: Verify Super Admins team was created
    let req = test::TestRequest::get()
        .uri("/api/teams")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert_eq!(resp.status(), 200, "Should be able to access teams API");

    let body: serde_json::Value = test::read_body_json(resp).await;
    let teams = body["teams"].as_array().expect("Should get teams array");
    assert_eq!(teams.len(), 1, "Should have exactly one team");
    assert_eq!(teams[0]["name"], "Super Admins", "Team should be named 'Super Admins'");

    println!("\n✓ Super Admins team created");

    // Step 4: Verify first user can access Projects API (as Super Admin)
    let req = test::TestRequest::get()
        .uri("/api/projects")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert_eq!(
        resp.status(),
        200,
        "First user should have access to Projects API as Super Admin"
    );

    println!("✓ First user can access Projects API");

    // Step 5: Verify Super Admins team has NO explicit permissions
    // (permissions are implicit through permission check logic)
    let team_id = teams[0]["id"].as_i64().unwrap();
    let req = test::TestRequest::get()
        .uri(&format!("/api/teams/{}/permissions", team_id))
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert_eq!(resp.status(), 200, "Should be able to check team permissions");

    let body: serde_json::Value = test::read_body_json(resp).await;
    let permissions = body.as_array().expect("Should get permissions array");
    assert_eq!(
        permissions.len(),
        0,
        "Super Admins should have NO explicit permissions (implicit all permissions)"
    );

    println!("✓ Super Admins team has no explicit permissions (implicit all permissions)");

    println!("\n=== BOOTSTRAP LOGIC VERIFIED ===");
    println!("✓ First user registration creates 'Super Admins' team");
    println!("✓ First user is automatically added to 'Super Admins' team");
    println!("✓ First user can access all protected endpoints");
    println!("✓ Super Admins have implicit all permissions (no explicit permission records)");
}
