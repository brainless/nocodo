# Authentication & Authorization System - Integration Tests

## Overview

This task focuses on implementing comprehensive integration tests for the Authentication & Authorization System in Nocodo Manager. The system supports three distinct connection types: HTTP JWT-based authentication, SSH key authentication, and Unix socket local connections. This is a **CRITICAL PRIORITY** security feature that requires thorough testing.

## Test File

**Location:** `manager/tests/integration/auth_system.rs`

**Test Type:** Integration tests with real database operations

## Business Logic Requirements

### Critical Security Logic
1. **First User Super Admin Promotion:** The first user to register must automatically receive Super Admin role
2. **Password Security:** Passwords must be hashed using Argon2id before storage
3. **JWT Token Management:** Tokens must be properly generated, validated, and expired
4. **SSH Key Validation:** SSH public keys must be validated for format and cryptographic correctness
5. **Unix Socket Bypass:** Local socket connections must bypass JWT authentication requirements

### Authentication Flow Architecture

The system implements three distinct authentication mechanisms:

1. **HTTP JWT Authentication** (`src/handlers/user_handlers.rs`)
   - Login endpoint: `POST /api/auth/login`
   - Register endpoint: `POST /api/auth/register`
   - Middleware: `src/middleware.rs` - `AuthenticationMiddleware`
   - Token generation: `src/auth.rs` - `generate_token()`
   - Token validation: `src/auth.rs` - `validate_token()`

2. **SSH Key Authentication** (`src/handlers/main_handlers.rs`)
   - Endpoint: `POST /api/settings/authorized-ssh-keys`
   - Model: `src/models.rs` - `UserSshKey`
   - Fingerprint-based authentication with JWT token containing `ssh_fingerprint` claim

3. **Unix Socket Local Connection**
   - Direct socket connection without HTTP layer
   - Bypasses JWT authentication in middleware (see `middleware.rs:64-68`)
   - Used for local development and CLI tools

## User Personas & Authentication Flows

### Persona 1: First-Time Setup Admin
**Scenario:** Initial system deployment, no users exist

**User Flow:**
1. Launch fresh Nocodo Manager instance (empty database)
2. Navigate to registration page
3. Register with username: "alice", password: "SecurePass123!", email: "alice@example.com"
4. System automatically creates "Super Admin" team
5. Alice is added to Super Admin team with full permissions
6. Receive JWT token with 24-hour expiration
7. Can now access all system resources

**Test Requirements:**
- Database must be empty before test
- Verify user count = 1 after registration
- Verify team "Super Admin" exists
- Verify Alice is member of Super Admin team
- Verify Alice has ALL permissions on ALL resource types
- Verify JWT token contains correct user_id and username
- Verify token is valid for 24 hours

### Persona 2: Subsequent User Registration
**Scenario:** Normal user joins existing system

**User Flow:**
1. Bob registers after Alice (database has users)
2. Register with username: "bob", password: "AnotherPass456!", email: "bob@example.com"
3. Bob receives user account but NO special privileges
4. Bob does NOT get added to Super Admin team
5. Bob needs explicit team assignments for permissions

**Test Requirements:**
- Database must have at least 1 user before test
- Verify Bob is created successfully
- Verify Bob is NOT in Super Admin team
- Verify Bob has NO default permissions
- Verify only first user (Alice) remains Super Admin

### Persona 3: SSH Key Developer
**Scenario:** Developer using SSH keys for authentication

**User Flow:**
1. Carol has existing account (registered via HTTP)
2. Generates SSH key pair locally: `ssh-keygen -t ed25519`
3. Submits public key via `POST /api/settings/authorized-ssh-keys`
4. System validates key format and generates fingerprint
5. System stores key associated with Carol's user_id
6. Carol can now authenticate using SSH key
7. SSH authentication generates JWT token with `ssh_fingerprint` claim

**Test Requirements:**
- Valid SSH key formats: ssh-ed25519, ssh-rsa, ecdsa-sha2-nistp256
- Invalid key format should return 400 Bad Request
- Duplicate key fingerprints should return 409 Conflict
- Fingerprint must be SHA256-based
- JWT token must include `ssh_fingerprint` claim
- Token generated from SSH auth must work for HTTP API calls

