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
use nocodo_manager::models::{Permission, ResourceOwnership, Team, TeamMember, User};
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

    let member = TeamMember::new(team_id, user_id, Some(user_id));
    db.add_team_member(&member).unwrap();

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

    let member = TeamMember::new(team_id, user_id, Some(user_id));
    db.add_team_member(&member).unwrap();

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

    let member = TeamMember::new(team_id, user_id, Some(user_id));
    db.add_team_member(&member).unwrap();

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
    let member1 = TeamMember::new(team1_id, user_id, Some(user_id));
    db.add_team_member(&member1).unwrap();

    let member2 = TeamMember::new(team2_id, user_id, Some(user_id));
    db.add_team_member(&member2).unwrap();

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

    let member = TeamMember::new(team_id, user_id, Some(user_id));
    db.add_team_member(&member).unwrap();

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

    let member = TeamMember::new(team_id, user_id, Some(user_id));
    db.add_team_member(&member).unwrap();

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

    let member = TeamMember::new(team_id, user_id, Some(user_id));
    db.add_team_member(&member).unwrap();

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

    let member = TeamMember::new(team_id, user_id, Some(user_id));
    db.add_team_member(&member).unwrap();

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
