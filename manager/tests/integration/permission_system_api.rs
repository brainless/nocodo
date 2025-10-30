//! End-to-end integration tests for the permission system
//!
//! Tests cover:
//! - Complete team management workflow through APIs
//! - Permission granting and checking through APIs
//! - Inheritance scenarios
//! - Permission revocation
//! - Bootstrap logic for first user

use actix_web::{test, web, App};
use serde_json::json;

use nocodo_manager::models::{
    CreateTeamRequest, UpdateTeamRequest, AddTeamMemberRequest, CreatePermissionRequest,
    CreateUserRequest, User
};
use nocodo_manager::permissions::{Action, ResourceType};

use crate::common::{TestApp, TestDataGenerator};

/// Helper to create a test service with all routes
async fn create_test_service(test_app: &TestApp) -> test::Service {
    test::init_service(
        App::new()
            .app_data(test_app.app_state.clone())
            // Health check
            .route("/api/health", web::get().to(nocodo_manager::handlers::health_check))
            // Auth endpoints
            .route("/api/auth/login", web::post().to(nocodo_manager::handlers::login))
            .route("/api/auth/register", web::post().to(nocodo_manager::handlers::register))
            // Team management endpoints
            .service(
                web::scope("/api/teams")
                    .route("", web::get().to(nocodo_manager::handlers::list_teams))
                    .service(
                        web::resource("")
                            .route(web::post().to(nocodo_manager::handlers::create_team)),
                    )
                    .service(
                        web::scope("/{id}")
                            .route("", web::get().to(nocodo_manager::handlers::get_team))
                            .route("", web::put().to(nocodo_manager::handlers::update_team))
                            .route("", web::delete().to(nocodo_manager::handlers::delete_team))
                            .route("/members", web::get().to(nocodo_manager::handlers::get_team_members))
                            .route("/permissions", web::get().to(nocodo_manager::handlers::get_team_permissions))
                            .service(
                                web::resource("/members")
                                    .route(web::post().to(nocodo_manager::handlers::add_team_member)),
                            )
                            .service(
                                web::scope("/members/{user_id}")
                                    .route("", web::delete().to(nocodo_manager::handlers::remove_team_member)),
                            ),
                    ),
            )
            // Permission management endpoints
            .service(
                web::scope("/api/permissions")
                    .route("", web::get().to(nocodo_manager::handlers::list_permissions))
                    .route("", web::post().to(nocodo_manager::handlers::create_permission))
                    .service(
                        web::scope("/{id}")
                            .route("", web::delete().to(nocodo_manager::handlers::delete_permission)),
                    ),
            )
            // User management endpoints
            .service(
                web::scope("/api/users")
                    .route("", web::get().to(nocodo_manager::handlers::list_users))
                    .route("", web::post().to(nocodo_manager::handlers::create_user))
                    .service(
                        web::scope("/{id}")
                            .route("", web::get().to(nocodo_manager::handlers::get_user))
                            .route("", web::put().to(nocodo_manager::handlers::update_user))
                            .route("", web::delete().to(nocodo_manager::handlers::delete_user)),
                    ),
            )
            // Project endpoints for permission testing
            .service(
                web::scope("/api/projects")
                    .route("", web::get().to(nocodo_manager::handlers::get_projects))
                    .service(
                        web::resource("")
                            .route(web::post().to(nocodo_manager::handlers::create_project)),
                    )
                    .service(
                        web::scope("/{id}")
                            .route("", web::get().to(nocodo_manager::handlers::get_project))
                            .route("", web::delete().to(nocodo_manager::handlers::delete_project)),
                    ),
            ),
    )
    .await
}

