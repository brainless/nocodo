//! Performance tests for the permission system
//!
//! Tests cover:
//! - Performance with many teams and permissions
//! - Query optimization for permission checks
//! - Scalability testing

use actix_web::{test, web, App};
use std::time::Instant;

use nocodo_manager::models::{CreateTeamRequest, AddTeamMemberRequest, CreatePermissionRequest, CreateUserRequest};

use crate::common::{TestApp, TestDataGenerator};

/// Macro to create app routes for performance tests
macro_rules! create_perf_test_routes {
    ($app:expr) => {
        $app
            // Auth endpoints
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
                            .route("/members", web::get().to(nocodo_manager::handlers::get_team_members))
                            .route("/permissions", web::get().to(nocodo_manager::handlers::get_team_permissions))
                            .service(
                                web::resource("/members")
                                    .route(web::post().to(nocodo_manager::handlers::add_team_member)),
                            ),
                    ),
            )
            // Permission management endpoints
            .service(
                web::scope("/api/permissions")
                    .route("", web::post().to(nocodo_manager::handlers::create_permission))
            )
    };
}

#[actix_rt::test]
async fn test_performance_many_teams_and_permissions() {
    let test_app = TestApp::new().await;
    let service = test::init_service(
        create_perf_test_routes!(App::new().app_data(test_app.app_state.clone()))
    ).await;

    let start_time = Instant::now();

    // 1. Register first user (Super Admin)
    let register_req = CreateUserRequest {
        username: "admin".to_string(),
        email: Some("admin@example.com".to_string()),
        password: "password123".to_string(),
        ssh_public_key: Some("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIGmJyR2T/DLSG6Q4Y5l2Hg test@example.com".to_string()),
        ssh_fingerprint: Some("SHA256:testfingerprint123".to_string()),
    };

    let req = test::TestRequest::post()
        .uri("/api/auth/register")
        .set_json(&register_req)
        .to_request();

    let resp = test::call_service(&service, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let admin_user_id = body["user"]["id"].as_i64().unwrap();

    println!("✅ User registration: {:?}", start_time.elapsed());

    // 2. Create 50 teams
    let mut team_ids = Vec::new();
    let team_creation_start = Instant::now();

    for i in 0..50 {
        let create_team_req = CreateTeamRequest {
            name: format!("Team {}", i),
            description: Some(format!("Performance test team {}", i)),
        };

        let req = test::TestRequest::post()
            .uri("/api/teams")
            .set_json(&create_team_req)
            .to_request();

        let resp = test::call_service(&service, req).await;
        assert!(resp.status().is_success());

        let body: serde_json::Value = test::read_body_json(resp).await;
        team_ids.push(body["id"].as_i64().unwrap());
    }

    println!("✅ Created 50 teams: {:?}", team_creation_start.elapsed());

    // 3. Register 100 users
    let mut user_ids = Vec::new();
    let user_registration_start = Instant::now();

    for i in 0..100 {
        let register_req = CreateUserRequest {
            username: format!("user{}", i),
            email: Some(format!("user{}@example.com", i)),
            password: "password123".to_string(),
            ssh_public_key: Some(format!("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIGmJyR2T/DLSG6Q4Y5l2Hg test{}@example.com", i)),
            ssh_fingerprint: Some(format!("SHA256:testfingerprint{}", i)),
        };

        let req = test::TestRequest::post()
            .uri("/api/auth/register")
            .set_json(&register_req)
            .to_request();

        let resp = test::call_service(&service, req).await;
        assert!(resp.status().is_success());

        let body: serde_json::Value = test::read_body_json(resp).await;
        user_ids.push(body["user"]["id"].as_i64().unwrap());
    }

    println!("✅ Registered 100 users: {:?}", user_registration_start.elapsed());

    // 4. Add users to teams (distribute users across teams)
    let member_assignment_start = Instant::now();
    let mut assignment_count = 0;

    for (i, &user_id) in user_ids.iter().enumerate() {
        let team_index = i % team_ids.len();
        let team_id = team_ids[team_index];

        let add_member_req = AddTeamMemberRequest { user_id };

        let req = test::TestRequest::post()
            .uri(&format!("/api/teams/{}/members", team_id))
            .set_json(&add_member_req)
            .to_request();

        let resp = test::call_service(&service, req).await;
        assert!(resp.status().is_success());
        assignment_count += 1;
    }

    println!("✅ Assigned {} team memberships: {:?}", assignment_count, member_assignment_start.elapsed());

    // 5. Create permissions for each team (200 permissions total)
    let permission_creation_start = Instant::now();
    let mut permission_count = 0;

    for &team_id in &team_ids {
        // Each team gets 4 permissions: read/write on projects, read/write on works
        let permissions = vec![
            ("project", Some(1), "read"),
            ("project", Some(1), "write"),
            ("work", None, "read"),
            ("work", None, "write"),
        ];

        for (resource_type, resource_id, action) in permissions {
            let create_perm_req = CreatePermissionRequest {
                team_id,
                resource_type: resource_type.to_string(),
                resource_id,
                action: action.to_string(),
            };

            let req = test::TestRequest::post()
                .uri("/api/permissions")
                .set_json(&create_perm_req)
                .to_request();

            let resp = test::call_service(&service, req).await;
            assert!(resp.status().is_success());
            permission_count += 1;
        }
    }

    println!("✅ Created {} permissions: {:?}", permission_count, permission_creation_start.elapsed());

    // 6. Performance test: List all teams
    let list_teams_start = Instant::now();

    for _ in 0..10 {
        let req = test::TestRequest::get().uri("/api/teams").to_request();
        let resp = test::call_service(&service, req).await;
        assert!(resp.status().is_success());

        let body: serde_json::Value = test::read_body_json(resp).await;
        let teams = body.as_array().unwrap();
        assert_eq!(teams.len(), 51); // 50 created + 1 Super Admins
    }

    println!("✅ 10 team listings (avg): {:?}", list_teams_start.elapsed() / 10);

    // 7. Performance test: Get team details with members and permissions
    let team_details_start = Instant::now();

    for &team_id in &team_ids[..5] { // Test first 5 teams
        // Get team details
        let req = test::TestRequest::get()
            .uri(&format!("/api/teams/{}", team_id))
            .to_request();

        let resp = test::call_service(&service, req).await;
        assert!(resp.status().is_success());

        // Get team members
        let req = test::TestRequest::get()
            .uri(&format!("/api/teams/{}/members", team_id))
            .to_request();

        let resp = test::call_service(&service, req).await;
        assert!(resp.status().is_success());

        // Get team permissions
        let req = test::TestRequest::get()
            .uri(&format!("/api/teams/{}/permissions", team_id))
            .to_request();

        let resp = test::call_service(&service, req).await;
        assert!(resp.status().is_success());
    }

    println!("✅ Team details queries (5 teams): {:?}", team_details_start.elapsed());

    // 8. Verify final state
    let final_check_start = Instant::now();

    // Count total teams
    let req = test::TestRequest::get().uri("/api/teams").to_request();
    let resp = test::call_service(&service, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let teams = body.as_array().unwrap();
    assert_eq!(teams.len(), 51); // 50 created + 1 Super Admins

    println!("✅ Final verification: {:?}", final_check_start.elapsed());
    println!("🎯 Total test time: {:?}", start_time.elapsed());
    println!("📊 Performance metrics:");
    println!("   - Teams: {}", teams.len());
    println!("   - Users: {}", user_ids.len());
    println!("   - Team memberships: {}", assignment_count);
    println!("   - Permissions: {}", permission_count);
}

#[actix_rt::test]
async fn test_permission_check_performance() {
    let test_app = TestApp::new().await;

    // Create test data directly in database for performance testing
    let admin_user_id = TestDataGenerator::create_test_user(&test_app.db()).await;

    // Create 20 teams
    let mut team_ids = Vec::new();
    for i in 0..20 {
        let team_id = TestDataGenerator::create_test_team(&test_app.db(), admin_user_id).await;
        team_ids.push(team_id);
    }

    // Create 50 users and distribute across teams
    let mut user_ids = Vec::new();
    for i in 0..50 {
        let user_id = TestDataGenerator::create_test_user(&test_app.db()).await;
        user_ids.push(user_id);

        // Add user to 2-3 random teams
        let num_teams = (i % 3) + 1; // 1-3 teams per user
        for j in 0..num_teams {
            let team_index = (i + j) % team_ids.len();
            let team_id = team_ids[team_index];
            test_app.db().add_team_member(team_id, user_id, Some(admin_user_id)).unwrap();
        }
    }

    // Create permissions for each team
    for &team_id in &team_ids {
        TestDataGenerator::create_test_permission(&test_app.db(), team_id, admin_user_id).await;
    }

    // Performance test: Check permissions for multiple users
    let check_start = Instant::now();
    let mut check_count = 0;

    for &user_id in &user_ids[..10] { // Test first 10 users
        for &team_id in &team_ids[..5] { // Check against first 5 teams
            // This simulates the permission checking that happens in middleware
            let teams = test_app.db().get_user_teams(user_id).unwrap();
            for team in teams {
                let has_perm = test_app.db()
                    .team_has_permission(team.id, "project", Some(1), "write")
                    .unwrap();
                check_count += 1;
            }
        }
    }

    let check_duration = check_start.elapsed();
    println!("✅ Permission checks ({} operations): {:?}", check_count, check_duration);
    println!("   - Average per check: {:?}", check_duration / check_count as u32);

    // The test passes if it completes within reasonable time
    // With proper indexing, this should be fast even with many teams/users
    assert!(check_duration.as_millis() < 1000, "Permission checks took too long: {:?}", check_duration);
}

#[actix_rt::test]
async fn test_database_query_performance() {
    let test_app = TestApp::new().await;

    // Create test data
    let admin_user_id = TestDataGenerator::create_test_user(&test_app.db()).await;

    // Create 25 teams
    let mut team_ids = Vec::new();
    for i in 0..25 {
        let team_id = TestDataGenerator::create_test_team(&test_app.db(), admin_user_id).await;
        team_ids.push(team_id);
    }

    // Create 40 users
    let mut user_ids = Vec::new();
    for i in 0..40 {
        let user_id = TestDataGenerator::create_test_user(&test_app.db()).await;
        user_ids.push(user_id);
    }

    // Distribute users across teams (each user in multiple teams)
    for (i, &user_id) in user_ids.iter().enumerate() {
        for j in 0..3 { // Each user in 3 teams
            let team_index = (i * 3 + j) % team_ids.len();
            let team_id = team_ids[team_index];
            test_app.db().add_team_member(team_id, user_id, Some(admin_user_id)).unwrap();
        }
    }

    // Create permissions
    for &team_id in &team_ids {
        TestDataGenerator::create_test_permission(&test_app.db(), team_id, admin_user_id).await;
    }

    // Test query performance
    let query_start = Instant::now();

    // Test get_user_teams for multiple users
    for &user_id in &user_ids[..10] {
        let _teams = test_app.db().get_user_teams(user_id).unwrap();
    }

    // Test get_team_members for multiple teams
    for &team_id in &team_ids[..10] {
        let _members = test_app.db().get_team_members(team_id).unwrap();
    }

    // Test get_team_permissions for multiple teams
    for &team_id in &team_ids[..10] {
        let _permissions = test_app.db().get_team_permissions(team_id).unwrap();
    }

    // Test get_all_permissions
    let _all_permissions = test_app.db().get_all_permissions().unwrap();

    let query_duration = query_start.elapsed();
    println!("✅ Database queries performance: {:?}", query_duration);
    println!("   - Tested with {} teams, {} users, {} memberships", team_ids.len(), user_ids.len(), user_ids.len() * 3);

    // Should complete within reasonable time
    assert!(query_duration.as_millis() < 500, "Database queries took too long: {:?}", query_duration);
}