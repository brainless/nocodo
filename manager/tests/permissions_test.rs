//! Unit tests for permission checking system
//!
//! Tests cover:
//! - Ownership grants automatic permissions
//! - Team permission inheritance
//! - Entity-level permissions
//! - Hierarchical project permission inheritance
//! - Action hierarchy (admin implies all, write implies read, etc.)
//! - Multiple team memberships
//! - Permission denial when no access

use nocodo_manager::database::Database;
use nocodo_manager::models::{Permission, ResourceOwnership, Team, User};
use nocodo_manager::permissions::{check_permission, Action, ResourceType};
use tempfile::TempDir;

/// Helper to create a test database
fn setup_test_db() -> (Database, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db = Database::new(&db_path).unwrap();
    (db, temp_dir)
}

/// Helper to create a test user
fn create_test_user(db: &Database, username: &str, email: &str) -> i64 {
    let user = User::new(
        username.to_string(),
        email.to_string(),
        "test_hash".to_string(),
    );
    db.create_user(&user).unwrap()
}

#[tokio::test]
async fn test_owner_has_automatic_permissions() {
    let (db, _temp) = setup_test_db();

    // Create user and project
    let user_id = create_test_user(&db, "alice", "alice@example.com");
    let project_id = 5;

    // Record ownership
    let ownership = ResourceOwnership::new("project".to_string(), project_id, user_id);
    db.create_ownership(&ownership).unwrap();

    // Owner should have read/write/delete
    assert!(check_permission(
        &db,
        user_id,
        ResourceType::Project,
        Some(project_id),
        Action::Read
    )
    .await
    .unwrap());
    assert!(check_permission(
        &db,
        user_id,
        ResourceType::Project,
        Some(project_id),
        Action::Write
    )
    .await
    .unwrap());
    assert!(check_permission(
        &db,
        user_id,
        ResourceType::Project,
        Some(project_id),
        Action::Delete
    )
    .await
    .unwrap());

    // But not admin (ownership doesn't grant admin)
    assert!(!check_permission(
        &db,
        user_id,
        ResourceType::Project,
        Some(project_id),
        Action::Admin
    )
    .await
    .unwrap());
}

#[tokio::test]
async fn test_team_member_inherits_permission() {
    let (db, _temp) = setup_test_db();

    let user_id = create_test_user(&db, "bob", "bob@example.com");
    let project_id = 5;

    // Create team and add user
    let team = Team::new(
        "Test Team".to_string(),
        Some("A test team".to_string()),
        user_id,
    );
    let team_id = db.create_team(&team).unwrap();

    db.add_team_member(team_id, user_id, Some(user_id)).unwrap();

    // Grant permission to team
    let permission = Permission::new(
        team_id,
        "project".to_string(),
        Some(project_id),
        "write".to_string(),
        Some(user_id),
    );
    db.create_permission(&permission).unwrap();

    // User should inherit permission from team
    assert!(check_permission(
        &db,
        user_id,
        ResourceType::Project,
        Some(project_id),
        Action::Write
    )
    .await
    .unwrap());
    // Write implies read
    assert!(check_permission(
        &db,
        user_id,
        ResourceType::Project,
        Some(project_id),
        Action::Read
    )
    .await
    .unwrap());
}

#[tokio::test]
async fn test_entity_level_permission() {
    let (db, _temp) = setup_test_db();

    let user_id = create_test_user(&db, "charlie", "charlie@example.com");

    // Create team and add user
    let team = Team::new(
        "Admins".to_string(),
        Some("Admin team".to_string()),
        user_id,
    );
    let team_id = db.create_team(&team).unwrap();

    db.add_team_member(team_id, user_id, Some(user_id)).unwrap();

    // Grant entity-level permission (resource_id = NULL)
    let permission = Permission::new(
        team_id,
        "project".to_string(),
        None, // ← All projects
        "admin".to_string(),
        Some(user_id),
    );
    db.create_permission(&permission).unwrap();

    // User should have access to ANY project
    assert!(
        check_permission(&db, user_id, ResourceType::Project, Some(1), Action::Admin)
            .await
            .unwrap()
    );
    assert!(check_permission(
        &db,
        user_id,
        ResourceType::Project,
        Some(999),
        Action::Admin
    )
    .await
    .unwrap());
}

