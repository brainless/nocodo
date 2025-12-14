# nocodo Authentication & Authorization Reference

Simple reference guide for the authentication and permission system.

---

## Architecture Overview

**Desktop App** ←SSH Tunnel→ **Manager (Cloud Server)**
- Desktop: Native GUI, SSH client, local API proxy
- Manager: REST API, SQLite database, JWT authentication

---

## Authentication Flow

### 1. Connection Setup (Desktop → Manager)

```
Desktop App starts
  ↓
Establish SSH tunnel (using SSH key pair)
  ↓
Port forward: localhost:random_port → server:8081
  ↓
API Client connects to localhost:random_port
```

**Implementation:**
- `desktop-app/src/ssh.rs` - SSH tunnel & key management
- `desktop-app/src/connection_manager.rs` - Connection lifecycle

### 2. User Authentication

#### Registration (First User = Super Admin)
```
POST /api/auth/register
{
  "username": "alice",
  "password": "secret",
  "email": "alice@example.com" (optional),
  "ssh_public_key": "ssh-ed25519 AAAAC3...",
  "ssh_fingerprint": "SHA256:abc123..."
}
```

**First user gets:**
- "Super Admins" team created automatically
- Added as member of "Super Admins" team
- **IMPORTANT:** Super Admins team has NO explicit permissions in the database
- Instead, Super Admins members get **implicit all permissions** through automatic grant in permission check logic
- This means Super Admins can perform ANY action on ANY resource, regardless of permission records

**Later users:**
- Account created, no default permissions
- Admin must add to teams manually

**Implementation:**
- `manager/src/handlers.rs:register_user()` - Registration endpoint
- `desktop-app/src/components/auth_dialog.rs` - Registration UI

#### Login
```
POST /api/auth/login
{
  "username": "alice",
  "password": "secret",
  "ssh_fingerprint": "SHA256:abc123..."
}

Response:
{
  "token": "eyJhbGciOiJIUzI1NiIs...",
  "user": { "id": 1, "username": "alice", ... }
}
```

**Implementation:**
- `manager/src/handlers.rs:login()` - Login endpoint
- `manager/src/auth.rs` - Password hashing (Argon2id), JWT generation
- `desktop-app/src/connection_manager.rs:login()` - Client-side login

### 3. JWT Token Management

**Token Format:**
```json
{
  "sub": "1",                              // User ID
  "username": "alice",
  "exp": 1735776000,                       // Expiration (24h default)
  "iat": 1735689600,                       // Issued at
  "ssh_fingerprint": "SHA256:abc123..."    // SSH key used for login
}
```

**Token Storage:**
- Desktop: Stored in `ConnectionManager.jwt_token` (in-memory)
- API calls: Sent as `Authorization: Bearer <token>` header
- Auto-restored: After SSH reconnection

**Implementation:**
- `manager/src/auth.rs:generate_token()` - JWT creation
- `manager/src/auth.rs:validate_token()` - JWT validation
- `desktop-app/src/connection_manager.rs` - Token persistence during reconnection

---

## Authorization (Team-Based Permissions)

### Core Concepts

1. **Teams** - Groups of users that share permissions
   - Example: "Frontend Team", "Super Admins", "Project Alpha"

2. **Permissions** - Assigned to teams, NOT individual users
   - Resource: `project`, `work`, `settings`, `user`, `team`
   - Action: `read`, `write`, `delete`, `admin`
   - Scope: Specific resource ID OR entity-level (all resources)

3. **Ownership** - Resource creators get auto read/write/delete

### Permission Check Flow

```
User makes API request
  ↓
Middleware extracts JWT token → validates → attaches UserInfo
  ↓
Permission middleware checks (in order):
  0. Is user in "Super Admins" team? → GRANT ALL (implicit, automatic)
  1. Is user the owner? (auto read/write/delete)
  2. Team permission on specific resource?
  3. Parent resource permission? (inheritance)
  4. Team permission on all resources of this type?
  ↓
Grant (200) or Deny (403)
```

