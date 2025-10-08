# Manager App Specification

## Overview

The Manager app is a Linux daemon that runs on the Operator server, orchestrating the entire software development lifecycle from idea to deployment. It serves as the central coordinator between the Manager Web app and various development tools. Written in Rust, it provides secure communication channels, system management, and development environment orchestration.

## Architecture

### Core Components

1. **System Orchestrator** - Manages server state, services, and system configuration
2. **Communication Hub** - Handles HTTP communication
3. **Development Environment Manager** - Installs and manages development tools
4. **Process Manager** - Manages long-running processes and services
5. **Project Manager** - Handles project lifecycle and file system operations
6. **Security Manager** - Manages authentication, authorization, and security policies
7. **Logging & Monitoring** - Comprehensive logging and system monitoring

### Technology Stack

- **Language**: Rust
- **Async Runtime**: Tokio
- **Web Framework**: Actix Web (for HTTP API)
- **Process Management**: tokio-process, systemd integration
- **File System**: tokio-fs, inotify for file watching
- **Configuration**: serde with TOML/YAML support
- **Logging**: tracing + tracing-subscriber
- **Database**: SQLite for local state management
- **Type Generation**: ts-rs for TypeScript type generation

## System Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Manager Daemon                       │
├─────────────────┬─────────────────┬────────────────────┤
│   HTTP Server   │   System Services  │   Development      │
│   (Web API)     │   Manager          │   Tools & Processes│
├─────────────────┼─────────────────┼────────────────────┤
│                 │                 │                    │
│  Manager Web    │   Development      │   Development      │
│  ←→ HTTP/WS     │   Tools & Processes│   Tools & Processes│
│                 │                 │                    │
└─────────────────┴─────────────────┴────────────────────┘
```

## Core Features

### 1. Project Management

**Responsibilities**:
- Project CRUD operations and lifecycle management
- File system operations and project structure
- Project templates and scaffolding
- Project analysis and metadata management

```rust
#[derive(Debug, Clone)]
pub struct ProjectManager {
    projects: HashMap<String, Project>,
    templates: TemplateRegistry,
    file_system: FileSystemManager,
}

impl ProjectManager {
    pub async fn create_project(&self, request: CreateProjectRequest) -> Result<Project, ProjectError> {
        // Create new project with scaffolding
        self.setup_project_structure(&request.name, &request.path).await?;
        self.initialize_git_repo(&request.path).await?;
        Ok(Project::new(request))
    }
    
    pub async fn analyze_project(&self, project_id: &str) -> Result<ProjectAnalysis, ProjectError> {
        // Analyze project structure and dependencies
        let project = self.get_project(project_id).await?;
        let structure = self.scan_project_structure(&project.path).await?;
        Ok(ProjectAnalysis { structure, project })
    }
}
```

### 2. Communication Hub

**HTTP API Server** (for Manager Web App):
```rust
use actix_web::{web, HttpResponse, Result};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

// All API types generated with ts-rs for TypeScript integration
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CreateProjectRequest {
    pub name: String,
    pub language: String,
    pub framework: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub status: String,
}

// Project management endpoints
async fn create_project(
    request: web::Json<CreateProjectRequest>
) -> Result<HttpResponse> {
    // Create new project with scaffolding
    Ok(HttpResponse::Ok().json(project))
}

async fn get_project_status(
    path: web::Path<String>
) -> Result<HttpResponse> {
    // Return current project status
    Ok(HttpResponse::Ok().json(project_status))
}

async fn execute_ai_request(
    request: web::Json<AiRequest>
) -> Result<HttpResponse> {
    // Coordinate with AI tools - request JSON response from LLM
    // using ts-rs generated TypeScript types
    Ok(HttpResponse::Ok().json(ai_response))
}
```

### 3. AI Session Management

**AI Tool Integration & Session Tracking**:
```rust
#[derive(Debug, Clone)]
pub struct AiSessionManager {
    sessions: HashMap<String, AiSession>,
    tools: HashMap<String, AiToolConfig>,
    project_context: ContextManager,
}

