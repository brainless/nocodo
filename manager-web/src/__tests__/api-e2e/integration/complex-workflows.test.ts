import { afterAll, beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { testApiClient } from '../setup/api-client';
import { testServer } from '../setup/test-server';
import { testDatabase } from '../setup/test-database';
import { testDataGenerator } from '../setup/test-data';
import { testStateManager } from '../utils/state-manager';

describe('Complex Workflows - API Only', () => {
  beforeAll(async () => {
    await testDatabase.setupTestDatabase();
    await testServer.startServer();
    await testStateManager.initialize();
  }, 60000);

  afterAll(async () => {
    testStateManager.clearState();
    await testServer.stopServer();
    await testDatabase.cleanupTestDatabase();
  });

  beforeEach(async () => {
    testDataGenerator.reset();
  });

  describe('Multi-Step Development Workflow', () => {
    it('should handle complete feature development lifecycle', async () => {
      // === PHASE 1: Project and Requirements Analysis ===
      console.log('🔍 Phase 1: Project Setup and Analysis');

      // Create project
      const project = await testStateManager.addProject(
        testDataGenerator.generateProjectData({
          name: 'Complex Feature Development',
          language: 'rust',
          description: 'Testing complete feature development workflow',
        })
      );

      // Create initial project structure
      const initialFiles = [
        testDataGenerator.generateFileData({
          project_id: project.id,
          path: 'README.md',
          content: '# Complex Feature Development\n\nTesting multi-step development workflows.',
        }),
        testDataGenerator.generateFileData({
          project_id: project.id,
          path: 'Cargo.toml',
          content: `[package]\nname = "complex-feature"\nversion = "0.1.0"\nedition = "2021"`,
        }),
        testDataGenerator.generateFileData({
          project_id: project.id,
          path: 'src/lib.rs',
          content: 'pub mod features;\npub mod utils;',
        }),
      ];

      for (const file of initialFiles) {
        await testApiClient.createFile(file);
      }

      // Verify project state
      const stateSummary = testStateManager.getStateSummary();
      expect(stateSummary.projects).toBeGreaterThan(0);

      // === PHASE 2: Feature Planning and Design ===
      console.log('📋 Phase 2: Feature Planning');

      // Create work session for feature planning
      const planningWork = await testStateManager.addWorkSession(
        testDataGenerator.generateWorkData({
          title: 'Feature Planning: User Management System',
          tool_name: 'llm-agent',
          project_id: project.id,
        })
      );

      // Add planning requirements
      await testApiClient.addMessageToWork(
        planningWork.work.id,
        testDataGenerator.generateMessageData({
          content: `Plan and implement a user management system with the following features:
        1. User registration and authentication
        2. User profile management
        3. Role-based access control
        4. User search and filtering
        5. Account deactivation/reactivation

        Start with analysis and design, then implement step by step.`,
          author_type: 'user',
        })
      );

      // AI analysis and planning
      await testApiClient.createAiSession(
        planningWork.work.id,
        testDataGenerator.generateAiSessionData()
      );
      await testApiClient.recordAiOutput(
        planningWork.work.id,
        'Analyzing requirements and creating implementation plan...'
      );

      // Create design document
      const designDoc = testDataGenerator.generateFileData({
        project_id: project.id,
        path: 'docs/USER_MANAGEMENT_DESIGN.md',
        content: `# User Management System Design

## Architecture Overview
- **Models**: User, Role, Permission entities
- **Services**: AuthService, UserService, RoleService
- **API**: RESTful endpoints with JWT authentication
- **Database**: SQLite with migration support

## Implementation Phases
1. **Phase 1**: Core user model and basic CRUD operations
2. **Phase 2**: Authentication system (login/register)
3. **Phase 3**: Role-based permissions
4. **Phase 4**: Advanced features (search, filtering, deactivation)

## Security Considerations
- Password hashing with bcrypt
- JWT tokens with expiration
- Input validation and sanitization
- SQL injection prevention`,
      });
      await testApiClient.createFile(designDoc);

      await testApiClient.recordAiOutput(
        planningWork.work.id,
        'Design document created. Ready to start implementation.'
      );

      // === PHASE 3: Core Implementation ===
      console.log('⚙️ Phase 3: Core Implementation');

      // Create implementation work session
      const implementationWork = await testStateManager.addWorkSession(
        testDataGenerator.generateWorkData({
          title: 'Implementation: Core User Model',
          tool_name: 'llm-agent',
          project_id: project.id,
        })
      );

      await testApiClient.addMessageToWork(
        implementationWork.work.id,
        testDataGenerator.generateMessageData({
          content:
            'Implement the core user model with basic CRUD operations. Include password hashing and validation.',
          author_type: 'user',
        })
      );

      const aiSession = await testStateManager.addAiSession(
        implementationWork.work.id,
        testDataGenerator.generateAiSessionData()
      );
      await testApiClient.recordAiOutput(
        implementationWork.work.id,
        'Starting core user model implementation...'
      );

      // Implement user model
      const userModel = testDataGenerator.generateFileData({
        project_id: project.id,
        path: 'src/models/user.rs',
        content: `use serde::{Deserialize, Serialize};
use bcrypt::{hash, verify, DEFAULT_COST, BcryptResult};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub username: String,
    pub password_hash: String,
    pub role: UserRole,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserRole {
    Admin,
    Moderator,
    User,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub email: String,
    pub username: String,
    pub password: String,
    pub role: Option<UserRole>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub email: Option<String>,
    pub username: Option<String>,
    pub role: Option<UserRole>,
    pub is_active: Option<bool>,
}

impl User {
    pub fn new(req: CreateUserRequest) -> BcryptResult<Self> {
        let password_hash = hash(req.password, DEFAULT_COST)?;

        Ok(Self {
            id: uuid::Uuid::new_v4().to_string(),
            email: req.email,
            username: req.username,
            password_hash,
            role: req.role.unwrap_or(UserRole::User),
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    pub fn verify_password(&self, password: &str) -> BcryptResult<bool> {
        verify(password, &self.password_hash)
    }

    pub fn update(&mut self, req: UpdateUserRequest) {
        if let Some(email) = req.email {
            self.email = email;
        }
        if let Some(username) = req.username {
            self.username = username;
        }
        if let Some(role) = req.role {
            self.role = role;
        }
        if let Some(is_active) = req.is_active {
            self.is_active = is_active;
        }
        self.updated_at = Utc::now();
    }
}

impl Default for UserRole {
    fn default() -> Self {
        UserRole::User
    }
}`,
      });
      await testApiClient.createFile(userModel);

      // Implement user service
      const userService = testDataGenerator.generateFileData({
        project_id: project.id,
        path: 'src/services/user_service.rs',
        content: `use crate::models::user::{User, CreateUserRequest, UpdateUserRequest, UserRole};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type UserStore = Arc<Mutex<HashMap<String, User>>>;

pub struct UserService {
    users: UserStore,
}

impl UserService {
    pub fn new() -> Self {
        Self {
            users: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn create_user(&self, req: CreateUserRequest) -> Result<User, String> {
        let mut users = self.users.lock().await;

        // Check if email already exists
        if users.values().any(|u| u.email == req.email) {
            return Err("Email already exists".to_string());
        }

        // Check if username already exists
        if users.values().any(|u| u.username == req.username) {
            return Err("Username already exists".to_string());
        }

        let user = User::new(req).map_err(|e| format!("Failed to create user: {}", e))?;
        let user_id = user.id.clone();
        users.insert(user_id, user.clone());

        Ok(user)
    }

    pub async fn get_user(&self, id: &str) -> Option<User> {
        let users = self.users.lock().await;
        users.get(id).cloned()
    }

    pub async fn get_user_by_email(&self, email: &str) -> Option<User> {
        let users = self.users.lock().await;
        users.values().find(|u| u.email == *email).cloned()
    }

    pub async fn update_user(&self, id: &str, req: UpdateUserRequest) -> Result<User, String> {
        let mut users = self.users.lock().await;

        if let Some(user) = users.get_mut(id) {
            // Check email uniqueness if changing email
            if let Some(ref new_email) = req.email {
                if users.values().any(|u| u.id != *id && u.email == *new_email) {
                    return Err("Email already exists".to_string());
                }
            }

            // Check username uniqueness if changing username
            if let Some(ref new_username) = req.username {
                if users.values().any(|u| u.id != *id && u.username == *new_username) {
                    return Err("Username already exists".to_string());
                }
            }

            user.update(req);
            Ok(user.clone())
        } else {
            Err("User not found".to_string())
        }
    }

    pub async fn delete_user(&self, id: &str) -> Result<(), String> {
        let mut users = self.users.lock().await;

        if users.remove(id).is_some() {
            Ok(())
        } else {
            Err("User not found".to_string())
        }
    }

    pub async fn list_users(&self, limit: Option<usize>, offset: Option<usize>) -> Vec<User> {
        let users = self.users.lock().await;
        let mut result: Vec<User> = users.values().cloned().collect();

        // Apply offset and limit
        let offset = offset.unwrap_or(0);
        let limit = limit.unwrap_or(result.len());

        result.into_iter().skip(offset).take(limit).collect()
    }

    pub async fn search_users(&self, query: &str) -> Vec<User> {
        let users = self.users.lock().await;
        let query_lower = query.to_lowercase();

        users.values()
            .filter(|u|
                u.email.to_lowercase().contains(&query_lower) ||
                u.username.to_lowercase().contains(&query_lower)
            )
            .cloned()
            .collect()
    }
}`,
      });
      await testApiClient.createFile(userService);

      await testApiClient.recordAiOutput(
        implementationWork.work.id,
        'Core user model and service implemented successfully.'
      );

      // === PHASE 4: Authentication System ===
      console.log('🔐 Phase 4: Authentication System');

      // Create authentication work session
      const authWork = await testStateManager.addWorkSession(
        testDataGenerator.generateWorkData({
          title: 'Implementation: Authentication System',
          tool_name: 'llm-agent',
          project_id: project.id,
        })
      );

      await testApiClient.addMessageToWork(
        authWork.work.id,
        testDataGenerator.generateMessageData({
          content: 'Implement JWT-based authentication system with login/register endpoints.',
          author_type: 'user',
        })
      );

      await testApiClient.createAiSession(
        authWork.work.id,
        testDataGenerator.generateAiSessionData()
      );
      await testApiClient.recordAiOutput(
        authWork.work.id,
        'Implementing JWT authentication system...'
      );

      // Implement auth service
      const authService = testDataGenerator.generateFileData({
        project_id: project.id,
        path: 'src/services/auth_service.rs',
        content: `use crate::models::user::{User, CreateUserRequest};
use crate::services::user_service::UserService;
use jsonwebtoken::{encode, decode, Header, Algorithm, Validation, EncodingKey, DecodingKey};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,  // user id
    pub email: String,
    pub role: String,
    pub exp: usize,   // expiration time
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserProfile,
}

#[derive(Debug, Serialize)]
pub struct UserProfile {
    pub id: String,
    pub email: String,
    pub username: String,
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

pub struct AuthService {
    user_service: UserService,
    jwt_secret: String,
}

impl AuthService {
    pub fn new(user_service: UserService) -> Self {
        Self {
            user_service,
            jwt_secret: "your-super-secret-jwt-key-change-in-production".to_string(),
        }
    }

    pub async fn register(&self, req: CreateUserRequest) -> Result<AuthResponse, String> {
        // Create user
        let user = self.user_service.create_user(req).await?;

        // Generate token
        let token = self.generate_token(&user)?;

        let profile = UserProfile {
            id: user.id,
            email: user.email,
            username: user.username,
            role: format!("{:?}", user.role),
        };

        Ok(AuthResponse { token, user: profile })
    }

    pub async fn login(&self, req: LoginRequest) -> Result<AuthResponse, String> {
        // Find user by email
        let user = self.user_service.get_user_by_email(&req.email).await
            .ok_or("Invalid credentials")?;

        // Verify password
        if !user.verify_password(&req.password).map_err(|e| e.to_string())? {
            return Err("Invalid credentials".to_string());
        }

        // Check if user is active
        if !user.is_active {
            return Err("Account is deactivated".to_string());
        }

        // Generate token
        let token = self.generate_token(&user)?;

        let profile = UserProfile {
            id: user.id,
            email: user.email,
            username: user.username,
            role: format!("{:?}", user.role),
        };

        Ok(AuthResponse { token, user: profile })
    }

    pub fn validate_token(&self, token: &str) -> Result<Claims, String> {
        let validation = Validation::new(Algorithm::HS256);
        let key = DecodingKey::from_secret(self.jwt_secret.as_ref());

        decode::<Claims>(token, &key, &validation)
            .map(|data| data.claims)
            .map_err(|e| format!("Invalid token: {}", e))
    }

    fn generate_token(&self, user: &User) -> Result<String, String> {
        let expiration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs() as usize + 24 * 3600; // 24 hours

        let claims = Claims {
            sub: user.id.clone(),
            email: user.email.clone(),
            role: format!("{:?}", user.role),
            exp: expiration,
        };

        let header = Header::new(Algorithm::HS256);
        let key = EncodingKey::from_secret(self.jwt_secret.as_ref());

        encode(&header, &claims, &key)
            .map_err(|e| format!("Failed to generate token: {}", e))
    }
}`,
      });
      await testApiClient.createFile(authService);

      await testApiClient.recordAiOutput(
        authWork.work.id,
        'JWT authentication system implemented.'
      );

      // === PHASE 5: API Routes and Integration ===
      console.log('🌐 Phase 5: API Routes and Integration');

      // Create API routes work session
      const apiWork = await testStateManager.addWorkSession(
        testDataGenerator.generateWorkData({
          title: 'Implementation: API Routes and Integration',
          tool_name: 'llm-agent',
          project_id: project.id,
        })
      );

      await testApiClient.addMessageToWork(
        apiWork.work.id,
        testDataGenerator.generateMessageData({
          content:
            'Create REST API routes for user management and authentication using Axum framework.',
          author_type: 'user',
        })
      );

      await testApiClient.createAiSession(
        apiWork.work.id,
        testDataGenerator.generateAiSessionData()
      );
      await testApiClient.recordAiOutput(apiWork.work.id, 'Creating REST API routes with Axum...');

      // Implement API routes
      const apiRoutes = testDataGenerator.generateFileData({
        project_id: project.id,
        path: 'src/routes.rs',
        content: `use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post, put, delete},
    Router,
};
use serde::Deserialize;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

use crate::models::user::{CreateUserRequest, UpdateUserRequest};
use crate::services::auth_service::{AuthService, LoginRequest};
use crate::services::user_service::UserService;

pub type AppState = Arc<AppServices>;

pub struct AppServices {
    pub user_service: UserService,
    pub auth_service: AuthService,
}

impl AppServices {
    pub fn new() -> Self {
        let user_service = UserService::new();
        let auth_service = AuthService::new(user_service.clone());

        Self {
            user_service,
            auth_service,
        }
    }
}

#[derive(Deserialize)]
pub struct PaginationQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

// Authentication routes
pub fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
}

async fn register(
    State(state): State<AppState>,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.auth_service.register(req).await {
        Ok(response) => Ok(Json(serde_json::json!({
            "success": true,
            "data": response
        }))),
        Err(err) => Err(StatusCode::BAD_REQUEST),
    }
}

async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.auth_service.login(req).await {
        Ok(response) => Ok(Json(serde_json::json!({
            "success": true,
            "data": response
        }))),
        Err(err) => Err(StatusCode::UNAUTHORIZED),
    }
}

// User management routes (protected)
pub fn user_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_users).post(create_user))
        .route("/:id", get(get_user).put(update_user).delete(delete_user))
        .route("/search", get(search_users))
}

async fn list_users(
    State(state): State<AppState>,
    Query(pagination): Query<PaginationQuery>,
) -> Json<serde_json::Value> {
    let users = state.user_service.list_users(pagination.limit, pagination.offset).await;
    Json(serde_json::json!({
        "success": true,
        "data": users
    }))
}

async fn get_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.user_service.get_user(&id).await {
        Some(user) => Ok(Json(serde_json::json!({
            "success": true,
            "data": user
        }))),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn create_user(
    State(state): State<AppState>,
    Json(req): Json<CreateUserRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.user_service.create_user(req).await {
        Ok(user) => Ok(Json(serde_json::json!({
            "success": true,
            "data": user
        }))),
        Err(err) => Err(StatusCode::BAD_REQUEST),
    }
}

async fn update_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<UpdateUserRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.user_service.update_user(&id, req).await {
        Ok(user) => Ok(Json(serde_json::json!({
            "success": true,
            "data": user
        }))),
        Err(err) => Err(StatusCode::BAD_REQUEST),
    }
}

async fn delete_user(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    match state.user_service.delete_user(&id).await {
        Ok(()) => Ok(Json(serde_json::json!({
            "success": true,
            "message": "User deleted successfully"
        }))),
        Err(err) => Err(StatusCode::NOT_FOUND),
    }
}

async fn search_users(
    State(state): State<AppState>,
    Query(search): Query<SearchQuery>,
) -> Json<serde_json::Value> {
    let users = state.user_service.search_users(&search.q).await;
    Json(serde_json::json!({
        "success": true,
        "data": users
    }))
}

// Main app router
pub fn create_router() -> Router<AppState> {
    let state = Arc::new(AppServices::new());

    Router::new()
        .nest("/auth", auth_routes())
        .nest("/users", user_routes())
        .layer(CorsLayer::permissive())
        .with_state(state)
}`,
      });
      await testApiClient.createFile(apiRoutes);

      // Update main.rs
      const updatedMain = testDataGenerator.generateFileData({
        project_id: project.id,
        path: 'src/main.rs',
        content: `mod models;
mod services;
mod routes;

use routes::create_router;

#[tokio::main]
async fn main() {
    let app = create_router();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("🚀 User Management API running on http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}`,
      });
      await testApiClient.createFile(updatedMain);

      // Update Cargo.toml with dependencies
      const updatedCargo = testDataGenerator.generateFileData({
        project_id: project.id,
        path: 'Cargo.toml',
        content: `[package]
name = "complex-feature"
version = "0.1.0"
edition = "2021"

[dependencies]
axum = "0.7"
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
bcrypt = "0.15"
jsonwebtoken = "9.0"
uuid = { version = "1.0", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
tower = "0.4"
tower-http = "0.5"`,
      });
      await testApiClient.updateFile('Cargo.toml', {
        content: updatedCargo.content,
        encoding: 'utf-8',
        project_id: project.id,
      });

      await testApiClient.recordAiOutput(
        apiWork.work.id,
        'REST API routes and main application integration completed.'
      );

      // === PHASE 6: Testing and Validation ===
      console.log('✅ Phase 6: Testing and Validation');

      // Create comprehensive tests
      const testFile = testDataGenerator.generateFileData({
        project_id: project.id,
        path: 'src/models/user_tests.rs',
        content: `#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_creation() {
        let req = CreateUserRequest {
            email: "test@example.com".to_string(),
            username: "testuser".to_string(),
            password: "password123".to_string(),
            role: Some(UserRole::User),
        };

        let user = User::new(req).unwrap();
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.username, "testuser");
        assert_eq!(user.role, UserRole::User);
        assert!(user.is_active);
    }

    #[test]
    fn test_password_verification() {
        let req = CreateUserRequest {
            email: "test@example.com".to_string(),
            username: "testuser".to_string(),
            password: "password123".to_string(),
            role: None,
        };

        let user = User::new(req).unwrap();
        assert!(user.verify_password("password123").unwrap());
        assert!(!user.verify_password("wrongpassword").unwrap());
    }

    #[test]
    fn test_user_update() {
        let mut user = User::new(CreateUserRequest {
            email: "test@example.com".to_string(),
            username: "testuser".to_string(),
            password: "password123".to_string(),
            role: None,
        }).unwrap();

        let update_req = UpdateUserRequest {
            email: Some("newemail@example.com".to_string()),
            username: Some("newusername".to_string()),
            role: Some(UserRole::Admin),
            is_active: Some(false),
        };

        user.update(update_req);

        assert_eq!(user.email, "newemail@example.com");
        assert_eq!(user.username, "newusername");
        assert_eq!(user.role, UserRole::Admin);
        assert!(!user.is_active);
    }
}`,
      });
      await testApiClient.createFile(testFile);

      await testApiClient.recordAiOutput(
        apiWork.work.id,
        'Comprehensive unit tests added for user management system.'
      );

      // === PHASE 7: Documentation and Finalization ===
      console.log('📚 Phase 7: Documentation and Finalization');

      // Update README with comprehensive documentation
      const finalReadme = testDataGenerator.generateFileData({
        project_id: project.id,
        path: 'README.md',
        content: `# Complex Feature Development - User Management System

A complete user management system with authentication, role-based access control, and comprehensive API endpoints.

## Features

- ✅ User registration and authentication
- ✅ JWT token-based authorization
- ✅ Role-based access control (Admin, Moderator, User)
- ✅ User profile management
- ✅ User search and filtering
- ✅ Account activation/deactivation
- ✅ Comprehensive REST API
- ✅ Unit tests

## API Endpoints

### Authentication
- \`POST /auth/register\` - Register a new user
- \`POST /auth/login\` - Login with email/password

### User Management
- \`GET /users\` - List all users (with pagination)
- \`GET /users/:id\` - Get user by ID
- \`POST /users\` - Create new user
- \`PUT /users/:id\` - Update user
- \`DELETE /users/:id\` - Delete user
- \`GET /users/search?q=query\` - Search users

## Usage Examples

### Register User
\`\`\`bash
curl -X POST http://localhost:3000/auth/register \\
  -H "Content-Type: application/json" \\
  -d '{
    "email": "user@example.com",
    "username": "testuser",
    "password": "password123",
    "role": "User"
  }'
\`\`\`

### Login
\`\`\`\`bash
curl -X POST http://localhost:3000/auth/login \\
  -H "Content-Type: application/json" \\
  -d '{
    "email": "user@example.com",
    "password": "password123"
  }'
\`\`\`

### List Users
\`\`\`bash
curl -X GET http://localhost:3000/users \\
  -H "Authorization: Bearer YOUR_JWT_TOKEN"
\`\`\`

## Running the Application

\`\`\`bash
cargo run
\`\`\`

## Running Tests

\`\`\`bash
cargo test
\`\`\`

## Architecture

- **Models**: User entity with role-based permissions
- **Services**: Business logic layer (UserService, AuthService)
- **Routes**: REST API endpoints with Axum
- **Security**: bcrypt password hashing, JWT authentication
- **Database**: In-memory storage (easily replaceable with persistent storage)

## Development Workflow

This project was developed using a complete LLM agent workflow:

1. **Planning**: Requirements analysis and design
2. **Implementation**: Step-by-step feature development
3. **Testing**: Unit tests and integration validation
4. **Documentation**: Comprehensive API documentation

## Security Features

- Password hashing with bcrypt
- JWT tokens with expiration
- Input validation and sanitization
- Role-based access control
- Account activation/deactivation

## Future Enhancements

- Database persistence (PostgreSQL/SQLite)
- Email verification
- Password reset functionality
- Rate limiting
- API documentation with OpenAPI/Swagger
- Docker containerization`,
      });
      await testApiClient.updateFile('README.md', {
        content: finalReadme.content,
        encoding: 'utf-8',
        project_id: project.id,
      });

      // Final validation
      const finalState = testStateManager.validateStateConsistency();
      expect(finalState.valid).toBe(true);

      const finalSummary = testStateManager.getStateSummary();
      expect(finalSummary.projects).toBeGreaterThan(0);
      expect(finalSummary.workSessions).toBeGreaterThan(2); // planning, implementation, auth, api

      console.log('🎉 Complex workflow test completed successfully!');
      console.log(`📊 Final state: ${JSON.stringify(finalSummary, null, 2)}`);
    });
  });

  describe('Error Recovery and Resilience', () => {
    it('should handle partial failures and recover gracefully', async () => {
      // Create project
      const project = await testStateManager.addProject(
        testDataGenerator.generateProjectData({
          name: 'Error Recovery Test',
        })
      );

      // Create work session
      const work = await testStateManager.addWorkSession(
        testDataGenerator.generateWorkData({
          title: 'Error recovery workflow',
          project_id: project.id,
        })
      );

      // Simulate partial success scenario
      await testApiClient.recordAiOutput(work.work.id, 'Starting error recovery test...');

      // Create some files successfully
      const successFile1 = testDataGenerator.generateFileData({
        project_id: project.id,
        path: 'success-1.txt',
        content: 'This file was created successfully',
      });
      await testApiClient.createFile(successFile1);

      // Simulate failure (try to create file with invalid path)
      try {
        const invalidFile = testDataGenerator.generateFileData({
          project_id: project.id,
          path: '../../../invalid-path.txt', // Invalid path
          content: 'This should fail',
        });
        await testApiClient.createFile(invalidFile);
        expect.fail('Should have rejected invalid file path');
      } catch (error) {
        await testApiClient.recordAiOutput(
          work.work.id,
          'Handled invalid file path error gracefully'
        );
      }

      // Continue with successful operations
      const successFile2 = testDataGenerator.generateFileData({
        project_id: project.id,
        path: 'success-2.txt',
        content: 'This file was created after error recovery',
      });
      await testApiClient.createFile(successFile2);

      await testApiClient.recordAiOutput(
        work.work.id,
        'Workflow resumed successfully after error handling'
      );

      // Verify final state
      const fileList = await testApiClient.listFiles({ project_id: project.id });
      const successFiles = fileList.files.filter(f => f.name.startsWith('success-'));
      expect(successFiles.length).toBe(2);

      const stateValidation = testStateManager.validateStateConsistency();
      expect(stateValidation.valid).toBe(true);
    });

    it('should handle concurrent workflow conflicts', async () => {
      // Create project
      const project = await testStateManager.addProject(
        testDataGenerator.generateProjectData({
          name: 'Concurrency Test',
        })
      );

      // Create multiple work sessions trying to modify the same file
      const work1 = await testStateManager.addWorkSession(
        testDataGenerator.generateWorkData({
          title: 'Concurrent workflow 1',
          project_id: project.id,
        })
      );

      const work2 = await testStateManager.addWorkSession(
        testDataGenerator.generateWorkData({
          title: 'Concurrent workflow 2',
          project_id: project.id,
        })
      );

      // Both workflows try to create the same file
      const sharedFile = 'shared-file.txt';

      // Start both operations
      const operation1 = testApiClient.createFile(
        testDataGenerator.generateFileData({
          project_id: project.id,
          path: sharedFile,
          content: 'Content from workflow 1',
        })
      );

      const operation2 = testApiClient.createFile(
        testDataGenerator.generateFileData({
          project_id: project.id,
          path: sharedFile,
          content: 'Content from workflow 2',
        })
      );

      // One should succeed, one should fail (depending on API implementation)
      let successCount = 0;
      let failureCount = 0;

      try {
        await operation1;
        successCount++;
        await testApiClient.recordAiOutput(work1.work.id, 'Successfully created shared file');
      } catch (error) {
        failureCount++;
        await testApiClient.recordAiOutput(
          work1.work.id,
          'File creation failed - concurrent conflict'
        );
      }

      try {
        await operation2;
        successCount++;
        await testApiClient.recordAiOutput(work2.work.id, 'Successfully created shared file');
      } catch (error) {
        failureCount++;
        await testApiClient.recordAiOutput(
          work2.work.id,
          'File creation failed - concurrent conflict'
        );
      }

      // Either both succeed (if API allows overwrites) or one succeeds and one fails
      expect(successCount + failureCount).toBe(2);
      expect(successCount).toBeGreaterThan(0); // At least one should succeed

      // Verify final state
      const fileList = await testApiClient.listFiles({ project_id: project.id });
      const sharedFileExists = fileList.files.some(f => f.name === sharedFile);
      expect(sharedFileExists).toBe(true);
    });
  });

  describe('Performance Under Load', () => {
    it('should handle high-frequency operations efficiently', async () => {
      // Create project
      const project = await testStateManager.addProject(
        testDataGenerator.generateProjectData({
          name: 'Performance Load Test',
        })
      );

      const work = await testStateManager.addWorkSession(
        testDataGenerator.generateWorkData({
          title: 'High-frequency operations test',
          project_id: project.id,
        })
      );

      await testApiClient.recordAiOutput(
        work.work.id,
        'Starting high-frequency operations test...'
      );

      const operationCount = 50;
      const operations = [];

      // Create many files rapidly
      for (let i = 0; i < operationCount; i++) {
        const fileData = testDataGenerator.generateFileData({
          project_id: project.id,
          path: `load-test-${i.toString().padStart(3, '0')}.txt`,
          content: `Load test file ${i}\nTimestamp: ${Date.now()}\nData: ${'x'.repeat(100)}`,
        });
        operations.push(testApiClient.createFile(fileData));
      }

      const startTime = Date.now();
      await Promise.all(operations);
      const endTime = Date.now();
      const duration = endTime - startTime;

      await testApiClient.recordAiOutput(
        work.work.id,
        `Completed ${operationCount} operations in ${duration}ms`
      );

      // Verify all operations completed
      const fileList = await testApiClient.listFiles({ project_id: project.id });
      const loadTestFiles = fileList.files.filter(f => f.name.startsWith('load-test-'));
      expect(loadTestFiles.length).toBe(operationCount);

      // Performance should be reasonable (under 30 seconds for 50 operations)
      expect(duration).toBeLessThan(30000);

      // Calculate operations per second
      const opsPerSecond = (operationCount / duration) * 1000;
      console.log(`Performance: ${opsPerSecond.toFixed(2)} operations/second`);

      // Record performance metrics
      await testApiClient.recordAiOutput(
        work.work.id,
        `Performance metrics: ${opsPerSecond.toFixed(2)} ops/sec, ${duration}ms total`
      );
    });

    it('should maintain state consistency during rapid updates', async () => {
      // Create project and file
      const project = await testStateManager.addProject(
        testDataGenerator.generateProjectData({
          name: 'State Consistency Test',
        })
      );

      const initialContent = 'Initial content';
      const testFile = testDataGenerator.generateFileData({
        project_id: project.id,
        path: 'consistency-test.txt',
        content: initialContent,
      });
      await testApiClient.createFile(testFile);

      const work = await testStateManager.addWorkSession(
        testDataGenerator.generateWorkData({
          title: 'State consistency during rapid updates',
          project_id: project.id,
        })
      );

      await testApiClient.recordAiOutput(work.work.id, 'Starting rapid update consistency test...');

      // Perform rapid updates to the same file
      const updateCount = 20;
      const updates = [];

      for (let i = 0; i < updateCount; i++) {
        const newContent = `Update ${i + 1}: ${Date.now()}\n${'x'.repeat(200)}`;
        updates.push(
          testApiClient.updateFile('consistency-test.txt', {
            content: newContent,
            encoding: 'utf-8',
            project_id: project.id,
          })
        );
      }

      const startTime = Date.now();
      await Promise.all(updates);
      const endTime = Date.now();
      const duration = endTime - startTime;

      // Verify final state
      const finalContent = await testApiClient.getFileContent('consistency-test.txt', project.id);
      expect(finalContent.content).toContain('Update 20:'); // Last update should be present

      // Verify state consistency
      const stateValidation = testStateManager.validateStateConsistency();
      expect(stateValidation.valid).toBe(true);

      await testApiClient.recordAiOutput(
        work.work.id,
        `State consistency maintained: ${updateCount} rapid updates in ${duration}ms`
      );

      // Performance check
      expect(duration).toBeLessThan(15000); // Should complete in under 15 seconds
    });
  });
});
