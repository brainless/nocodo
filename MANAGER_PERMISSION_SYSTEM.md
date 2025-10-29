# nocodo Manager Permission System

## Overview

Team-based access control system for fine-grained resource permissions. All permissions are assigned to **teams**, not individual users. Users inherit permissions from all teams they belong to.

**Philosophy**: Keep it simple - teams with 1 user are perfectly fine (e.g., "Super Admins" team).

---

## Core Concepts

### 1. Teams
Groups of users that share the same permissions. Examples:
- "Super Admins" (1 user)
- "Frontend Developers" (5 users)
- "Project Alpha Team" (3 users)

### 2. Permissions
Access rules assigned to teams, specifying what actions can be performed on which resources.

### 3. Resource Types
```
- project      (CRUD projects)
- work         (CRUD work sessions)
- settings     (read, update system settings)
- user         (CRUD users - admin only)
- team         (CRUD teams - admin only)
```

### 4. Actions
```
- read         (view resource)
- write        (create, update resource)
- delete       (remove resource)
- admin        (full control, implies all other actions)
```

### 5. Permission Scope
- **Resource-specific**: `resource_id = 5` → Access to Project #5 only
- **Entity-level**: `resource_id = NULL` → Access to all projects

### 6. Ownership
Resource creators automatically become owners and get read/write/delete permissions.

---

## Database Schema

### Tables

```sql
-- 1. Teams
CREATE TABLE teams (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    created_by INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (created_by) REFERENCES users (id) ON DELETE SET NULL
);

-- 2. Team Members
CREATE TABLE team_members (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    team_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    added_by INTEGER,
    added_at INTEGER NOT NULL,
    FOREIGN KEY (team_id) REFERENCES teams (id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE,
    FOREIGN KEY (added_by) REFERENCES users (id) ON DELETE SET NULL,
    UNIQUE(team_id, user_id)
);

-- 3. Permissions (team-only)
CREATE TABLE permissions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    team_id INTEGER NOT NULL,
    resource_type TEXT NOT NULL,  -- 'project', 'work', 'settings', 'user', 'team'
    resource_id INTEGER,          -- NULL = all resources of this type
    action TEXT NOT NULL,         -- 'read', 'write', 'delete', 'admin'
    granted_by INTEGER,
    granted_at INTEGER NOT NULL,
    FOREIGN KEY (team_id) REFERENCES teams (id) ON DELETE CASCADE,
    FOREIGN KEY (granted_by) REFERENCES users (id) ON DELETE SET NULL
);

-- 4. Resource Ownership
CREATE TABLE resource_ownership (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    resource_type TEXT NOT NULL,
    resource_id INTEGER NOT NULL,
    owner_id INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (owner_id) REFERENCES users (id) ON DELETE CASCADE,
    UNIQUE(resource_type, resource_id)
);

-- Indexes
CREATE INDEX idx_teams_created_by ON teams(created_by);
CREATE INDEX idx_team_members_team_id ON team_members(team_id);
CREATE INDEX idx_team_members_user_id ON team_members(user_id);
CREATE INDEX idx_permissions_team_id ON permissions(team_id);
CREATE INDEX idx_permissions_resource ON permissions(resource_type, resource_id);
CREATE INDEX idx_resource_ownership_owner ON resource_ownership(owner_id);
CREATE INDEX idx_resource_ownership_resource ON resource_ownership(resource_type, resource_id);
```

---

## Permission Check Algorithm

### Priority Order

```
1. Ownership        → Owner gets read/write/delete automatically
2. Resource-level   → Check permission on specific resource (closest rule)
3. Parent resource  → Inherit from parent (files from project, child project from parent)
4. Entity-level     → Check permission on all resources of this type (resource_id = NULL)
```

### Pseudocode

```rust
fn check_permission(user_id, resource_type, resource_id, action) -> bool {
    // 1. Check ownership (automatic read/write/delete)
    if resource_id.is_some() && is_owner(user_id, resource_type, resource_id) {
        if action in [read, write, delete] {
            return true;
        }
    }

    // 2. Check resource-specific permission (closest rule)
    if resource_id.is_some() && has_team_permission(user_id, resource_type, resource_id, action) {
        return true;
    }

    // 3. Check parent resource permission (inheritance)
    if resource_id.is_some() {
        if let Some(parent_allowed) = check_parent_permission(user_id, resource_type, resource_id, action) {
            if parent_allowed {
                return true;
            }
        }
    }

    // 4. Check entity-level permission (all resources of this type)
    if has_team_permission(user_id, resource_type, None, action) {
        return true;
    }

    return false;
}

fn has_team_permission(user_id, resource_type, resource_id, action) -> bool {
    // Get all teams user belongs to
    teams = get_user_teams(user_id);

    // Check if any team has the required permission
    for team in teams {
        if team_has_permission(team.id, resource_type, resource_id, action) {
            return true;
        }
    }

    return false;
}
```