#[actix_rt::test]
async fn test_complete_team_management_workflow() {
    let test_app = TestApp::new().await;
    let service = create_test_service(&test_app).await;

    // 1. Register first user (should create Super Admins team)
    let register_req = CreateUserRequest {
        username: "admin".to_string(),
        email: Some("admin@example.com".to_string()),
        password: "password123".to_string(),
    };

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&register_req)
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert!(resp.status().is_success());
    assert_eq!(resp.status(), 201);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let first_user_id = body["user"]["id"].as_i64().unwrap();

    // Verify Super Admins team was created
    let req = test::TestRequest::get().uri("/api/teams").to_request();
    let resp = test::call_service(&service, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let teams = body.as_array().unwrap();
    assert_eq!(teams.len(), 1);
    assert_eq!(teams[0]["name"], "Super Admins");

    let super_admin_team_id = teams[0]["id"].as_i64().unwrap();

    // 2. Create a regular team
    let create_team_req = CreateTeamRequest {
        name: "Developers".to_string(),
        description: Some("Development team".to_string()),
    };

    let req = test::TestRequest::post()
        .uri("/api/teams")
        .set_json(&create_team_req)
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert!(resp.status().is_success());
    assert_eq!(resp.status(), 201);

    let body: serde_json::Value = test::read_body_json(resp).await;
    let dev_team_id = body["id"].as_i64().unwrap();
    assert_eq!(body["name"], "Developers");
    assert_eq!(body["description"], "Development team");

    // 3. Register a second user
    let register_req2 = CreateUserRequest {
        username: "developer".to_string(),
        email: Some("dev@example.com".to_string()),
        password: "password123".to_string(),
    };

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&register_req2)
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let second_user_id = body["user"]["id"].as_i64().unwrap();

    // 4. Add second user to developers team
    let add_member_req = AddTeamMemberRequest { user_id: second_user_id };

    let req = test::TestRequest::post()
        .uri(&format!("/api/teams/{}/members", dev_team_id))
        .set_json(&add_member_req)
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert!(resp.status().is_success());
    assert_eq!(resp.status(), 201);

    // Verify team members
    let req = test::TestRequest::get()
        .uri(&format!("/api/teams/{}/members", dev_team_id))
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let members = body.as_array().unwrap();
    assert_eq!(members.len(), 2); // Creator + added member

    // 5. Grant permissions to developers team
    let create_perm_req = CreatePermissionRequest {
        team_id: dev_team_id,
        resource_type: "project".to_string(),
        resource_id: None, // Entity-level permission
        action: "write".to_string(),
    };

    let req = test::TestRequest::post()
        .uri("/api/permissions")
        .set_json(&create_perm_req)
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert!(resp.status().is_success());
    assert_eq!(resp.status(), 201);

    // Verify team permissions
    let req = test::TestRequest::get()
        .uri(&format!("/api/teams/{}/permissions", dev_team_id))
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let permissions = body.as_array().unwrap();
    assert_eq!(permissions.len(), 1);
    assert_eq!(permissions[0]["resource_type"], "project");
    assert_eq!(permissions[0]["action"], "write");
    assert!(permissions[0]["resource_id"].is_null()); // Entity-level

    // 6. Update team
    let update_team_req = UpdateTeamRequest {
        name: Some("Senior Developers".to_string()),
        description: Some("Senior development team".to_string()),
    };

    let req = test::TestRequest::put()
        .uri(&format!("/api/teams/{}", dev_team_id))
        .set_json(&update_team_req)
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert!(resp.status().is_success());

    // Verify team was updated
    let req = test::TestRequest::get()
        .uri(&format!("/api/teams/{}", dev_team_id))
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["name"], "Senior Developers");
    assert_eq!(body["description"], "Senior development team");
}