**CRITICAL:** Step 0 (Super Admins check) happens BEFORE all other checks. Super Admins members automatically pass ALL permission checks without needing explicit permission records in the database.

**Implementation:**
- `manager/src/middleware.rs:AuthenticationMiddleware` - JWT validation
- `manager/src/middleware.rs:PermissionMiddleware` - Permission enforcement
- `manager/src/permissions.rs:check_permission()` - Permission logic

### Action Hierarchy

```
admin  →  implies read, write, delete
write  →  implies read
delete →  implies read
```

### Permission Examples

**Entity-level (all projects):**
```sql
INSERT INTO permissions (team_id, resource_type, resource_id, action)
VALUES (10, 'project', NULL, 'write');
```
→ Team 10 can read/write ALL projects

**Resource-specific:**
```sql
INSERT INTO permissions (team_id, resource_type, resource_id, action)
VALUES (10, 'project', 5, 'read');
```
→ Team 10 can read Project #5 only

---

## SSH Key Integration

### SSH Fingerprint Calculation
```rust
// SHA256 hash of SSH public key
calculate_ssh_fingerprint() → "SHA256:base64hash"
```

**Used for:**
- Login credential (alternative to password alone)
- Tracking which SSH key was used for authentication

**Implementation:**
- `desktop-app/src/ssh.rs:calculate_ssh_fingerprint()` - Fingerprint calculation
- `desktop-app/src/ssh.rs:read_ssh_public_key()` - Public key reading

### Key Locations (auto-detected)
```
~/.ssh/id_ed25519
~/.ssh/id_rsa
~/.ssh/id_ecdsa
```

---

## Connection Management

### Health Checks & Auto-Reconnect

**Desktop monitors connection every 30 seconds:**
- Sends `GET /api/health` ping
- If 401 Unauthorized → show login dialog
- If connection fails → auto-reconnect (preserve JWT token)
- SSH keepalive: every 30s, timeout after 10min inactivity

**Implementation:**
- `desktop-app/src/connection_manager.rs:start_health_check_task()` - Health monitoring
- `desktop-app/src/connection_manager.rs:reconnect()` - Auto-reconnect with JWT persistence

### Connection States
```
Disconnected → Connecting → Connected (unauthenticated) → Authenticated
                    ↓                        ↓
                  Failed               Auth Required (401)
```

---

## Security Features

### Password Storage
- **Algorithm:** Argon2id (OWASP recommended)
- **Parameters:** 19 MiB memory, 2 iterations
- **Implementation:** `manager/src/auth.rs:hash_password()`

### JWT Security
- **Secret:** Configured in `manager.toml` (`auth.jwt_secret`)
- **Expiration:** 24 hours (configurable)
- **Validation:** On every protected endpoint

### SSH Security
- **Key-based auth:** SSH public key authentication
- **Server verification:** Accepts all keys (TODO: verify against known_hosts in production)

### Protected Endpoints
```
✓ Public:  /api/health, /api/auth/login, /api/auth/register
✗ Private: All other /api/* routes (require JWT)
```

---

## Database Schema

### Users
```sql
users (id, username, password_hash, email, ssh_public_key, ssh_fingerprint, created_at)
```

### Teams & Permissions

```sql
-- Teams: Groups of users with shared permissions
CREATE TABLE teams (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    created_by INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (created_by) REFERENCES users (id) ON DELETE SET NULL
);

-- Team Members: User membership in teams
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

-- Permissions: Team-level access control
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

-- Resource Ownership: Automatic permissions for resource creators
CREATE TABLE resource_ownership (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    resource_type TEXT NOT NULL,
    resource_id INTEGER NOT NULL,
    owner_id INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (owner_id) REFERENCES users (id) ON DELETE CASCADE,
    UNIQUE(resource_type, resource_id)
);
```

---

## Key Files

### Manager (Server)
```
manager/src/
├── auth.rs           - Password hashing, JWT generation/validation
├── middleware.rs     - Authentication & permission middleware
├── permissions.rs    - Permission checking logic
├── handlers.rs       - API endpoints (login, register, etc.)
└── database.rs       - User & permission database operations
```