### Action Hierarchy

Actions have implied permissions:
- `admin` → `read`, `write`, `delete`
- `write` → `read`
- `delete` → `read`

---

## Inheritance Rules

### Hierarchical Projects
Projects can have parent projects (`parent_id` field):

```rust
// User tries to access project #10 (child of project #5)
check_permission(user_id, "project", 10, "write")
  → Check ownership of project #10
  → Check permission on project #10
  → Check permission on parent project #5  ← INHERITANCE
  → Check entity-level permission on all projects
```

### Work Sessions
Works are **independent** - they do NOT inherit permissions from their parent project.

```rust
// User tries to access work #7 (linked to project #5)
check_permission(user_id, "work", 7, "read")
  → Check ownership of work #7
  → Check permission on work #7
  → Check entity-level permission on all works
  → NO inheritance from project
```

---

## Special Cases

### Settings (Global Resources)
Settings always use `resource_id = NULL`:

```rust
// Check settings admin permission
check_permission(user_id, "settings", None, "admin")
```

### First User (Bootstrap)
The first registered user automatically gets a "Super Admins" team with entity-level admin permissions on everything:

```sql
-- Create "Super Admins" team
INSERT INTO teams (name, description, created_by) VALUES
    ('Super Admins', 'System administrators with full access', first_user_id);

-- Add first user to team
INSERT INTO team_members (team_id, user_id, added_by) VALUES
    (super_admin_team_id, first_user_id, first_user_id);

-- Grant entity-level admin permissions
INSERT INTO permissions (team_id, resource_type, resource_id, action, granted_by) VALUES
    (super_admin_team_id, 'project', NULL, 'admin', first_user_id),
    (super_admin_team_id, 'work', NULL, 'admin', first_user_id),
    (super_admin_team_id, 'settings', NULL, 'admin', first_user_id),
    (super_admin_team_id, 'user', NULL, 'admin', first_user_id),
    (super_admin_team_id, 'team', NULL, 'admin', first_user_id);
```

### Resource Creation (Automatic Ownership)
When a user creates a resource, they automatically become the owner:

```rust
// User creates project
let project_id = db.create_project(&project, user_id).await?;

// Automatically record ownership
db.create_ownership(ResourceOwnership {
    resource_type: "project",
    resource_id: project_id,
    owner_id: user_id,
}).await?;

// Owner automatically gets read/write/delete
// check_permission(user_id, "project", project_id, "write") → true
```

---

## Usage Examples

### Example 1: Create a Team with Project Access

```rust
// Admin creates "Frontend Team"
let team_id = db.create_team(Team {
    name: "Frontend Team",
    description: "Frontend developers",
    created_by: admin_user_id,
}).await?;

// Add members
db.add_team_member(team_id, alice_id, admin_user_id).await?;
db.add_team_member(team_id, bob_id, admin_user_id).await?;

// Grant team write access to Project #5
db.create_permission(Permission {
    team_id: team_id,
    resource_type: "project",
    resource_id: Some(5),
    action: "write",
    granted_by: admin_user_id,
}).await?;

// Result: Alice and Bob can now read/write Project #5
```

### Example 2: Entity-Level Permission (All Projects)

```rust
// Create "Developers" team
let dev_team_id = db.create_team(Team {
    name: "Developers",
    description: "All developers",
    created_by: admin_user_id,
}).await?;

// Grant entity-level permission (resource_id = NULL)
db.create_permission(Permission {
    team_id: dev_team_id,
    resource_type: "project",
    resource_id: None,  // ← NULL = all projects
    action: "write",
    granted_by: admin_user_id,
}).await?;

// Result: All members can create projects and access all existing projects
```

### Example 3: Single-User Admin Team