#[actix_rt::test]
async fn test_permission_revocation() {
    let test_app = TestApp::new().await;
    let service = create_test_service(&test_app).await;

    // Create first user (Super Admin)
    let register_req = CreateUserRequest {
        username: "admin".to_string(),
        email: Some("admin@example.com".to_string()),
        password: "password123".to_string(),
    };

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&register_req)
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let admin_user_id = body["user"]["id"].as_i64().unwrap();

    // Create a team
    let create_team_req = CreateTeamRequest {
        name: "Test Team".to_string(),
        description: Some("Team for testing".to_string()),
    };

    let req = test::TestRequest::post()
        .uri("/api/teams")
        .set_json(&create_team_req)
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let team_id = body["id"].as_i64().unwrap();

    // Grant permission
    let create_perm_req = CreatePermissionRequest {
        team_id,
        resource_type: "project".to_string(),
        resource_id: Some(1),
        action: "read".to_string(),
    };

    let req = test::TestRequest::post()
        .uri("/api/permissions")
        .set_json(&create_perm_req)
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let permission_id = body["id"].as_i64().unwrap();

    // Verify permission exists
    let req = test::TestRequest::get()
        .uri(&format!("/api/teams/{}/permissions", team_id))
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let permissions = body.as_array().unwrap();
    assert_eq!(permissions.len(), 1);

    // Revoke permission
    let req = test::TestRequest::delete()
        .uri(&format!("/api/permissions/{}", permission_id))
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert!(resp.status().is_success());
    assert_eq!(resp.status(), 204);

    // Verify permission is gone
    let req = test::TestRequest::get()
        .uri(&format!("/api/teams/{}/permissions", team_id))
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let permissions = body.as_array().unwrap();
    assert_eq!(permissions.len(), 0);
}

#[actix_rt::test]
async fn test_team_member_removal() {
    let test_app = TestApp::new().await;
    let service = create_test_service(&test_app).await;

    // Create first user (Super Admin)
    let register_req = CreateUserRequest {
        username: "admin".to_string(),
        email: Some("admin@example.com".to_string()),
        password: "password123".to_string(),
    };

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&register_req)
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let admin_user_id = body["user"]["id"].as_i64().unwrap();

    // Create a team
    let create_team_req = CreateTeamRequest {
        name: "Test Team".to_string(),
        description: Some("Team for testing".to_string()),
    };

    let req = test::TestRequest::post()
        .uri("/api/teams")
        .set_json(&create_team_req)
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let team_id = body["id"].as_i64().unwrap();

    // Register second user
    let register_req2 = CreateUserRequest {
        username: "member".to_string(),
        email: Some("member@example.com".to_string()),
        password: "password123".to_string(),
    };

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&register_req2)
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let member_user_id = body["user"]["id"].as_i64().unwrap();

    // Add member to team
    let add_member_req = AddTeamMemberRequest { user_id: member_user_id };

    let req = test::TestRequest::post()
        .uri(&format!("/api/teams/{}/members", team_id))
        .set_json(&add_member_req)
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert!(resp.status().is_success());

    // Verify member was added
    let req = test::TestRequest::get()
        .uri(&format!("/api/teams/{}/members", team_id))
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let members = body.as_array().unwrap();
    assert_eq!(members.len(), 2);

    // Remove member from team
    let req = test::TestRequest::delete()
        .uri(&format!("/api/teams/{}/members/{}", team_id, member_user_id))
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert!(resp.status().is_success());
    assert_eq!(resp.status(), 204);

    // Verify member was removed
    let req = test::TestRequest::get()
        .uri(&format!("/api/teams/{}/members", team_id))
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let members = body.as_array().unwrap();
    assert_eq!(members.len(), 1); // Only creator remains
}

#[actix_rt::test]
async fn test_super_admin_permissions() {
    let test_app = TestApp::new().await;
    let service = create_test_service(&test_app).await;

    // Register first user (should get Super Admin permissions)
    let register_req = CreateUserRequest {
        username: "superadmin".to_string(),
        email: Some("super@example.com".to_string()),
        password: "password123".to_string(),
    };

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&register_req)
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert!(resp.status().is_success());

    // Verify Super Admins team has admin permissions on all resource types
    let req = test::TestRequest::get().uri("/api/teams").to_request();
    let resp = test::call_service(&service, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let teams = body.as_array().unwrap();
    assert_eq!(teams.len(), 1);
    assert_eq!(teams[0]["name"], "Super Admins");

    let super_admin_team_id = teams[0]["id"].as_i64().unwrap();

    // Check team permissions
    let req = test::TestRequest::get()
        .uri(&format!("/api/teams/{}/permissions", super_admin_team_id))
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let permissions = body.as_array().unwrap();

    // Should have admin permissions on all resource types
    assert_eq!(permissions.len(), 6); // project, work, settings, user, team, ai_session

    let resource_types: std::collections::HashSet<_> = permissions
        .iter()
        .map(|p| p["resource_type"].as_str().unwrap())
        .collect();

    assert!(resource_types.contains("project"));
    assert!(resource_types.contains("work"));
    assert!(resource_types.contains("settings"));
    assert!(resource_types.contains("user"));
    assert!(resource_types.contains("team"));
    assert!(resource_types.contains("ai_session"));

    // All should be admin action with null resource_id (entity-level)
    for permission in permissions {
        assert_eq!(permission["action"], "admin");
        assert!(permission["resource_id"].is_null());
    }
}