#[tokio::test]
async fn test_action_hierarchy() {
    let (db, _temp) = setup_test_db();

    let user_id = create_test_user(&db, "diana", "diana@example.com");
    let project_id = 5;

    // Create team and add user
    let team = Team::new("Team".to_string(), None, user_id);
    let team_id = db.create_team(&team).unwrap();

    db.add_team_member(team_id, user_id, Some(user_id)).unwrap();

    // Grant ADMIN permission
    let permission = Permission::new(
        team_id,
        "project".to_string(),
        Some(project_id),
        "admin".to_string(),
        Some(user_id),
    );
    db.create_permission(&permission).unwrap();

    // Admin should imply all other actions
    assert!(check_permission(
        &db,
        user_id,
        ResourceType::Project,
        Some(project_id),
        Action::Admin
    )
    .await
    .unwrap());
    assert!(check_permission(
        &db,
        user_id,
        ResourceType::Project,
        Some(project_id),
        Action::Write
    )
    .await
    .unwrap());
    assert!(check_permission(
        &db,
        user_id,
        ResourceType::Project,
        Some(project_id),
        Action::Read
    )
    .await
    .unwrap());
    assert!(check_permission(
        &db,
        user_id,
        ResourceType::Project,
        Some(project_id),
        Action::Delete
    )
    .await
    .unwrap());
}

#[tokio::test]
async fn test_no_permission_denied() {
    let (db, _temp) = setup_test_db();

    let user_id = create_test_user(&db, "eve", "eve@example.com");
    let project_id = 5;

    // User exists but has no teams or permissions

    // Should be denied
    assert!(!check_permission(
        &db,
        user_id,
        ResourceType::Project,
        Some(project_id),
        Action::Read
    )
    .await
    .unwrap());
}

#[tokio::test]
async fn test_multiple_team_memberships() {
    let (db, _temp) = setup_test_db();

    let user_id = create_test_user(&db, "frank", "frank@example.com");
    let project_id = 5;

    // Create two teams
    let team1 = Team::new("Team 1".to_string(), None, user_id);
    let team1_id = db.create_team(&team1).unwrap();

    let team2 = Team::new("Team 2".to_string(), None, user_id);
    let team2_id = db.create_team(&team2).unwrap();

    // Add user to both teams
    db.add_team_member(team1_id, user_id, Some(user_id)).unwrap();
    db.add_team_member(team2_id, user_id, Some(user_id)).unwrap();

    // Grant different permissions to each team
    let permission1 = Permission::new(
        team1_id,
        "project".to_string(),
        Some(project_id),
        "read".to_string(),
        Some(user_id),
    );
    db.create_permission(&permission1).unwrap();

    let permission2 = Permission::new(
        team2_id,
        "project".to_string(),
        Some(project_id),
        "write".to_string(),
        Some(user_id),
    );
    db.create_permission(&permission2).unwrap();

    // User should have the highest permission from any team (write)
    assert!(check_permission(
        &db,
        user_id,
        ResourceType::Project,
        Some(project_id),
        Action::Write
    )
    .await
    .unwrap());
    assert!(check_permission(
        &db,
        user_id,
        ResourceType::Project,
        Some(project_id),
        Action::Read
    )
    .await
    .unwrap());
}

#[tokio::test]
async fn test_write_implies_read() {
    let (db, _temp) = setup_test_db();

    let user_id = create_test_user(&db, "grace", "grace@example.com");
    let project_id = 5;

    // Create team and add user
    let team = Team::new("Writers".to_string(), None, user_id);
    let team_id = db.create_team(&team).unwrap();

    db.add_team_member(team_id, user_id, Some(user_id)).unwrap();

    // Grant WRITE permission
    let permission = Permission::new(
        team_id,
        "project".to_string(),
        Some(project_id),
        "write".to_string(),
        Some(user_id),
    );
    db.create_permission(&permission).unwrap();

    // Write should imply read
    assert!(check_permission(
        &db,
        user_id,
        ResourceType::Project,
        Some(project_id),
        Action::Write
    )
    .await
    .unwrap());
    assert!(check_permission(
        &db,
        user_id,
        ResourceType::Project,
        Some(project_id),
        Action::Read
    )
    .await
    .unwrap());
    // But not delete or admin
    assert!(!check_permission(
        &db,
        user_id,
        ResourceType::Project,
        Some(project_id),
        Action::Delete
    )
    .await
    .unwrap());
    assert!(!check_permission(
        &db,
        user_id,
        ResourceType::Project,
        Some(project_id),
        Action::Admin
    )
    .await
    .unwrap());
}