### Desktop App (Client)
```
desktop-app/src/
├── ssh.rs                        - SSH tunnel & fingerprint calculation
├── connection_manager.rs         - Connection lifecycle, health checks
├── components/auth_dialog.rs     - Login/register UI
└── api_client.rs                 - HTTP client with JWT headers
```

---

## Common Workflows

### First-Time Setup
1. User installs desktop app
2. Desktop connects to server via SSH
3. User clicks "Register" → creates account
4. Server creates "Super Admins" team, grants all permissions
5. User logs in → receives JWT token
6. Desktop stores token, uses for all API calls

### Daily Usage
1. Desktop app starts → establishes SSH tunnel
2. Auto-login with stored credentials
3. Health check detects expired JWT → shows login dialog
4. User logs in → new token issued → work continues

### Adding New User
1. Admin creates user account (or user self-registers)
2. Admin creates team (or uses existing team)
3. Admin adds user to team
4. Admin grants permissions to team
5. New user logs in → inherits team permissions

---

## Testing Auth Flow

### Testing Super Admins Permissions

**CRITICAL:** Always verify Super Admins team behavior in tests. Super Admins should have implicit all permissions without explicit permission records.

**Required tests:**
```rust
// 1. Verify Super Admins team is created for first user
#[test]
fn test_first_user_creates_super_admins_team() {
    // Register first user
    // Assert "Super Admins" team exists
    // Assert first user is member of "Super Admins"
}

// 2. Verify Super Admins have NO explicit permissions
#[test]
fn test_super_admins_no_explicit_permissions() {
    // Register first user → creates Super Admins team
    // Query permissions table for Super Admins team
    // Assert: permission count = 0 (implicit, not explicit)
}

// 3. Verify Super Admins can access ALL endpoints
#[test]
fn test_super_admins_implicit_all_permissions() {
    // Register first user → becomes Super Admin
    // Test access to endpoints requiring different permissions:
    //   - GET /api/projects (project:read)
    //   - POST /api/projects/scan (project:write)
    //   - GET /api/settings (settings:read)
    //   - POST /api/settings/projects-path (settings:write)
    //   - POST /api/teams (team:write)
    //   - POST /api/permissions (team:admin)
    // Assert: All requests succeed (200/201)
}
```

**Test locations:**
- `manager/tests/fresh_install_403_test.rs` - First user bootstrap verification
- `manager/tests/integration/permission_system_api.rs:test_super_admin_permissions()` - Super Admins implicit permissions
- `manager/tests/integration/scan_endpoint_super_admin.rs` - Super Admins can scan projects

**What to verify:**
1. ✅ First user registration creates "Super Admins" team
2. ✅ First user is automatically added as team member
3. ✅ Super Admins team has ZERO permission records in database
4. ✅ Super Admins members can access endpoints requiring any permission
5. ✅ Later users do NOT get Super Admins membership

### Manual Test (with curl)
```bash
# 1. Register
curl -X POST http://localhost:8081/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{
    "username": "alice",
    "password": "secret123",
    "ssh_public_key": "ssh-ed25519 AAAAC3...",
    "ssh_fingerprint": "SHA256:abc123..."
  }'

# 2. Login
TOKEN=$(curl -X POST http://localhost:8081/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "username": "alice",
    "password": "secret123",
    "ssh_fingerprint": "SHA256:abc123..."
  }' | jq -r '.token')

# 3. Access protected endpoint
curl http://localhost:8081/api/projects \
  -H "Authorization: Bearer $TOKEN"
```

---

## Permission System Details

### Permission Check Priority Order

When checking if a user can perform an action on a resource, the system checks in this order:

0. **Super Admins team** - If user is member of "Super Admins" team → **automatic grant of ALL permissions**
   - This is an implicit check - Super Admins have NO permission records in database
   - They automatically pass ALL permission checks regardless of resource or action
   - See: `manager/src/permissions.rs:129-135`