impl AiSessionManager {
    pub async fn create_session(&self, request: CreateAiSessionRequest) -> Result<AiSession, AiError> {
        // Create new AI session with project context
        let context = self.project_context.get_context(&request.work_id).await?;
        let session = AiSession::new(request, context);
        self.sessions.insert(session.id.clone(), session.clone());
        Ok(session)
    }
    
    pub async fn process_ai_request(&self, session_id: &str, message: &str) -> Result<AiResponse, AiError> {
        // Process AI request with context and guardrails
        let session = self.sessions.get(session_id).ok_or(AiError::SessionNotFound)?;
        let response = self.call_ai_tool(&session.tool_name, message, &session.context).await?;
        Ok(response)
    }
}
```

### 4. Process Management

**Long-running Process Orchestration**:
```rust
#[derive(Debug)]
pub struct ProcessManager {
    processes: HashMap<String, ManagedProcess>,
    supervisor: ProcessSupervisor,
}

#[derive(Debug)]
pub struct ManagedProcess {
    pub id: String,
    pub command: String,
    pub args: Vec<String>,
    pub working_dir: PathBuf,
    pub env: HashMap<String, String>,
    pub status: ProcessStatus,
    pub handle: Option<tokio::process::Child>,
}

impl ProcessManager {
    pub async fn start_ai_session(
        &mut self,
        tool: &str,
        project_path: &str,
    ) -> Result<String, ProcessError> {
        let process_id = format!("{}-{}", tool, Uuid::new_v4());
        let process = ManagedProcess {
            id: process_id.clone(),
            command: tool.to_string(),
            args: vec!["--interactive".to_string()],
            working_dir: PathBuf::from(project_path),
            env: self.get_ai_tool_env(),
            status: ProcessStatus::Starting,
            handle: None,
        };
        
        self.processes.insert(process_id.clone(), process);
        self.start_process(&process_id).await?;
        Ok(process_id)
    }
}
```

### 5. Project Management

**Project Lifecycle & Structure**:
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectManager {
    projects: HashMap<String, Project>,
    templates: TemplateRegistry,
    guardrails: GuardrailsEngine,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub language: String,
    pub framework: Option<String>,
    pub structure: ProjectStructure,
    pub metadata: ProjectMetadata,
    pub status: ProjectStatus,
}

impl ProjectManager {
    pub async fn analyze_project(&self, path: &str) -> Result<ProjectAnalysis, ProjectError> {
        let structure = self.scan_project_structure(path).await?;
        let language = self.detect_primary_language(&structure)?;
        let dependencies = self.analyze_dependencies(path, &language).await?;
        let quality_metrics = self.assess_code_quality(&structure).await?;
        
        Ok(ProjectAnalysis {
            structure,
            language,
            dependencies,
            quality_metrics,
            recommendations: self.generate_recommendations(&quality_metrics),
        })
    }
    
    pub async fn apply_guardrails(&self, code: &str, context: &ProjectContext) -> Result<GuardrailsResult, ProjectError> {
        self.guardrails.validate(code, context).await
    }
}
```

### 6. Security Model

**Current Security Approach**:
- **SSH-based access**: Users access the system via SSH to the Linux server
- **Localhost-only services**: Both manager (8081) and manager-web (3000) listen on localhost only
- **SSH port forwarding**: Users forward ports 3000 and 8081 over SSH for access
- **No public HTTP exposure**: Services are not accessible from the internet
- **Trusted environment**: Local-only MVP assumes trusted user with SSH access

**Future Security (Post-MVP)**:
- Authentication and authorization features are planned for cloud deployment phase
- JWT-based authentication, TLS certificates, and audit logging will be added when needed
- Current MVP focuses on functionality without complex security requirements

## Configuration Management

### Main Configuration File

```toml
# ~/.config/nocodo/manager.toml

[server]
host = "127.0.0.1"
port = 8081

[database]
path = "~/.local/share/nocodo/nocodo.db"

[socket]
path = "/tmp/nocodo-manager.sock"

[api_keys]
xai = ""          # XAI API key for Claude/Grok
openai = ""        # OpenAI API key for GPT models
anthropic = ""     # Anthropic API key for Claude models
```