### Persona 4: Local CLI Developer
**Scenario:** Developer using Unix socket for local development

**User Flow:**
1. Dave runs Nocodo Manager locally on localhost
2. Dave's CLI tool connects via Unix socket (e.g., `/tmp/nocodo.sock`)
3. Socket connection bypasses JWT authentication
4. Middleware detects Unix socket connection
5. Request proceeds without Authorization header
6. All operations execute with local user privileges

**Test Requirements:**
- Mock Unix socket connection detection
- Verify requests without Authorization header succeed
- Verify local socket requests can access protected endpoints
- Verify socket permissions prevent unauthorized local access

### Persona 5: Authenticated API User
**Scenario:** Developer making authenticated HTTP requests

**User Flow:**
1. Eve logs in via `POST /api/auth/login`
2. Provides username: "eve", password: "EvePass789!"
3. System verifies password hash using Argon2
4. System generates JWT token with 24-hour expiration
5. Eve includes token in subsequent requests: `Authorization: Bearer <token>`
6. Middleware validates token on each request
7. Token extracts user_id and attaches to request context

**Test Requirements:**
- Valid credentials return 200 with JWT token
- Invalid username returns 401 Unauthorized
- Invalid password returns 401 Unauthorized
- Empty username returns 400 Bad Request
- Empty password returns 400 Bad Request
- Token must be validated by middleware
- Expired token returns 401 Unauthorized
- Malformed token returns 401 Unauthorized
- Missing Authorization header returns 401 Unauthorized
- Wrong secret key must not validate token

## Test Structure

### Module Organization

```rust
//! Integration tests for Authentication & Authorization System
//!
//! Tests cover:
//! - HTTP JWT authentication (login/register)
//! - SSH key authentication and validation
//! - Unix socket local connection authentication
//! - First user Super Admin promotion logic
//! - Password hashing and verification
//! - JWT token generation, validation, and expiration
//! - Authorization header parsing and validation

mod http_jwt_auth;
mod ssh_key_auth;
mod unix_socket_auth;
mod security_validation;
```

### Test Naming Convention

**Format:** `test_<persona>_<scenario>_<expected_outcome>`

**Examples:**
- `test_first_user_registration_receives_super_admin`
- `test_subsequent_user_registration_no_super_admin`
- `test_ssh_key_valid_ed25519_generates_token`
- `test_unix_socket_bypasses_jwt_requirement`
- `test_login_invalid_credentials_returns_401`

## Detailed Test Specifications

### 1. HTTP JWT Authentication Tests

#### Module: `http_jwt_auth`

**Test:** `test_first_user_registration_receives_super_admin`
- **Setup:** Empty database
- **Action:** Register user "alice"
- **Assertions:**
  - Response status: 201 Created
  - User ID returned
  - Query teams for "Super Admin" team
  - Verify team exists and contains alice
  - Verify alice has permissions: (Project, Create), (Project, Read), (User, Create), etc.
- **Technical Details:**
  - Use `data.database.get_all_users()` to verify count = 1
  - Use `data.database.get_user_teams(user_id)` to check team membership
  - Use permissions API to verify full access

**Test:** `test_subsequent_user_registration_no_super_admin`
- **Setup:** Database with 1 existing user
- **Action:** Register user "bob"
- **Assertions:**
  - Response status: 201 Created
  - Bob's user ID returned
  - Query teams for bob
  - Verify bob NOT in "Super Admin" team
  - Verify bob has 0 permissions
- **Technical Details:**
  - Seed database with first user before test
  - Use `data.database.get_user_teams(bob_id)` should return empty or non-admin teams

**Test:** `test_registration_duplicate_username_returns_error`
- **Setup:** Database with user "alice"
- **Action:** Attempt to register "alice" again
- **Assertions:**
  - Response status: 400 Bad Request
  - Error message: "Username already exists"
- **Technical Details:**
  - Handler checks `data.database.get_user_by_name(&username)` before creation

**Test:** `test_registration_empty_username_returns_error`
- **Action:** Register with username: ""
- **Assertions:**
  - Response status: 400 Bad Request
  - Error message: "Username cannot be empty"

