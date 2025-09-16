import { afterAll, beforeAll, beforeEach, describe, expect, it } from 'vitest';
import { testApiClient } from '../setup/api-client';
import { testServer } from '../setup/test-server';
import { testDatabase } from '../setup/test-database';
import { testDataGenerator } from '../setup/test-data';
import { wsTestManager } from '../utils/websocket-client';

describe('End-to-End Workflow - API Only', () => {
  beforeAll(async () => {
    await testDatabase.setupTestDatabase();
    await testServer.startServer();
  }, 60000); // Extended timeout for full workflow

  afterAll(async () => {
    await wsTestManager.disconnectAll();
    await testServer.stopServer();
    await testDatabase.cleanupTestDatabase();
  });

  beforeEach(async () => {
    testDataGenerator.reset();
    await wsTestManager.disconnectAll();
  });

  describe('Complete LLM Agent Development Workflow', () => {
    it('should execute full project creation to deployment workflow', async () => {
      // === PHASE 1: Project Setup ===
      console.log('🚀 Phase 1: Project Setup');

      // Create project
      const projectData = testDataGenerator.generateProjectData({
        language: 'rust',
        description: 'Complete end-to-end workflow test project',
      });
      const project = await testApiClient.createProject(projectData);
      expect(project.id).toBeDefined();
      expect(project.name).toBe(projectData.name);

      // Create initial project structure
      const initialFiles = [
        testDataGenerator.generateFileData({
          project_id: project.id,
          path: 'README.md',
          content: '# E2E Test Project\n\nA complete workflow test.',
        }),
        testDataGenerator.generateFileData({
          project_id: project.id,
          path: 'Cargo.toml',
          content: `[package]
name = "e2e-test-project"
version = "0.1.0"
edition = "2021"`,
        }),
        testDataGenerator.generateFileData({
          project_id: project.id,
          path: 'src/main.rs',
          content: 'fn main() {\n    println!("Hello, World!");\n}',
        }),
      ];

      await Promise.all(initialFiles.map(file => testApiClient.createFile(file)));

      // Verify project structure
      const fileList = await testApiClient.listFiles({ project_id: project.id });
      expect(fileList.files.length).toBeGreaterThanOrEqual(3);

      // === PHASE 2: Work Session Creation ===
      console.log('🔧 Phase 2: Work Session Creation');

      // Create work session
      const workData = testDataGenerator.generateWorkData({
        title: 'Implement user authentication feature',
        tool_name: 'llm-agent',
        project_id: project.id,
      });
      const work = await testApiClient.createWork(workData);
      const workId = work.work.id;

      // Add initial user prompt
      const userPrompt = testDataGenerator.generateMessageData({
        content: `Please implement a user authentication system for this Rust project.
        Include:
        1. User registration and login
        2. Password hashing
        3. JWT token generation
        4. Basic middleware for protected routes

        Start by analyzing the current codebase and then implement the features.`,
        author_type: 'user',
      });
      await testApiClient.addMessageToWork(workId, userPrompt);

      // === PHASE 3: LLM Agent Analysis ===
      console.log('🤖 Phase 3: LLM Agent Analysis');

      // Create AI session
      const aiSessionData = testDataGenerator.generateAiSessionData({
        tool_name: 'llm-agent',
        project_context: `Project: ${project.name} (${project.language})`,
      });
      const aiSession = await testApiClient.createAiSession(workId, aiSessionData);

      // Simulate LLM agent analyzing the codebase
      await testApiClient.recordAiOutput(workId, 'Analyzing project structure...');

      // LLM agent reads key files
      const readmeContent = await testApiClient.getFileContent('README.md', project.id);
      const cargoContent = await testApiClient.getFileContent('Cargo.toml', project.id);
      const mainContent = await testApiClient.getFileContent('src/main.rs', project.id);

      await testApiClient.recordAiOutput(
        workId,
        `Analysis complete. Found Rust project with basic structure. README: ${readmeContent.content.substring(0, 50)}...`
      );

      // === PHASE 4: Implementation Planning ===
      console.log('📋 Phase 4: Implementation Planning');

      // LLM agent creates implementation plan
      const planContent = `# Implementation Plan

## Current State
- Basic Rust project with Cargo.toml and main.rs
- No authentication system implemented

## Required Components
1. **Dependencies**: Add authentication-related crates
2. **Models**: User model with password hashing
3. **Services**: Auth service for registration/login
4. **Middleware**: JWT validation middleware
5. **Routes**: Auth endpoints (register, login, logout)

## Implementation Steps
1. Update Cargo.toml with dependencies
2. Create user model and auth structures
3. Implement password hashing
4. Create auth service
5. Add middleware
6. Create auth routes
7. Update main.rs to include routes`;

      const planFile = testDataGenerator.generateFileData({
        project_id: project.id,
        path: 'IMPLEMENTATION_PLAN.md',
        content: planContent,
      });
      await testApiClient.createFile(planFile);

      await testApiClient.recordAiOutput(
        workId,
        'Implementation plan created. Starting with dependencies...'
      );

      // === PHASE 5: Dependency Management ===
      console.log('📦 Phase 5: Dependency Management');

      // Update Cargo.toml with authentication dependencies
      const updatedCargoContent = `[package]
name = "e2e-test-project"
version = "0.1.0"
edition = "2021"

[dependencies]
axum = "0.7"
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
bcrypt = "0.15"
jsonwebtoken = "9.0"
tower = "0.4"
tower-http = "0.5"`;

      await testApiClient.updateFile('Cargo.toml', {
        content: updatedCargoContent,
        encoding: 'utf-8',
        project_id: project.id,
      });

      await testApiClient.recordAiOutput(
        workId,
        'Dependencies updated. Added authentication crates.'
      );

      // === PHASE 6: Core Implementation ===
      console.log('⚙️ Phase 6: Core Implementation');

      // Create user model
      const userModelContent = `use serde::{Deserialize, Serialize};
use bcrypt::{hash, verify, DEFAULT_COST};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub password_hash: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserResponse,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
}

impl User {
    pub fn new(email: String, password: String) -> Result<Self, Box<dyn std::error::Error>> {
        let password_hash = hash(password, DEFAULT_COST)?;

        Ok(Self {
            id: uuid::Uuid::new_v4().to_string(),
            email,
            password_hash,
            created_at: chrono::Utc::now(),
        })
    }

    pub fn verify_password(&self, password: &str) -> Result<bool, bcrypt::BcryptError> {
        verify(password, &self.password_hash)
    }
}`;

      const userModelFile = testDataGenerator.generateFileData({
        project_id: project.id,
        path: 'src/models.rs',
        content: userModelContent,
      });
      await testApiClient.createFile(userModelFile);

      // Create auth service
      const authServiceContent = `use crate::models::{User, RegisterRequest, LoginRequest, AuthResponse};
use jsonwebtoken::{encode, Header, EncodingKey};
use std::collections::HashMap;

pub struct AuthService {
    users: HashMap<String, User>, // In production, use a database
    jwt_secret: String,
}

impl AuthService {
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
            jwt_secret: "your-secret-key".to_string(), // In production, use env var
        }
    }

    pub async fn register(&mut self, req: RegisterRequest) -> Result<AuthResponse, String> {
        // Check if user already exists
        if self.users.values().any(|u| u.email == req.email) {
            return Err("User already exists".to_string());
        }

        // Create new user
        let user = User::new(req.email.clone(), req.password)
            .map_err(|e| format!("Failed to create user: {}", e))?;

        let user_id = user.id.clone();
        self.users.insert(user_id.clone(), user);

        // Generate JWT token
        let token = self.generate_token(&user_id)?;

        let user_response = models::UserResponse {
            id: user_id.clone(),
            email: req.email,
        };

        Ok(AuthResponse {
            token,
            user: user_response,
        })
    }

    pub async fn login(&self, req: LoginRequest) -> Result<AuthResponse, String> {
        // Find user by email
        let user = self.users.values()
            .find(|u| u.email == req.email)
            .ok_or("User not found")?;

        // Verify password
        if !user.verify_password(&req.password).map_err(|e| e.to_string())? {
            return Err("Invalid password".to_string());
        }

        // Generate JWT token
        let token = self.generate_token(&user.id)?;

        let user_response = models::UserResponse {
            id: user.id.clone(),
            email: user.email.clone(),
        };

        Ok(AuthResponse {
            token,
            user: user_response,
        })
    }

    fn generate_token(&self, user_id: &str) -> Result<String, String> {
        // Simplified JWT generation - in production, add proper claims
        encode(
            &Header::default(),
            &jsonwebtoken::claims::Claims::new(user_id.to_string()),
            &EncodingKey::from_secret(self.jwt_secret.as_ref()),
        ).map_err(|e| e.to_string())
    }
}`;

      const authServiceFile = testDataGenerator.generateFileData({
        project_id: project.id,
        path: 'src/auth.rs',
        content: authServiceContent,
      });
      await testApiClient.createFile(authServiceFile);

      await testApiClient.recordAiOutput(
        workId,
        'Core authentication models and service implemented.'
      );

      // === PHASE 7: Middleware and Routes ===
      console.log('🔒 Phase 7: Middleware and Routes');

      // Create middleware
      const middlewareContent = `use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, Validation, DecodingKey};

pub async fn auth_middleware(
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract token from Authorization header
    let auth_header = req.headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "));

    let token = match auth_header {
        Some(token) => token,
        None => return Err(StatusCode::UNAUTHORIZED),
    };

    // Verify token (simplified - in production, validate properly)
    match decode::<String>(
        token,
        &DecodingKey::from_secret("your-secret-key".as_ref()),
        &Validation::default(),
    ) {
        Ok(_) => Ok(next.run(req).await),
        Err(_) => Err(StatusCode::UNAUTHORIZED),
    }
}`;

      const middlewareFile = testDataGenerator.generateFileData({
        project_id: project.id,
        path: 'src/middleware.rs',
        content: middlewareContent,
      });
      await testApiClient.createFile(middlewareFile);

      // Create routes
      const routesContent = `use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::auth::AuthService;
use crate::models::{RegisterRequest, LoginRequest};

pub fn auth_routes() -> Router<Arc<Mutex<AuthService>>> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/protected", get(protected_route))
}

async fn register(
    State(auth_service): State<Arc<Mutex<AuthService>>>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut service = auth_service.lock().await;

    match service.register(req).await {
        Ok(response) => Ok(Json(serde_json::json!({
            "success": true,
            "data": response
        }))),
        Err(err) => Err(StatusCode::BAD_REQUEST),
    }
}

async fn login(
    State(auth_service): State<Arc<Mutex<AuthService>>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let service = auth_service.lock().await;

    match service.login(req).await {
        Ok(response) => Ok(Json(serde_json::json!({
            "success": true,
            "data": response
        }))),
        Err(err) => Err(StatusCode::UNAUTHORIZED),
    }
}

async fn protected_route() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "message": "You have access to this protected route!"
    }))
}`;

      const routesFile = testDataGenerator.generateFileData({
        project_id: project.id,
        path: 'src/routes.rs',
        content: routesContent,
      });
      await testApiClient.createFile(routesFile);

      await testApiClient.recordAiOutput(
        workId,
        'Authentication routes and middleware implemented.'
      );

      // === PHASE 8: Main Application Update ===
      console.log('🚀 Phase 8: Main Application Update');

      // Update main.rs to include the authentication system
      const updatedMainContent = `mod models;
mod auth;
mod middleware;
mod routes;

use axum::{
    middleware as axum_middleware,
    routing::get,
    Router,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;

use auth::AuthService;
use routes::auth_routes;

#[tokio::main]
async fn main() {
    // Initialize auth service
    let auth_service = Arc::new(Mutex::new(AuthService::new()));

    // Build application with routes
    let app = Router::new()
        .route("/", get(|| async { "Authentication API Server" }))
        .nest("/auth", auth_routes())
        .layer(CorsLayer::permissive())
        .with_state(auth_service);

    // Run server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("🚀 Server running on http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}`;

      await testApiClient.updateFile('src/main.rs', {
        content: updatedMainContent,
        encoding: 'utf-8',
        project_id: project.id,
      });

      await testApiClient.recordAiOutput(
        workId,
        'Main application updated with authentication system.'
      );

      // === PHASE 9: Testing and Validation ===
      console.log('✅ Phase 9: Testing and Validation');

      // Create a simple test file
      const testContent = `#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_creation() {
        let user = User::new(
            "test@example.com".to_string(),
            "password123".to_string()
        ).unwrap();

        assert_eq!(user.email, "test@example.com");
        assert!(!user.password_hash.is_empty());
    }

    #[test]
    fn test_password_verification() {
        let user = User::new(
            "test@example.com".to_string(),
            "password123".to_string()
        ).unwrap();

        assert!(user.verify_password("password123").unwrap());
        assert!(!user.verify_password("wrongpassword").unwrap());
    }
}`;

      const testFile = testDataGenerator.generateFileData({
        project_id: project.id,
        path: 'src/models_tests.rs',
        content: testContent,
      });
      await testApiClient.createFile(testFile);

      await testApiClient.recordAiOutput(
        workId,
        'Basic unit tests added for authentication models.'
      );

      // === PHASE 10: Documentation Update ===
      console.log('📚 Phase 10: Documentation Update');

      // Update README with authentication documentation
      const updatedReadmeContent = `# E2E Test Project

A complete workflow test project with user authentication system.

## Features

- User registration and login
- Password hashing with bcrypt
- JWT token authentication
- Protected routes with middleware
- Axum web framework

## API Endpoints

### Authentication
- \`POST /auth/register\` - Register a new user
- \`POST /auth/login\` - Login with existing credentials
- \`GET /auth/protected\` - Access protected route (requires JWT token)

### Usage Example

\`\`\`bash
# Register a new user
curl -X POST http://localhost:3000/auth/register \\
  -H "Content-Type: application/json" \\
  -d '{"email": "user@example.com", "password": "password123"}'

# Login
curl -X POST http://localhost:3000/auth/login \\
  -H "Content-Type: application/json" \\
  -d '{"email": "user@example.com", "password": "password123"}'

# Access protected route
curl -X GET http://localhost:3000/auth/protected \\
  -H "Authorization: Bearer YOUR_JWT_TOKEN"
\`\`\`

## Running the Application

\`\`\`bash
cargo run
\`\`\`

The server will start on http://localhost:3000

## Testing

\`\`\`bash
cargo test
\`\`\``;

      await testApiClient.updateFile('README.md', {
        content: updatedReadmeContent,
        encoding: 'utf-8',
        project_id: project.id,
      });

      await testApiClient.recordAiOutput(
        workId,
        'Documentation updated with authentication API details.'
      );

      // === PHASE 11: Final Verification ===
      console.log('🎉 Phase 11: Final Verification');

      // Verify all files exist
      const finalFileList = await testApiClient.listFiles({ project_id: project.id });
      expect(finalFileList.files.length).toBeGreaterThanOrEqual(8);

      // Verify key files
      const fileNames = finalFileList.files.map(f => f.name);
      expect(fileNames).toContain('README.md');
      expect(fileNames).toContain('Cargo.toml');
      expect(fileNames).toContain('src');

      // Verify work session has complete history
      const finalWorkSession = await testApiClient.getWork(workId);
      expect(finalWorkSession.messages.length).toBeGreaterThan(0);

      // Verify AI outputs were recorded
      const outputs = await testApiClient.listAiOutputs(workId);
      expect(outputs.outputs.length).toBeGreaterThan(5); // Multiple AI responses recorded

      await testApiClient.recordAiOutput(
        workId,
        '🎉 Authentication system implementation complete! All features successfully implemented.'
      );

      console.log('✅ End-to-End Workflow Test Completed Successfully!');
    });
  });

  describe('Workflow Error Handling and Recovery', () => {
    it('should handle and recover from workflow interruptions', async () => {
      // Create project and work session
      const projectData = testDataGenerator.generateProjectData({
        description: 'Error recovery test project',
      });
      const project = await testApiClient.createProject(projectData);

      const workData = testDataGenerator.generateWorkData({
        title: 'Test error recovery workflow',
        project_id: project.id,
      });
      const work = await testApiClient.createWork(workData);
      const workId = work.work.id;

      // Simulate workflow starting
      await testApiClient.recordAiOutput(workId, 'Starting error recovery test workflow...');

      // Simulate an error scenario (trying to access non-existent file)
      try {
        await testApiClient.getFileContent('non-existent-file.txt', project.id);
        expect.fail('Should have thrown error for non-existent file');
      } catch (error) {
        // Expected error - continue workflow
        await testApiClient.recordAiOutput(workId, 'Handled error gracefully: File not found');
      }

      // Continue with recovery - create the missing file
      const recoveryFile = testDataGenerator.generateFileData({
        project_id: project.id,
        path: 'error-recovery-log.txt',
        content: 'Error recovery successful - file created after error handling',
      });
      await testApiClient.createFile(recoveryFile);

      await testApiClient.recordAiOutput(
        workId,
        'Error recovery complete - workflow resumed successfully'
      );

      // Verify recovery was successful
      const recoveryFileContent = await testApiClient.getFileContent(
        'error-recovery-log.txt',
        project.id
      );
      expect(recoveryFileContent.content).toContain('Error recovery successful');
    });

    it('should handle concurrent workflow operations', async () => {
      // Create multiple projects and work sessions
      const projects = [];
      const works = [];

      for (let i = 0; i < 3; i++) {
        const projectData = testDataGenerator.generateProjectData({
          name: `Concurrent Project ${i + 1}`,
        });
        const project = await testApiClient.createProject(projectData);
        projects.push(project);

        const workData = testDataGenerator.generateWorkData({
          title: `Concurrent Workflow ${i + 1}`,
          project_id: project.id,
        });
        const work = await testApiClient.createWork(workData);
        works.push(work.work);
      }

      // Perform concurrent operations on all workflows
      const concurrentOperations = works.map(async (work, index) => {
        const workId = work.id;

        // Add message
        await testApiClient.addMessageToWork(
          workId,
          testDataGenerator.generateMessageData({
            content: `Concurrent operation ${index + 1}`,
          })
        );

        // Create AI session
        await testApiClient.createAiSession(workId, testDataGenerator.generateAiSessionData());

        // Record output
        await testApiClient.recordAiOutput(workId, `Concurrent AI output ${index + 1}`);

        // Create file
        const fileData = testDataGenerator.generateFileData({
          project_id: projects[index].id,
          path: `concurrent-file-${index + 1}.txt`,
          content: `Content for concurrent file ${index + 1}`,
        });
        await testApiClient.createFile(fileData);
      });

      // Execute all concurrent operations
      await Promise.all(concurrentOperations);

      // Verify all operations completed successfully
      for (let i = 0; i < works.length; i++) {
        const workSession = await testApiClient.getWork(works[i].id);
        expect(workSession.messages.length).toBeGreaterThan(0);

        const outputs = await testApiClient.listAiOutputs(works[i].id);
        expect(outputs.outputs.length).toBeGreaterThan(0);

        // Verify file was created
        const fileContent = await testApiClient.getFileContent(
          `concurrent-file-${i + 1}.txt`,
          projects[i].id
        );
        expect(fileContent.content).toContain(`concurrent file ${i + 1}`);
      }
    });
  });

  describe('Performance and Scalability Validation', () => {
    it('should handle high-volume file operations', async () => {
      // Create project for performance testing
      const projectData = testDataGenerator.generateProjectData({
        description: 'Performance test project',
      });
      const project = await testApiClient.createProject(projectData);

      const workData = testDataGenerator.generateWorkData({
        title: 'High-volume file operations test',
        project_id: project.id,
      });
      const work = await testApiClient.createWork(workData);
      const workId = work.work.id;

      // Create large number of files
      const fileCount = 20;
      const fileOperations = [];

      for (let i = 0; i < fileCount; i++) {
        const fileData = testDataGenerator.generateFileData({
          project_id: project.id,
          path: `perf-file-${i.toString().padStart(3, '0')}.txt`,
          content: `Performance test file ${i}\n${'x'.repeat(1000)}`, // 1KB content each
        });
        fileOperations.push(testApiClient.createFile(fileData));
      }

      // Execute all file creation operations
      const startTime = Date.now();
      await Promise.all(fileOperations);
      const endTime = Date.now();

      const duration = endTime - startTime;
      console.log(`Created ${fileCount} files in ${duration}ms`);

      // Record performance result
      await testApiClient.recordAiOutput(
        workId,
        `Performance test: Created ${fileCount} files in ${duration}ms`
      );

      // Verify all files were created
      const fileList = await testApiClient.listFiles({ project_id: project.id });
      const perfFiles = fileList.files.filter(f => f.name.startsWith('perf-file-'));
      expect(perfFiles.length).toBe(fileCount);

      // Verify performance is reasonable (should complete in reasonable time)
      expect(duration).toBeLessThan(30000); // Should complete in less than 30 seconds
    });

    it('should maintain data consistency across rapid operations', async () => {
      // Create project and work session
      const projectData = testDataGenerator.generateProjectData({
        description: 'Consistency test project',
      });
      const project = await testApiClient.createProject(projectData);

      const workData = testDataGenerator.generateWorkData({
        title: 'Data consistency test',
        project_id: project.id,
      });
      const work = await testApiClient.createWork(workData);
      const workId = work.work.id;

      // Perform rapid create/update/delete operations
      const testFilePath = 'consistency-test.txt';

      // Create file
      const createData = testDataGenerator.generateFileData({
        project_id: project.id,
        path: testFilePath,
        content: 'Initial content',
      });
      await testApiClient.createFile(createData);

      // Update file multiple times rapidly
      for (let i = 1; i <= 5; i++) {
        await testApiClient.updateFile(testFilePath, {
          content: `Updated content v${i}`,
          encoding: 'utf-8',
          project_id: project.id,
        });
      }

      // Verify final state is consistent
      const finalContent = await testApiClient.getFileContent(testFilePath, project.id);
      expect(finalContent.content).toBe('Updated content v5');

      // Record consistency result
      await testApiClient.recordAiOutput(
        workId,
        'Data consistency maintained across rapid operations'
      );

      // Clean up
      await testApiClient.deleteFile(testFilePath, project.id);

      // Verify deletion
      await expect(testApiClient.getFileContent(testFilePath, project.id)).rejects.toThrow();
    });
  });
});