#[tokio::test]
async fn test_delete_implies_read() {
    let (db, _temp) = setup_test_db();

    let user_id = create_test_user(&db, "henry", "henry@example.com");
    let project_id = 5;

    // Create team and add user
    let team = Team::new("Deleters".to_string(), None, user_id);
    let team_id = db.create_team(&team).unwrap();

    db.add_team_member(team_id, user_id, Some(user_id)).unwrap();

    // Grant DELETE permission
    let permission = Permission::new(
        team_id,
        "project".to_string(),
        Some(project_id),
        "delete".to_string(),
        Some(user_id),
    );
    db.create_permission(&permission).unwrap();

    // Delete should imply read
    assert!(check_permission(
        &db,
        user_id,
        ResourceType::Project,
        Some(project_id),
        Action::Delete
    )
    .await
    .unwrap());
    assert!(check_permission(
        &db,
        user_id,
        ResourceType::Project,
        Some(project_id),
        Action::Read
    )
    .await
    .unwrap());
    // But not write or admin
    assert!(!check_permission(
        &db,
        user_id,
        ResourceType::Project,
        Some(project_id),
        Action::Write
    )
    .await
    .unwrap());
    assert!(!check_permission(
        &db,
        user_id,
        ResourceType::Project,
        Some(project_id),
        Action::Admin
    )
    .await
    .unwrap());
}

#[tokio::test]
async fn test_resource_specific_vs_entity_level() {
    let (db, _temp) = setup_test_db();

    let user_id = create_test_user(&db, "iris", "iris@example.com");
    let project_id = 5;

    // Create team and add user
    let team = Team::new("Team".to_string(), None, user_id);
    let team_id = db.create_team(&team).unwrap();

    db.add_team_member(team_id, user_id, Some(user_id)).unwrap();

    // Grant entity-level read permission
    let permission1 = Permission::new(
        team_id,
        "project".to_string(),
        None, // All projects
        "read".to_string(),
        Some(user_id),
    );
    db.create_permission(&permission1).unwrap();

    // Grant resource-specific write permission
    let permission2 = Permission::new(
        team_id,
        "project".to_string(),
        Some(project_id), // Specific project
        "write".to_string(),
        Some(user_id),
    );
    db.create_permission(&permission2).unwrap();

    // Should have write on specific project
    assert!(check_permission(
        &db,
        user_id,
        ResourceType::Project,
        Some(project_id),
        Action::Write
    )
    .await
    .unwrap());

    // Should have only read on other projects
    assert!(
        check_permission(&db, user_id, ResourceType::Project, Some(999), Action::Read)
            .await
            .unwrap()
    );
    assert!(!check_permission(
        &db,
        user_id,
        ResourceType::Project,
        Some(999),
        Action::Write
    )
    .await
    .unwrap());
}

#[tokio::test]
async fn test_non_existent_user() {
    let (db, _temp) = setup_test_db();

    let non_existent_user_id = 99999;
    let project_id = 5;

    // Should be denied
    assert!(!check_permission(
        &db,
        non_existent_user_id,
        ResourceType::Project,
        Some(project_id),
        Action::Read
    )
    .await
    .unwrap());
}

#[tokio::test]
async fn test_team_deletion_cascades_permissions() {
    let (db, _temp) = setup_test_db();

    let user_id = create_test_user(&db, "judy", "judy@example.com");
    let project_id = 5;

    // Create team and add user
    let team = Team::new("Temp Team".to_string(), None, user_id);
    let team_id = db.create_team(&team).unwrap();

    db.add_team_member(team_id, user_id, Some(user_id)).unwrap();

    // Grant permission
    let permission = Permission::new(
        team_id,
        "project".to_string(),
        Some(project_id),
        "write".to_string(),
        Some(user_id),
    );
    db.create_permission(&permission).unwrap();

    // User should have permission
    assert!(check_permission(
        &db,
        user_id,
        ResourceType::Project,
        Some(project_id),
        Action::Write
    )
    .await
    .unwrap());

    // Delete the team
    db.delete_team(team_id).unwrap();

    // User should no longer have permission
    assert!(!check_permission(
        &db,
        user_id,
        ResourceType::Project,
        Some(project_id),
        Action::Write
    )
    .await
    .unwrap());
}