1. **Ownership** - If user owns the resource → grant read/write/delete automatically
2. **Resource-level permission** - Check permission on specific resource (team_id + resource_id)
3. **Parent resource permission** - For hierarchical projects, check parent project permissions
4. **Entity-level permission** - Check permission on all resources of this type (resource_id = NULL)

### Usage Examples

**Example 1: Grant team access to specific project**
```sql
-- Create "Frontend Team"
INSERT INTO teams (name, description, created_by, created_at, updated_at)
VALUES ('Frontend Team', 'Frontend developers', 1, unixepoch(), unixepoch());

-- Add members
INSERT INTO team_members (team_id, user_id, added_by, added_at)
VALUES (10, 5, 1, unixepoch()),  -- Add Alice
       (10, 6, 1, unixepoch());  -- Add Bob

-- Grant write access to Project #5
INSERT INTO permissions (team_id, resource_type, resource_id, action, granted_by, granted_at)
VALUES (10, 'project', 5, 'write', 1, unixepoch());

-- Result: Alice and Bob can now read/write Project #5
```

**Example 2: Grant entity-level access (all projects)**
```sql
-- Grant "Developers" team write access to ALL projects
INSERT INTO permissions (team_id, resource_type, resource_id, action, granted_by, granted_at)
VALUES (20, 'project', NULL, 'write', 1, unixepoch());

-- Result: All team members can create projects and access all existing projects
```

**Example 3: Single-user team (contractor with limited access)**
```sql
-- Create team for contractor
INSERT INTO teams (name, description, created_by, created_at, updated_at)
VALUES ('John Contractor', 'Temporary access for John', 1, unixepoch(), unixepoch());

-- Add contractor to team
INSERT INTO team_members (team_id, user_id, added_by, added_at)
VALUES (30, 15, 1, unixepoch());

-- Grant read-only access to specific project
INSERT INTO permissions (team_id, resource_type, resource_id, action, granted_by, granted_at)
VALUES (30, 'project', 10, 'read', 1, unixepoch());

-- When contract ends, delete the team or remove the user
```

### Hierarchical Project Permissions

Projects can have parent projects (via `parent_id` field). Permissions inherit from parent to child:

```rust
// User tries to access child project #10 (parent is project #5)
check_permission(user_id, "project", 10, "write")
  → Check ownership of project #10
  → Check permission on project #10
  → Check permission on parent project #5  ← INHERITANCE
  → Check entity-level permission on all projects
```

**Note:** Work sessions do NOT inherit permissions from their parent project - they are independent.

---

## API Endpoints

### Authentication
```
POST   /api/auth/register    - Register new user (first user becomes super admin)
POST   /api/auth/login       - Login with username/password/SSH fingerprint
GET    /api/health           - Health check (no auth required)
```

### Team Management (TODO)
```
POST   /api/teams            - Create team
GET    /api/teams            - List teams
GET    /api/teams/{id}       - Get team details
PUT    /api/teams/{id}       - Update team
DELETE /api/teams/{id}       - Delete team

POST   /api/teams/{id}/members         - Add member to team
DELETE /api/teams/{id}/members/{uid}   - Remove member from team
GET    /api/teams/{id}/members         - List team members
```

### Permission Management (TODO)
```
POST   /api/permissions                - Grant permission to team
GET    /api/permissions                - List all permissions (admin)
GET    /api/teams/{id}/permissions     - List team permissions
DELETE /api/permissions/{id}           - Revoke permission
```

---

## Future Improvements

Planned enhancements:
- [ ] Team management APIs (CRUD teams, members)
- [ ] Permission management APIs (grant/revoke)
- [ ] OAuth integration (GitHub, Google)
- [ ] Two-factor authentication (2FA)
- [ ] Permission expiry (time-limited access)
- [ ] Audit logging for permission changes
- [ ] Permission templates (pre-defined permission sets)
- [ ] Server key verification against known_hosts (production SSH security)

---

*Last Updated: 2025-11-01*