```rust
// Create "Super Admins" team with 1 member
let admin_team_id = db.create_team(Team {
    name: "Super Admins",
    description: "System administrators",
    created_by: first_user_id,
}).await?;

db.add_team_member(admin_team_id, first_user_id, first_user_id).await?;

// Grant admin on all resource types
for resource_type in ["project", "work", "settings", "user", "team"] {
    db.create_permission(Permission {
        team_id: admin_team_id,
        resource_type: resource_type,
        resource_id: None,
        action: "admin",
        granted_by: first_user_id,
    }).await?;
}

// Result: First user has full system access
```

### Example 4: Temporary Project Access

```rust
// Create single-user team for contractor
let contractor_team_id = db.create_team(Team {
    name: "John Contractor",
    description: "Temporary access for John",
    created_by: admin_user_id,
}).await?;

db.add_team_member(contractor_team_id, john_id, admin_user_id).await?;

// Grant read-only access to specific project
db.create_permission(Permission {
    team_id: contractor_team_id,
    resource_type: "project",
    resource_id: Some(10),
    action: "read",
    granted_by: admin_user_id,
}).await?;

// When contract ends, simply remove John from team or delete the team
```

---

## Implementation Tasks

### Phase 1: Database & Models
- [ ] **Task 1.1**: Add database migrations for 4 new tables
  - `teams`
  - `team_members`
  - `permissions`
  - `resource_ownership`
- [ ] **Task 1.2**: Create Rust models
  - `Team` struct
  - `TeamMember` struct
  - `Permission` struct
  - `ResourceOwnership` struct
- [ ] **Task 1.3**: Add database methods
  - Team CRUD operations
  - Team member management
  - Permission CRUD operations
  - Ownership tracking

### Phase 2: Permission Checking
- [ ] **Task 2.1**: Create `manager/src/permissions.rs` module
  - `Action` enum
  - `ResourceType` enum
  - `check_permission()` function
  - `has_team_permission()` helper
  - `check_parent_permission()` helper
  - Action hierarchy logic
- [ ] **Task 2.2**: Write unit tests for permission checking
  - Test ownership permissions
  - Test resource-level permissions
  - Test entity-level permissions
  - Test inheritance (hierarchical projects)
  - Test action hierarchy (admin implies write)
  - Test multiple team memberships
- [ ] **Task 2.3**: Add permission check methods to Database
  - `get_user_teams(user_id)`
  - `team_has_permission(team_id, resource_type, resource_id, action)`
  - `is_owner(user_id, resource_type, resource_id)`
  - `get_parent_project_id(resource_type, resource_id)`

### Phase 3: Middleware & Enforcement
- [ ] **Task 3.1**: Create authentication middleware
  - Extract JWT token from request
  - Validate token
  - Attach user_id to request context
- [ ] **Task 3.2**: Create permission enforcement middleware
  - Extract resource_type, resource_id, action from route
  - Call `check_permission()`
  - Return 403 Forbidden if denied
- [ ] **Task 3.3**: Add permission checks to all endpoints
  - Projects: create, read, update, delete
  - Works: create, read, delete
  - Settings: read, update
  - Users: CRUD (admin only)
  - Teams: CRUD

### Phase 4: Ownership Tracking
- [ ] **Task 4.1**: Add ownership tracking on resource creation
  - Projects: record owner on create
  - Works: record owner on create
- [ ] **Task 4.2**: Handle resource deletion
  - Cascade delete ownership records
  - Clean up permissions referencing deleted resources

### Phase 5: Bootstrap & Registration
- [ ] **Task 5.1**: First user registration logic
  - Check if user count == 0
  - Create "Super Admins" team
  - Add user to team
  - Grant entity-level admin permissions
- [ ] **Task 5.2**: User registration endpoint
  - Create user account
  - Do NOT auto-assign permissions (admin must add to team)
  - Optional: require admin approval

### Phase 6: Team Management APIs
- [ ] **Task 6.1**: Team CRUD endpoints
  - `POST /api/teams` - Create team
  - `GET /api/teams` - List teams
  - `GET /api/teams/{id}` - Get team details with members
  - `PUT /api/teams/{id}` - Update team
  - `DELETE /api/teams/{id}` - Delete team
- [ ] **Task 6.2**: Team member management endpoints
  - `POST /api/teams/{id}/members` - Add user to team
  - `DELETE /api/teams/{id}/members/{user_id}` - Remove user from team
  - `GET /api/teams/{id}/members` - List team members