**Test:** `test_registration_password_is_hashed`
- **Action:** Register user with password "PlainTextPass"
- **Assertions:**
  - Password in database starts with "$argon2id$"
  - Password hash is not equal to plain text
  - Verify password using `crate::auth::verify_password()`
- **Technical Details:**
  - Query database directly: `SELECT password_hash FROM users WHERE name = 'alice'`
  - Hash format: `$argon2id$v=19$m=19456,t=2,p=1$...`

**Test:** `test_login_valid_credentials_returns_token`
- **Setup:** User "alice" with password "SecurePass123!"
- **Action:** POST /api/auth/login with username and password
- **Assertions:**
  - Response status: 200 OK
  - Response contains "token" field
  - Response contains "user" object with id, username, email
  - Token is valid JWT
  - Token claims contain correct user_id and username
- **Technical Details:**
  - Decode token using `crate::auth::validate_token()`
  - Verify `claims.sub == user_id.to_string()`
  - Verify `claims.username == "alice"`

**Test:** `test_login_invalid_username_returns_401`
- **Action:** Login with non-existent username
- **Assertions:**
  - Response status: 401 Unauthorized
  - Error message: "Invalid credentials"

**Test:** `test_login_invalid_password_returns_401`
- **Setup:** User "alice" exists
- **Action:** Login with correct username, wrong password
- **Assertions:**
  - Response status: 401 Unauthorized
  - Error message: "Invalid credentials"

**Test:** `test_login_empty_username_returns_400`
- **Action:** Login with empty username
- **Assertions:**
  - Response status: 400 Bad Request
  - Error message: "Username is required"

**Test:** `test_login_empty_password_returns_400`
- **Action:** Login with empty password
- **Assertions:**
  - Response status: 400 Bad Request
  - Error message: "Password is required"

**Test:** `test_jwt_token_expiration_24_hours`
- **Action:** Register/login to get token
- **Assertions:**
  - Token claims contain `exp` field
  - `exp - iat == 86400` (24 hours in seconds)
- **Technical Details:**
  - Claims created in `auth.rs:57-59` with 24-hour expiration

**Test:** `test_jwt_token_expired_returns_401`
- **Action:** Create token with expiration in past
- **Attempt:** Use expired token for authenticated request
- **Assertions:**
  - Response status: 401 Unauthorized
  - Error message: "Invalid or expired token"
- **Technical Details:**
  - Use `Claims::new_with_duration()` with negative duration
  - Or manually set `exp` to past timestamp

**Test:** `test_jwt_token_malformed_returns_401`
- **Action:** Send request with malformed token: "Bearer invalid.token.here"
- **Assertions:**
  - Response status: 401 Unauthorized
  - Error message: "Invalid or expired token"

**Test:** `test_jwt_token_missing_authorization_header_returns_401`
- **Action:** Send authenticated request without Authorization header
- **Assertions:**
  - Response status: 401 Unauthorized
  - Error message: "Missing Authorization header"

**Test:** `test_jwt_token_wrong_secret_fails_validation`
- **Action:** Generate token with secret "key1", validate with secret "key2"
- **Assertions:**
  - Validation fails
  - Returns AppError::Unauthorized
- **Technical Details:**
  - Unit test in `auth.rs:153-162` already covers this
  - Integration test should verify middleware behavior

**Test:** `test_authenticated_request_with_valid_token_succeeds`
- **Setup:** Login to get valid token
- **Action:** GET /api/projects with Authorization header
- **Assertions:**
  - Response status: 200 OK
  - Request proceeds successfully
  - User info attached to request context

### 2. SSH Key Authentication Tests

#### Module: `ssh_key_auth`

**Test:** `test_ssh_key_valid_ed25519_format_accepted`
- **Setup:** User "carol" exists
- **Action:** POST /api/settings/authorized-ssh-keys with valid ed25519 key
- **Request Body:**
  ```json
  {
    "ssh_key": "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIGj... carol@work-laptop"
  }
  ```
- **Assertions:**
  - Response status: 200 OK
  - Response: `{"success": true}`
- **Mock:** SSH key cryptographic validation (accept all valid formats)