#[tokio::test]
async fn test_hierarchical_project_permission_inheritance() {
    let (db, _temp) = setup_test_db();

    let user_id = create_test_user(&db, "kevin", "kevin@example.com");
    let parent_project_id = 5;
    let child_project_id = 10;

    // Create parent project
    let parent_project = nocodo_manager::models::Project {
        id: parent_project_id,
        name: "Parent Project".to_string(),
        path: "/tmp/test/parent".to_string(),
        description: Some("Parent project".to_string()),
        parent_id: None,
        created_at: chrono::Utc::now().timestamp(),
        updated_at: chrono::Utc::now().timestamp(),
    };
    db.create_project(&parent_project).unwrap();

    // Create child project with parent_id = 5
    let child_project = nocodo_manager::models::Project {
        id: child_project_id,
        name: "Child Project".to_string(),
        path: "/tmp/test/child".to_string(),
        description: Some("Child project".to_string()),
        parent_id: Some(parent_project_id),
        created_at: chrono::Utc::now().timestamp(),
        updated_at: chrono::Utc::now().timestamp(),
    };
    db.create_project(&child_project).unwrap();

    // Create team and add user
    let team = Team::new("Team".to_string(), None, user_id);
    let team_id = db.create_team(&team).unwrap();
    db.add_team_member(team_id, user_id, Some(user_id)).unwrap();

    // Grant permission on PARENT project only
    let permission = Permission::new(
        team_id,
        "project".to_string(),
        Some(parent_project_id),
        "write".to_string(),
        Some(user_id),
    );
    db.create_permission(&permission).unwrap();

    // User should have access to PARENT project
    assert!(check_permission(
        &db,
        user_id,
        ResourceType::Project,
        Some(parent_project_id),
        Action::Write
    )
    .await
    .unwrap());

    // User should have access to CHILD project (inheritance)
    assert!(check_permission(
        &db,
        user_id,
        ResourceType::Project,
        Some(child_project_id),
        Action::Write
    )
    .await
    .unwrap());

    // Both should imply read
    assert!(check_permission(
        &db,
        user_id,
        ResourceType::Project,
        Some(parent_project_id),
        Action::Read
    )
    .await
    .unwrap());
    assert!(check_permission(
        &db,
        user_id,
        ResourceType::Project,
        Some(child_project_id),
        Action::Read
    )
    .await
    .unwrap());
}

#[tokio::test]
async fn test_ownership_deleted_with_resource() {
    let (db, _temp) = setup_test_db();

    let user_id = create_test_user(&db, "lisa", "lisa@example.com");
    let project_id = 15;

    // Create project
    let project = nocodo_manager::models::Project {
        id: project_id,
        name: "Test Project".to_string(),
        path: "/tmp/test/project15".to_string(),
        description: None,
        parent_id: None,
        created_at: chrono::Utc::now().timestamp(),
        updated_at: chrono::Utc::now().timestamp(),
    };
    db.create_project(&project).unwrap();

    // Create ownership record
    let ownership = ResourceOwnership::new("project".to_string(), project_id, user_id);
    db.create_ownership(&ownership).unwrap();

    // Verify ownership exists
    assert!(db.is_owner(user_id, "project", project_id).unwrap());

    // Delete the project
    db.delete_project(project_id).unwrap();

    // Verify ownership record is gone (is_owner should return false)
    assert!(!db.is_owner(user_id, "project", project_id).unwrap());
}

#[test]
fn test_bootstrap_first_user_creates_super_admin_team() {
    let (db, _temp) = setup_test_db();

    // Verify no users exist initially
    assert_eq!(db.get_all_users().unwrap().len(), 0);

    // Create first user (simulating registration)
    let first_user = User {
        id: 0,
        username: "admin".to_string(),
        email: "admin@example.com".to_string(),
        password_hash: "hashed_password".to_string(),
        is_active: true,
        created_at: chrono::Utc::now().timestamp(),
        updated_at: chrono::Utc::now().timestamp(),
    };

    let user_id = db.create_user(&first_user).unwrap();

    // Simulate bootstrap logic (normally done in register handler)
    let user_count = db.get_all_users().unwrap().len();
    assert_eq!(user_count, 1); // Should be 1 now

    // Create "Super Admins" team
    let super_admin_team = Team {
        id: 0,
        name: "Super Admins".to_string(),
        description: Some("System administrators with full access".to_string()),
        created_by: user_id,
        created_at: chrono::Utc::now().timestamp(),
        updated_at: chrono::Utc::now().timestamp(),
    };

    let team_id = db.create_team(&super_admin_team).unwrap();

    // Add first user to the team
    db.add_team_member(team_id, user_id, Some(user_id)).unwrap();

    // Grant entity-level admin permissions on all resource types
    let resource_types = ["project", "work", "settings", "user", "team", "ai_session"];
    for resource_type in &resource_types {
        let permission = Permission {
            id: 0,
            team_id,
            resource_type: resource_type.to_string(),
            resource_id: None, // Entity-level (all resources of this type)
            action: "admin".to_string(),
            granted_by: Some(user_id),
            granted_at: chrono::Utc::now().timestamp(),
        };
        db.create_permission(&permission).unwrap();
    }

    // Verify the bootstrap worked
    let teams = db.get_user_teams(user_id).unwrap();
    assert_eq!(teams.len(), 1);
    assert_eq!(teams[0].name, "Super Admins");

    let team_permissions = db.get_team_permissions(team_id).unwrap();
    assert_eq!(team_permissions.len(), 6); // One permission per resource type

    // Verify admin permissions work
    assert!(db.team_has_permission(team_id, "project", None, "admin").unwrap());
    assert!(db.team_has_permission(team_id, "user", None, "admin").unwrap());
    assert!(db.team_has_permission(team_id, "settings", None, "admin").unwrap());
}