## Database Schema

The database schema is defined in the code at `manager/src/database.rs`. Please refer to the source code for the current schema implementation, which includes:

- Projects table for project management
- AI sessions table linked to work sessions and messages
- Work messages table for conversation tracking
- Other supporting tables for the application state

The schema is managed through Rust migrations and may evolve over time.

## API Specification

### REST Endpoints

```rust
// Project Management
GET    /api/projects                    // List all projects
POST   /api/projects                    // Create new project
GET    /api/projects/{id}               // Get project details
PUT    /api/projects/{id}               // Update project
DELETE /api/projects/{id}               // Delete project

// Work Management
POST   /api/work                        // Create new work session
GET    /api/work                        // List all work sessions
GET    /api/work/{id}                   // Get work session details
DELETE /api/work/{id}                   // Delete work session
POST   /api/work/{id}/messages          // Add message to work session
GET    /api/work/{id}/messages          // Get messages for work session

// AI Sessions
POST   /api/ai/sessions                 // Create new AI session
GET    /api/ai/sessions                 // List AI sessions
GET    /api/ai/sessions/{id}            // Get AI session details

// File Operations
GET    /api/files                       // Browse file system
POST   /api/files                       // Create file/directory
PUT    /api/files/{path}                // Update file content
DELETE /api/files/{path}                // Delete file/directory

// Health Check
GET    /api/health                      // Service health check
GET    /api/version                     // Service version info
```

### WebSocket Endpoints

```rust
// Real-time updates
WS /ws/projects/{id}                    // Project updates
WS /ws/work/{id}                        // Work session communication
WS /ws/system                          // System status updates
WS /ws/logs                            // Real-time log streaming
```

## System Integration

### Development Setup

For local development, the Manager daemon and Web app are run manually:

```bash
# Start Manager daemon (API server on localhost:8081)
nocodo-manager --config ~/.config/nocodo/manager.toml

# Start Web app (dev server on localhost:3000)
cd manager-web && npm run dev
```

### Access Pattern

Users access the system through SSH port forwarding:

```bash
# Forward ports from local machine to remote server
ssh -L 3000:localhost:3000 -L 8081:localhost:8081 user@server

# Access web interface at http://localhost:3000
# Web app proxies API requests to http://localhost:8081
```

### Production Deployment (Future)

Production deployment scripts and configurations will be provided post-MVP when cloud deployment features are implemented.

## Monitoring & Observability

### Health Checks

```rust
#[derive(Debug, Serialize)]
pub struct HealthStatus {
    pub overall: ServiceStatus,
    pub components: HashMap<String, ComponentHealth>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ComponentHealth {
    pub status: ServiceStatus,
    pub message: String,
    pub last_check: DateTime<Utc>,
    pub metrics: Option<serde_json::Value>,
}

impl HealthMonitor {
    pub async fn check_system_health(&self) -> HealthStatus {
        let mut components = HashMap::new();
        
        components.insert("database".to_string(), self.check_database().await);
        components.insert("filesystem".to_string(), self.check_filesystem().await);
        components.insert("ai_tools".to_string(), self.check_ai_tools().await);
        components.insert("services".to_string(), self.check_services().await);
        
        let overall = self.calculate_overall_status(&components);
        
        HealthStatus {
            overall,
            components,
            timestamp: Utc::now(),
        }
    }
}
```

### Metrics Collection

```rust
#[derive(Debug)]
pub struct MetricsCollector {
    registry: prometheus::Registry,
    counters: HashMap<String, prometheus::Counter>,
    gauges: HashMap<String, prometheus::Gauge>,
    histograms: HashMap<String, prometheus::Histogram>,
}

impl MetricsCollector {
    pub fn record_request(&self, endpoint: &str, method: &str, status_code: u16) {
        let counter = self.counters.get("http_requests_total").unwrap();
        counter.with_label_values(&[endpoint, method, &status_code.to_string()]).inc();
    }
    
    pub fn record_ai_session_duration(&self, tool: &str, duration: f64) {
        let histogram = self.histograms.get("ai_session_duration_seconds").unwrap();
        histogram.with_label_values(&[tool]).observe(duration);
    }
}
```