**Test:** `test_ssh_key_valid_rsa_format_accepted`
- **Action:** POST with valid RSA key format
- **Request Body:**
  ```json
  {
    "ssh_key": "ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABA... carol@laptop"
  }
  ```
- **Assertions:**
  - Response status: 200 OK

**Test:** `test_ssh_key_valid_ecdsa_format_accepted`
- **Action:** POST with valid ECDSA key format
- **Request Body:**
  ```json
  {
    "ssh_key": "ecdsa-sha2-nistp256 AAAAE2VjZHNhLXNo... carol@home"
  }
  ```
- **Assertions:**
  - Response status: 200 OK

**Test:** `test_ssh_key_invalid_format_returns_400`
- **Action:** POST with malformed key
- **Request Body:**
  ```json
  {
    "ssh_key": "invalid-key-format blahblah"
  }
  ```
- **Assertions:**
  - Response status: 400 Bad Request
  - Error message: "Invalid SSH key format"
- **Technical Details:**
  - Current implementation (`main_handlers.rs:145-167`) only validates presence
  - Test should expect future validation logic

**Test:** `test_ssh_key_missing_parameter_returns_400`
- **Action:** POST without ssh_key field
- **Assertions:**
  - Response status: 400 Bad Request
  - Error message: "Invalid ssh_key parameter"

**Test:** `test_ssh_key_authentication_generates_jwt_token`
- **Action:** Authenticate using SSH key (simulated)
- **Assertions:**
  - JWT token generated
  - Token claims contain `ssh_fingerprint` field
  - Fingerprint format: "SHA256:base64hash"
- **Technical Details:**
  - Claims struct in `auth.rs:51` includes optional ssh_fingerprint
  - Mock SSH authentication to return fingerprint

**Test:** `test_ssh_key_duplicate_fingerprint_returns_409`
- **Setup:** Carol has SSH key with fingerprint "SHA256:abc123"
- **Action:** Add same SSH key again
- **Assertions:**
  - Response status: 409 Conflict
  - Error message: "SSH key already registered"
- **Technical Details:**
  - Requires database schema for user_ssh_keys table
  - Model exists in `models.rs:26-65` - `UserSshKey`

**Test:** `test_ssh_key_removal_succeeds`
- **Setup:** Carol has active SSH key
- **Action:** DELETE /api/settings/authorized-ssh-keys/{key_id}
- **Assertions:**
  - Response status: 204 No Content
  - Key marked inactive or deleted
  - Subsequent auth with that key fails

### 3. Unix Socket Connection Tests

#### Module: `unix_socket_auth`

**Test:** `test_unix_socket_bypasses_jwt_authentication`
- **Setup:** Mock Unix socket connection detection
- **Action:** Request to protected endpoint without Authorization header
- **Assertions:**
  - Request proceeds successfully
  - No 401 Unauthorized error
  - User context populated with local user
- **Technical Details:**
  - Middleware (`middleware.rs:64-68`) checks for health/login/register paths
  - For local socket, modify test to simulate socket connection
  - May need test-only flag or environment variable

**Test:** `test_unix_socket_local_user_context`
- **Setup:** Unix socket connection
- **Action:** Protected endpoint call
- **Assertions:**
  - Request extensions contain UserInfo
  - UserInfo populated with local system user
- **Mock:** Socket server connection detection

**Test:** `test_unix_socket_permission_validation`
- **Action:** Verify socket file permissions (0600 or 0660)
- **Assertions:**
  - Only owner or group can access socket
  - Prevents unauthorized local access
- **Mock:** File system permission checks

**Test:** `test_http_connection_requires_jwt_token`
- **Setup:** Regular HTTP connection (not Unix socket)
- **Action:** Protected endpoint without Authorization header
- **Assertions:**
  - Response status: 401 Unauthorized
  - Error: "Missing Authorization header"
- **Technical Details:**
  - Ensures Unix socket bypass ONLY applies to socket connections
  - Regular HTTP still requires JWT

### 4. Security Validation Tests

#### Module: `security_validation`

**Test:** `test_password_hash_uses_argon2id`
- **Action:** Hash password "TestPass123"
- **Assertions:**
  - Hash starts with "$argon2id$"
  - Hash contains version "v=19"
  - Hash contains memory cost "m=19456"
  - Hash contains time cost "t=2"
  - Hash contains parallelism "p=1"
