//! Permission checking system for team-based access control
//!
//! This module implements a hierarchical permission system where:
//! - Permissions are assigned to teams, not individual users
//! - Users inherit permissions from all teams they belong to
//! - Ownership grants automatic read/write/delete permissions
//! - Permissions can be resource-specific or entity-level (all resources of a type)

use crate::database::Database;
use crate::error::AppResult;
use serde::{Deserialize, Serialize};

/// Actions that can be performed on resources
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Read,
    Write,
    Delete,
    Admin,
}

impl Action {
    /// Convert action to string for database storage
    pub fn as_str(&self) -> &'static str {
        match self {
            Action::Read => "read",
            Action::Write => "write",
            Action::Delete => "delete",
            Action::Admin => "admin",
        }
    }

    /// Parse action from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "read" => Some(Action::Read),
            "write" => Some(Action::Write),
            "delete" => Some(Action::Delete),
            "admin" => Some(Action::Admin),
            _ => None,
        }
    }

    /// Check if this action implies another action
    /// - admin implies all actions
    /// - write implies read
    /// - delete implies read
    pub fn implies(&self, other: &Action) -> bool {
        match self {
            Action::Admin => true, // Admin implies everything
            Action::Write => matches!(other, Action::Read | Action::Write),
            Action::Delete => matches!(other, Action::Read | Action::Delete),
            Action::Read => matches!(other, Action::Read),
        }
    }
}

/// Resource types in the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResourceType {
    Project,
    Work,
    Settings,
    User,
    Team,
}

impl ResourceType {
    /// Convert resource type to string for database storage
    pub fn as_str(&self) -> &'static str {
        match self {
            ResourceType::Project => "project",
            ResourceType::Work => "work",
            ResourceType::Settings => "settings",
            ResourceType::User => "user",
            ResourceType::Team => "team",
        }
    }

    /// Parse resource type from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "project" => Some(ResourceType::Project),
            "work" => Some(ResourceType::Work),
            "settings" => Some(ResourceType::Settings),
            "user" => Some(ResourceType::User),
            "team" => Some(ResourceType::Team),
            _ => None,
        }
    }
}

/// Check if a user has permission to perform an action on a resource
///
/// Permission check priority order:
/// 1. Ownership - Owner gets read/write/delete automatically
/// 2. Resource-level - Check permission on specific resource (closest rule)
/// 3. Parent resource - Inherit from parent (files from project, child project from parent)
/// 4. Entity-level - Check permission on all resources of this type (resource_id = NULL)
pub async fn check_permission(
    db: &Database,
    user_id: i64,
    resource_type: ResourceType,
    resource_id: Option<i64>,
    action: Action,
) -> AppResult<bool> {
    // 1. Check ownership (automatic read/write/delete)
    if let Some(rid) = resource_id {
        if db.is_owner(user_id, resource_type.as_str(), rid)? {
            // Owners get read, write, delete but NOT admin
            if matches!(action, Action::Read | Action::Write | Action::Delete) {
                return Ok(true);
            }
        }
    }

    // 2. Check resource-specific permission (closest rule)
    if let Some(rid) = resource_id {
        if has_team_permission(db, user_id, resource_type, Some(rid), action).await? {
            return Ok(true);
        }
    }

    // 3. Check parent resource permission (inheritance)
    if let Some(rid) = resource_id {
        if check_parent_permission(db, user_id, resource_type, rid, action).await? {
            return Ok(true);
        }
    }

    // 4. Check entity-level permission (all resources of this type)
    if has_team_permission(db, user_id, resource_type, None, action).await? {
        return Ok(true);
    }

    Ok(false)
}

/// Check if user has permission through any of their teams
async fn has_team_permission(
    db: &Database,
    user_id: i64,
    resource_type: ResourceType,
    resource_id: Option<i64>,
    action: Action,
) -> AppResult<bool> {
    // Get all teams user belongs to
    let teams = db.get_user_teams(user_id)?;

    // Check if any team has the required permission
    for team in teams {
        if db.team_has_permission(
            team.id,
            resource_type.as_str(),
            resource_id,
            action.as_str(),
        )? {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Check if user has permission through parent resource (hierarchical projects)
fn check_parent_permission(
    db: &Database,
    user_id: i64,
    resource_type: ResourceType,
    resource_id: i64,
    action: Action,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = AppResult<bool>> + '_>> {
    Box::pin(async move {
        // Only projects support inheritance
        if resource_type != ResourceType::Project {
            return Ok(false);
        }

        // Get parent project ID
        if let Some(parent_id) = db.get_parent_project_id(resource_id)? {
            // Recursively check parent permission
            return check_permission(db, user_id, resource_type, Some(parent_id), action).await;
        }

        Ok(false)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_implies() {
        // Admin implies everything
        assert!(Action::Admin.implies(&Action::Read));
        assert!(Action::Admin.implies(&Action::Write));
        assert!(Action::Admin.implies(&Action::Delete));
        assert!(Action::Admin.implies(&Action::Admin));

        // Write implies read
        assert!(Action::Write.implies(&Action::Read));
        assert!(Action::Write.implies(&Action::Write));
        assert!(!Action::Write.implies(&Action::Delete));
        assert!(!Action::Write.implies(&Action::Admin));

        // Delete implies read
        assert!(Action::Delete.implies(&Action::Read));
        assert!(!Action::Delete.implies(&Action::Write));
        assert!(Action::Delete.implies(&Action::Delete));
        assert!(!Action::Delete.implies(&Action::Admin));

        // Read only implies read
        assert!(Action::Read.implies(&Action::Read));
        assert!(!Action::Read.implies(&Action::Write));
        assert!(!Action::Read.implies(&Action::Delete));
        assert!(!Action::Read.implies(&Action::Admin));
    }

    #[test]
    fn test_action_from_str() {
        assert_eq!(Action::from_str("read"), Some(Action::Read));
        assert_eq!(Action::from_str("write"), Some(Action::Write));
        assert_eq!(Action::from_str("delete"), Some(Action::Delete));
        assert_eq!(Action::from_str("admin"), Some(Action::Admin));
        assert_eq!(Action::from_str("READ"), Some(Action::Read)); // Case insensitive
        assert_eq!(Action::from_str("invalid"), None);
    }

    #[test]
    fn test_resource_type_from_str() {
        assert_eq!(
            ResourceType::from_str("project"),
            Some(ResourceType::Project)
        );
        assert_eq!(ResourceType::from_str("work"), Some(ResourceType::Work));
        assert_eq!(
            ResourceType::from_str("settings"),
            Some(ResourceType::Settings)
        );
        assert_eq!(ResourceType::from_str("user"), Some(ResourceType::User));
        assert_eq!(ResourceType::from_str("team"), Some(ResourceType::Team));
        assert_eq!(
            ResourceType::from_str("PROJECT"),
            Some(ResourceType::Project)
        ); // Case insensitive
        assert_eq!(ResourceType::from_str("invalid"), None);
    }
}