- [ ] **Task 6.3**: Permission management endpoints
  - `POST /api/permissions` - Grant permission to team
  - `GET /api/permissions` - List all permissions (admin only)
  - `GET /api/teams/{id}/permissions` - List team permissions
  - `DELETE /api/permissions/{id}` - Revoke permission

### Phase 7: Integration Testing
- [ ] **Task 7.1**: End-to-end permission tests
  - Create team, add members, grant permissions
  - Test permission checks across different resources
  - Test inheritance scenarios
  - Test permission revocation
- [ ] **Task 7.2**: Performance testing
  - Test with many teams and permissions
  - Optimize queries if needed

---

## Unit Test Examples

### Test 1: Ownership Grants Automatic Permissions

```rust
#[test]
async fn test_owner_has_automatic_permissions() {
    let db = setup_test_db().await;

    // Create user and project
    let user_id = 1;
    let project_id = 5;

    // Record ownership
    db.create_ownership(ResourceOwnership {
        resource_type: "project".to_string(),
        resource_id: project_id,
        owner_id: user_id,
    }).await.unwrap();

    // Owner should have read/write/delete
    assert!(check_permission(&db, user_id, "project", Some(project_id), Action::Read).await.unwrap());
    assert!(check_permission(&db, user_id, "project", Some(project_id), Action::Write).await.unwrap());
    assert!(check_permission(&db, user_id, "project", Some(project_id), Action::Delete).await.unwrap());

    // But not admin (ownership doesn't grant admin)
    assert!(!check_permission(&db, user_id, "project", Some(project_id), Action::Admin).await.unwrap());
}
```

### Test 2: Team Permission Inheritance

```rust
#[test]
async fn test_team_member_inherits_permission() {
    let db = setup_test_db().await;

    let user_id = 1;
    let team_id = 10;
    let project_id = 5;

    // Create team and add user
    db.create_team(Team { id: team_id, name: "Test Team".to_string(), ... }).await.unwrap();
    db.add_team_member(team_id, user_id, None).await.unwrap();

    // Grant permission to team
    db.create_permission(Permission {
        team_id,
        resource_type: "project".to_string(),
        resource_id: Some(project_id),
        action: "write".to_string(),
        ...
    }).await.unwrap();

    // User should inherit permission from team
    assert!(check_permission(&db, user_id, "project", Some(project_id), Action::Write).await.unwrap());
    assert!(check_permission(&db, user_id, "project", Some(project_id), Action::Read).await.unwrap()); // Write implies read
}
```

### Test 3: Entity-Level Permission

```rust
#[test]
async fn test_entity_level_permission() {
    let db = setup_test_db().await;

    let user_id = 1;
    let team_id = 10;

    db.create_team(Team { id: team_id, name: "Admins".to_string(), ... }).await.unwrap();
    db.add_team_member(team_id, user_id, None).await.unwrap();

    // Grant entity-level permission (resource_id = NULL)
    db.create_permission(Permission {
        team_id,
        resource_type: "project".to_string(),
        resource_id: None,  // ← All projects
        action: "admin".to_string(),
        ...
    }).await.unwrap();

    // User should have access to ANY project
    assert!(check_permission(&db, user_id, "project", Some(1), Action::Admin).await.unwrap());
    assert!(check_permission(&db, user_id, "project", Some(999), Action::Admin).await.unwrap());
}
```

### Test 4: Hierarchical Project Permission Inheritance

```rust
#[test]
async fn test_hierarchical_project_permission() {
    let db = setup_test_db().await;

    let user_id = 1;
    let team_id = 10;
    let parent_project_id = 5;
    let child_project_id = 10;

    // Setup: child project has parent_id = 5
    db.create_project(Project { id: child_project_id, parent_id: Some(parent_project_id), ... }).await.unwrap();

    db.create_team(Team { id: team_id, name: "Team".to_string(), ... }).await.unwrap();
    db.add_team_member(team_id, user_id, None).await.unwrap();

    // Grant permission on PARENT project only
    db.create_permission(Permission {
        team_id,
        resource_type: "project".to_string(),
        resource_id: Some(parent_project_id),
        action: "write".to_string(),
        ...
    }).await.unwrap();

    // User should have access to CHILD project (inheritance)
    assert!(check_permission(&db, user_id, "project", Some(child_project_id), Action::Write).await.unwrap());
}
```

### Test 5: Action Hierarchy