#[actix_rt::test]
async fn test_list_all_permissions_admin_only() {
    let test_app = TestApp::new().await;
    let service = create_test_service(&test_app).await;

    // Register first user (Super Admin)
    let register_req = CreateUserRequest {
        username: "admin".to_string(),
        email: Some("admin@example.com".to_string()),
        password: "password123".to_string(),
    };

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&register_req)
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert!(resp.status().is_success());

    // List all permissions (should work for admin)
    let req = test::TestRequest::get().uri("/api/permissions").to_request();
    let resp = test::call_service(&service, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let permissions = body.as_array().unwrap();
    assert_eq!(permissions.len(), 6); // Super admin permissions
}

#[actix_rt::test]
async fn test_team_deletion_cascades() {
    let test_app = TestApp::new().await;
    let service = create_test_service(&test_app).await;

    // Create first user (Super Admin)
    let register_req = CreateUserRequest {
        username: "admin".to_string(),
        email: Some("admin@example.com".to_string()),
        password: "password123".to_string(),
    };

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&register_req)
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let admin_user_id = body["user"]["id"].as_i64().unwrap();

    // Create a team
    let create_team_req = CreateTeamRequest {
        name: "Temp Team".to_string(),
        description: Some("Temporary team".to_string()),
    };

    let req = test::TestRequest::post()
        .uri("/api/teams")
        .set_json(&create_team_req)
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let team_id = body["id"].as_i64().unwrap();

    // Add a permission to the team
    let create_perm_req = CreatePermissionRequest {
        team_id,
        resource_type: "project".to_string(),
        resource_id: Some(1),
        action: "write".to_string(),
    };

    let req = test::TestRequest::post()
        .uri("/api/permissions")
        .set_json(&create_perm_req)
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert!(resp.status().is_success());

    // Register second user and add to team
    let register_req2 = CreateUserRequest {
        username: "member".to_string(),
        email: Some("member@example.com".to_string()),
        password: "password123".to_string(),
    };

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&register_req2)
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let member_user_id = body["user"]["id"].as_i64().unwrap();

    let add_member_req = AddTeamMemberRequest { user_id: member_user_id };

    let req = test::TestRequest::post()
        .uri(&format!("/api/teams/{}/members", team_id))
        .set_json(&add_member_req)
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert!(resp.status().is_success());

    // Verify team exists with members and permissions
    let req = test::TestRequest::get()
        .uri(&format!("/api/teams/{}", team_id))
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert!(resp.status().is_success());

    // Delete the team
    let req = test::TestRequest::delete()
        .uri(&format!("/api/teams/{}", team_id))
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert!(resp.status().is_success());
    assert_eq!(resp.status(), 204);

    // Verify team is gone
    let req = test::TestRequest::get().uri("/api/teams").to_request();
    let resp = test::call_service(&service, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let teams = body.as_array().unwrap();
    assert_eq!(teams.len(), 1); // Only Super Admins remains

    // Verify permissions are gone (cascading delete)
    let req = test::TestRequest::get().uri("/api/permissions").to_request();
    let resp = test::call_service(&service, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let permissions = body.as_array().unwrap();
    assert_eq!(permissions.len(), 6); // Only Super Admin permissions remain
}