- **Technical Details:**
  - OWASP recommended parameters (`auth.rs:16-20`)

**Test:** `test_password_verification_correct_password`
- **Setup:** Hash password "SecurePass"
- **Action:** Verify with "SecurePass"
- **Assertions:**
  - Returns Ok(true)

**Test:** `test_password_verification_incorrect_password`
- **Setup:** Hash password "SecurePass"
- **Action:** Verify with "WrongPass"
- **Assertions:**
  - Returns Ok(false)

**Test:** `test_jwt_secret_not_configured_test_mode`
- **Setup:** AppState without JWT secret in config
- **Action:** Protected endpoint request
- **Assertions:**
  - Request proceeds with test user
  - UserInfo populated: id=1, username="testuser"
- **Technical Details:**
  - Middleware (`middleware.rs:84-94`) handles missing JWT secret
  - Used for integration tests

**Test:** `test_authorization_header_invalid_format_returns_401`
- **Action:** Request with "Authorization: InvalidFormat token123"
- **Assertions:**
  - Response status: 401 Unauthorized
  - Error: "Invalid Authorization header format. Expected 'Bearer <token>'"

**Test:** `test_concurrent_login_requests_succeed`
- **Setup:** User "alice" exists
- **Action:** Send 10 concurrent login requests with same credentials
- **Assertions:**
  - All 10 requests return 200 OK
  - All tokens are valid
  - All tokens have different `iat` timestamps
- **Technical Details:**
  - Tests thread safety of authentication system

**Test:** `test_rate_limiting_login_attempts`
- **Action:** Send 100 rapid login requests from same IP
- **Assertions:**
  - After threshold (e.g., 5 attempts), return 429 Too Many Requests
  - Lockout duration enforced
- **Note:** Rate limiting not implemented yet - mark as TODO

**Test:** `test_session_cleanup_on_logout`
- **Setup:** User logged in with valid token
- **Action:** POST /api/auth/logout (endpoint not implemented yet)
- **Assertions:**
  - Token invalidated
  - Subsequent requests with token fail
- **Note:** Logout endpoint not implemented - mark as TODO

## Mock Requirements (This Phase Only)

### LLM SDK Calls
**Not needed for authentication tests** - No AI features in auth flow

### SSH Key Validation
**Required:**
- **Function:** `validate_ssh_public_key(key: &str) -> Result<(String, String), Error>`
- **Returns:** `(key_type, fingerprint)` or error
- **Mock Behavior:**
  - Accept: "ssh-ed25519 ...", "ssh-rsa ...", "ecdsa-sha2-nistp256 ..."
  - Reject: Invalid formats, empty strings
  - Generate deterministic fingerprints for testing
- **Implementation:**
  ```rust
  fn mock_validate_ssh_key(key: &str) -> Result<(String, String), AppError> {
      if key.starts_with("ssh-ed25519 ") {
          Ok(("ssh-ed25519".to_string(), "SHA256:mock-ed25519-fingerprint".to_string()))
      } else if key.starts_with("ssh-rsa ") {
          Ok(("ssh-rsa".to_string(), "SHA256:mock-rsa-fingerprint".to_string()))
      } else if key.starts_with("ecdsa-sha2-nistp256 ") {
          Ok(("ecdsa-sha2-nistp256".to_string(), "SHA256:mock-ecdsa-fingerprint".to_string()))
      } else {
          Err(AppError::InvalidRequest("Invalid SSH key format".to_string()))
      }
  }
  ```

### Unix Socket Server
**Required:**
- **Mock:** Connection type detection in test framework
- **Approach:** Use test-only configuration flag `is_unix_socket_connection: bool`
- **Integration:**
  - Add field to test request builder
  - Modify middleware to check flag in test mode
  - Regular production code detects actual socket connection

### JWT Token Generation
**No mock needed** - Use real `jsonwebtoken` crate for actual token operations

## Test Setup & Teardown

### Database Fixture Management

**Before Each Test:**
```rust
async fn setup_test_db() -> TestApp {
    let test_db_path = format!("test_auth_{}.db", uuid::Uuid::new_v4());
    let app = TestApp::new(&test_db_path).await;
    app
}
```