```rust
#[test]
async fn test_action_hierarchy() {
    let db = setup_test_db().await;

    let user_id = 1;
    let team_id = 10;
    let project_id = 5;

    db.create_team(Team { id: team_id, name: "Team".to_string(), ... }).await.unwrap();
    db.add_team_member(team_id, user_id, None).await.unwrap();

    // Grant ADMIN permission
    db.create_permission(Permission {
        team_id,
        resource_type: "project".to_string(),
        resource_id: Some(project_id),
        action: "admin".to_string(),
        ...
    }).await.unwrap();

    // Admin should imply all other actions
    assert!(check_permission(&db, user_id, "project", Some(project_id), Action::Admin).await.unwrap());
    assert!(check_permission(&db, user_id, "project", Some(project_id), Action::Write).await.unwrap());
    assert!(check_permission(&db, user_id, "project", Some(project_id), Action::Read).await.unwrap());
    assert!(check_permission(&db, user_id, "project", Some(project_id), Action::Delete).await.unwrap());
}
```

### Test 6: No Permission Denied

```rust
#[test]
async fn test_no_permission_denied() {
    let db = setup_test_db().await;

    let user_id = 1;
    let project_id = 5;

    // User exists but has no teams or permissions

    // Should be denied
    assert!(!check_permission(&db, user_id, "project", Some(project_id), Action::Read).await.unwrap());
}
```

---

## API Endpoints (To Be Implemented)

### Team Management
```
POST   /api/teams                     - Create team
GET    /api/teams                     - List teams
GET    /api/teams/{id}                - Get team details
PUT    /api/teams/{id}                - Update team
DELETE /api/teams/{id}                - Delete team

POST   /api/teams/{id}/members        - Add member to team
DELETE /api/teams/{id}/members/{uid}  - Remove member from team
GET    /api/teams/{id}/members        - List team members
```

### Permission Management
```
POST   /api/permissions               - Grant permission to team
GET    /api/permissions               - List all permissions (admin)
GET    /api/teams/{id}/permissions    - List team permissions
DELETE /api/permissions/{id}          - Revoke permission
```

### User Management (Admin Only)
```
POST   /api/users                     - Create user (admin)
GET    /api/users                     - List users (admin)
GET    /api/users/{id}                - Get user details (admin)
PUT    /api/users/{id}                - Update user (admin)
DELETE /api/users/{id}                - Delete user (admin)
```

### Registration
```
POST   /api/auth/register             - Self-registration (pending approval)
POST   /api/auth/login                - Login (already implemented)
```

---

## Migration from Current State

Since there's no existing permission system, migration is straightforward:

1. Run database migrations to create new tables
2. Deploy permission system code
3. First user to register becomes super admin
4. Admin creates teams and assigns permissions
5. All existing resources have no owner (historical data)
   - Option A: Assign all existing resources to first user
   - Option B: Leave ownership NULL for historical data (only enforce on new resources)

---

## Security Considerations

1. **JWT Secret**: Must be configured in `manager.toml` before deployment
2. **SQL Injection**: All queries use parameterized statements
3. **Path Traversal**: File operations already bounded to project directory
4. **Rate Limiting**: Consider adding rate limiting to login and registration endpoints
5. **Audit Logging**: Track permission changes (granted_by, granted_at already included)
6. **Token Expiry**: JWT tokens expire after 24 hours (configurable)

---

## Performance Considerations

1. **Indexes**: All foreign keys and frequently-queried columns are indexed
2. **Caching**: Consider caching user teams and permissions in memory (future optimization)
3. **Query Optimization**: Use JOINs instead of N+1 queries when fetching team permissions
4. **Permission Check Frequency**: Called on every protected endpoint - keep fast

---

## Future Enhancements (Out of Scope)

- [ ] Permission expiry (time-limited access)
- [ ] Audit log table for permission changes
- [ ] Permission templates (pre-defined permission sets)
- [ ] Nested teams (teams within teams)
- [ ] IP whitelist for sensitive operations
- [ ] Two-factor authentication (2FA)
- [ ] OAuth integration (GitHub, Google)
- [ ] API rate limiting per user/team

---

## References

- Database Schema: `/home/brainless/Projects/nocodo/manager/src/database.rs`
- User Models: `/home/brainless/Projects/nocodo/manager/src/models.rs`
- Auth Module: `/home/brainless/Projects/nocodo/manager/src/auth.rs`
- Config: `/home/brainless/Projects/nocodo/manager/src/config.rs`

---

*Last Updated: 2025-10-29*