## Error Handling & Recovery

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum ManagerError {
    #[error("System error: {0}")]
    System(#[from] SystemError),
    
    #[error("Process error: {0}")]
    Process(#[from] ProcessError),
    
    #[error("Project error: {0}")]
    Project(#[from] ProjectError),
    
    #[error("Security error: {0}")]
    Security(#[from] SecurityError),
    
    #[error("Communication error: {0}")]
    Communication(String),
    
    #[error("Configuration error: {0}")]
    Configuration(String),
}

impl From<ManagerError> for AppError {
    fn from(err: ManagerError) -> Self {
        AppError {
            code: err.error_code(),
            message: err.to_string(),
            details: Some(err.error_details()),
        }
    }
}
```

### Recovery Strategies

```rust
#[derive(Debug)]
pub struct RecoveryManager {
    strategies: HashMap<ErrorType, RecoveryStrategy>,
    retry_config: RetryConfig,
}

impl RecoveryManager {
    pub async fn handle_error(&self, error: &ManagerError) -> RecoveryResult {
        let strategy = self.strategies.get(&error.error_type())
            .unwrap_or(&RecoveryStrategy::LogAndContinue);
            
        match strategy {
            RecoveryStrategy::Restart => self.restart_service().await,
            RecoveryStrategy::Retry => self.retry_operation().await,
            RecoveryStrategy::Failover => self.failover_to_backup().await,
            RecoveryStrategy::LogAndContinue => self.log_and_continue().await,
        }
    }
}
```

## Security Considerations

1. **Process Isolation**: All AI tools run in isolated processes with limited permissions
2. **File System Security**: Restricted file system access with proper permissions
3. **Network Security**: TLS for all external communication, local-only socket access
4. **Authentication**: JWT-based authentication with the Bootstrap app
5. **Audit Logging**: Comprehensive audit trail for all operations
6. **Input Validation**: Strict input validation for all API endpoints
7. **Resource Limits**: Process and resource limits to prevent abuse

## Installation & Deployment

### Installation Script

```bash
#!/bin/bash
# install-manager.sh

set -e

# Create nocodo user
sudo useradd -r -s /bin/false -d /var/lib/nocodo nocodo

# Create directories
sudo mkdir -p /etc/nocodo /var/lib/nocodo /var/log/nocodo /usr/local/bin

# Install binary
sudo cp nocodo-manager /usr/local/bin/
sudo chmod +x /usr/local/bin/nocodo-manager

# Install configuration
sudo cp manager.toml /etc/nocodo/
sudo chown -R nocodo:nocodo /var/lib/nocodo /var/log/nocodo

# Install systemd service
sudo cp nocodo-manager.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable nocodo-manager

# Setup nginx configuration
sudo cp nginx-manager.conf /etc/nginx/sites-available/nocodo-manager
sudo ln -sf /etc/nginx/sites-available/nocodo-manager /etc/nginx/sites-enabled/

echo "Nocodo Manager installed successfully!"
```

## Clarification Questions

1. **Resource Management**: What are the memory and CPU limits for AI tool processes?
2. **Concurrency**: How many simultaneous AI sessions should be supported?
3. **Project Isolation**: Should different projects run in completely isolated environments?
4. **Backup Strategy**: How should project data and configurations be backed up?
5. **Update Strategy**: How should the Manager daemon handle self-updates?
6. **Multi-user Support**: Should the Manager support multiple concurrent users?
7. **Container Strategy**: Should projects run in Docker containers for isolation?
8. **Logging Retention**: How long should logs be retained and what rotation policy?

## Future Enhancements

- Bootstrap app for cloud deployment
- Authentication and security features for cloud deployment
- Docker container support for project isolation
- Kubernetes integration for scalable deployments
- Advanced project templates and scaffolding
- Integration with external monitoring tools (Prometheus, Grafana)
- Multi-tenant support with user isolation
- Advanced caching and performance optimization
- Integration with CI/CD pipelines
- Backup and disaster recovery automation
- Advanced security scanning and vulnerability detection