**After Each Test:**
```rust
async fn teardown_test_db(test_db_path: &str) {
    std::fs::remove_file(test_db_path).ok();
}
```

**Seed Data Functions:**
```rust
async fn seed_first_user(app: &TestApp) -> i64 {
    // Register first user "alice"
    // Returns user_id
}

async fn seed_multiple_users(app: &TestApp, count: usize) -> Vec<i64> {
    // Register multiple users
    // Returns Vec<user_id>
}
```

## Success Criteria

### Code Coverage
- **Target:** 100% endpoint coverage for authentication endpoints
- **Endpoints:**
  - `POST /api/auth/login`
  - `POST /api/auth/register`
  - `POST /api/settings/authorized-ssh-keys`

### Security Validation
- ✅ All password hashing uses Argon2id with OWASP parameters
- ✅ First user Super Admin promotion works correctly
- ✅ Subsequent users do NOT get Super Admin
- ✅ JWT tokens properly expire after 24 hours
- ✅ Invalid tokens rejected by middleware
- ✅ SSH key formats validated correctly
- ✅ Unix socket connections bypass JWT requirement

### Test Quality Metrics
- All tests isolated (independent database for each test)
- No test interdependencies
- Clear test names describing scenario
- Comprehensive error case coverage
- All user personas covered

## Implementation Phases

### Phase 1: HTTP JWT Authentication (3-5 tests at a time)
1. First user registration and Super Admin promotion
2. Login with valid/invalid credentials
3. Password hashing validation
4. JWT token generation and validation

### Phase 2: Token Validation & Middleware (3-5 tests at a time)
1. Expired token handling
2. Malformed token handling
3. Missing authorization header
4. Authenticated requests with valid tokens

### Phase 3: SSH Key Authentication (3-5 tests at a time)
1. Valid SSH key format acceptance
2. Invalid format rejection
3. JWT generation with ssh_fingerprint
4. Duplicate key handling

### Phase 4: Unix Socket & Edge Cases (3-5 tests at a time)
1. Unix socket bypass logic
2. Concurrent authentication requests
3. Security validation tests

## Technical Notes

### Password Hashing Details
- **Algorithm:** Argon2id (hybrid of Argon2i and Argon2d)
- **Memory Cost:** 19,456 KiB (19 MiB)
- **Time Cost:** 2 iterations
- **Parallelism:** 1 thread
- **Source:** `manager/src/auth.rs:12-28`

### JWT Token Structure
```json
{
  "sub": "42",  // user_id as string
  "username": "alice",
  "exp": 1733974800,  // Unix timestamp
  "iat": 1733888400,  // Unix timestamp
  "ssh_fingerprint": "SHA256:abc123"  // Optional
}
```

### Middleware Flow
1. Check if path is public (`/api/health`, `/api/auth/login`, `/api/auth/register`)
2. If JWT secret not configured → test mode with default user
3. Extract Authorization header
4. Parse Bearer token
5. Validate token with JWT secret
6. Attach UserInfo to request extensions
7. Proceed to handler

### Database Schema References

**Users Table:**
```sql
CREATE TABLE users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    email TEXT NOT NULL,
    role TEXT,
    password_hash TEXT NOT NULL,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    last_login_at INTEGER
)
```

**User SSH Keys Table (from model):**
```sql
CREATE TABLE user_ssh_keys (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    key_type TEXT NOT NULL,
    fingerprint TEXT NOT NULL UNIQUE,
    public_key_data TEXT NOT NULL,
    label TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,
    last_used_at INTEGER,
    FOREIGN KEY (user_id) REFERENCES users(id)
)
```

## References

- Test Priority Plan: `manager/.test-analysis/test_priority_plan.txt`
- Coverage Report: `manager/.test-analysis/coverage_report.txt`
- Existing Permission Tests: `manager/tests/integration/permission_system_api.rs`
- Auth Module: `manager/src/auth.rs`
- User Handlers: `manager/src/handlers/user_handlers.rs`
- Middleware: `manager/src/middleware.rs`
- Models: `manager/src/models.rs`
