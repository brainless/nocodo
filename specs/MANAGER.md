# Manager App Specification

## Overview

The Manager app is a Linux daemon that runs on the Operator server, orchestrating the entire software development lifecycle from idea to deployment. It serves as the central coordinator between the nocodo CLI, the Manager Web app, and various development tools. Written in Rust, it provides secure communication channels, system management, and development environment orchestration.

## Architecture

### Core Components

1. **System Orchestrator** - Manages server state, services, and system configuration
2. **Communication Hub** - Handles Unix socket and HTTP communication
3. **Development Environment Manager** - Installs and manages development tools
4. **Process Manager** - Manages long-running processes and services
5. **Project Manager** - Handles project lifecycle and file system operations
6. **Security Manager** - Manages authentication, authorization, and security policies
7. **Logging & Monitoring** - Comprehensive logging and system monitoring

### Technology Stack

- **Language**: Rust
- **Async Runtime**: Tokio
- **Web Framework**: Actix Web (for HTTP API)
- **Unix Sockets**: Unix domain sockets for CLI communication
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
│   Unix Socket   │   HTTP Server   │   System Services  │
│   Server        │   (Web API)     │   Manager          │
├─────────────────┼─────────────────┼────────────────────┤
│                 │                 │                    │
│   nocodo CLI    │  Manager Web    │   Development      │
│   ←→ Socket     │  ←→ HTTP/WS     │   Tools & Processes│
│                 │                 │                    │
└─────────────────┴─────────────────┴────────────────────┘
```

## Core Features

### 1. System Orchestration

**Responsibilities**:
- Server initialization and configuration
- System service management (nginx, PostgreSQL, etc.)
- Security hardening and maintenance
- System monitoring and health checks
- Automatic updates and maintenance tasks

```rust
#[derive(Debug, Clone)]
pub struct SystemOrchestrator {
    services: HashMap<String, ServiceConfig>,
    health_monitor: HealthMonitor,
    update_scheduler: UpdateScheduler,
}

impl SystemOrchestrator {
    pub async fn initialize_system(&self) -> Result<(), SystemError> {
        self.setup_development_environment().await?;
        self.configure_security().await?;
        self.start_core_services().await?;
        self.setup_monitoring().await?;
        Ok(())
    }
    
    pub async fn install_development_tools(&self) -> Result<(), SystemError> {
        // Install Git, Python, Node.js, Rust, Docker, etc.
    }
}
```

### 2. Communication Hub

**Unix Socket Server** (for nocodo CLI):
```rust
#[derive(Debug, Serialize, Deserialize)]
pub enum CliMessage {
    ProjectAnalysis { path: String },
    GeneratePrompt { request: PromptRequest },
    ValidateCode { code: String, language: String },
    GetProjectStructure { path: String },
    ExecuteCommand { command: String, args: Vec<String> },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum CliResponse {
    Analysis(ProjectAnalysis),
    Prompt(GeneratedPrompt),
    Validation(ValidationResult),
    Structure(ProjectStructure),
    CommandResult(CommandOutput),
    Error(String),
}
```

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

### 3. Development Environment Management

**Tool Installation & Management**:
```rust
#[derive(Debug, Clone)]
pub struct DevelopmentEnvironment {
    installed_tools: HashMap<String, ToolVersion>,
    package_managers: Vec<PackageManager>,
    containers: DockerManager,
}

impl DevelopmentEnvironment {
    pub async fn install_base_tools(&mut self) -> Result<(), EnvError> {
        self.install_git().await?;
        self.install_python().await?;
        self.install_nodejs().await?;
        self.install_rust().await?;
        self.install_docker().await?;
        self.install_ai_tools().await?; // Claude Code, Gemini CLI, etc.
        Ok(())
    }
    
    pub async fn install_ai_tools(&mut self) -> Result<(), EnvError> {
        // Install Claude Code, Gemini CLI, OpenAI CLI, etc.
        self.install_tool("claude-code", "latest").await?;
        self.install_tool("gemini-cli", "latest").await?;
        Ok(())
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

### 6. Security Management

**Authentication & Authorization**:
```rust
#[derive(Debug)]
pub struct SecurityManager {
    auth_tokens: HashMap<String, AuthToken>,
    permissions: PermissionMatrix,
    audit_log: AuditLogger,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthToken {
    pub token: String,
    pub user_id: String,
    pub expires_at: DateTime<Utc>,
    pub permissions: Vec<Permission>,
}

impl SecurityManager {
    pub fn validate_token(&self, token: &str) -> Result<AuthToken, SecurityError> {
        // Validate JWT token from Bootstrap app
    }
    
    pub fn check_permission(&self, token: &str, action: &str) -> Result<bool, SecurityError> {
        // Check if token has permission for action
    }
    
    pub async fn audit_action(&self, user_id: &str, action: &str, details: &str) {
        self.audit_log.log_action(user_id, action, details).await;
    }
}
```

## Configuration Management

### Main Configuration File

```toml
# /etc/nocodo/manager.toml

[server]
unix_socket_path = "/var/run/nocodo/manager.sock"
http_port = 8081
https_port = 8443
max_connections = 1000

[security]
jwt_secret_file = "/etc/nocodo/jwt.secret"
tls_cert_file = "/etc/nocodo/ssl/cert.pem"
tls_key_file = "/etc/nocodo/ssl/key.pem"
audit_log_file = "/var/log/nocodo/audit.log"

[development]
workspace_root = "/home/developer/projects"
ai_tools_path = "/usr/local/bin"
default_shell = "/bin/bash"

[services]
nginx_config_path = "/etc/nginx/sites-available"
postgresql_data_path = "/var/lib/postgresql/data"
redis_config_path = "/etc/redis/redis.conf"

[monitoring]
metrics_port = 9090
health_check_interval = 30
log_level = "info"
log_file = "/var/log/nocodo/manager.log"

[ai_tools]
claude_code_path = "/usr/local/bin/claude"
gemini_cli_path = "/usr/local/bin/gemini"
timeout_seconds = 300
max_concurrent_sessions = 5
```

## Database Schema

```sql
-- Projects table
CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    path TEXT NOT NULL,
    language TEXT,
    framework TEXT,
    status TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

-- AI sessions table
CREATE TABLE ai_sessions (
    id TEXT PRIMARY KEY,
    project_id TEXT,
    tool_name TEXT NOT NULL,
    status TEXT NOT NULL,
    started_at INTEGER NOT NULL,
    ended_at INTEGER,
    FOREIGN KEY (project_id) REFERENCES projects (id)
);

-- Process registry
CREATE TABLE managed_processes (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    command TEXT NOT NULL,
    working_dir TEXT,
    status TEXT NOT NULL,
    pid INTEGER,
    started_at INTEGER NOT NULL,
    stopped_at INTEGER
);

-- Audit log
CREATE TABLE audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT,
    action TEXT NOT NULL,
    resource TEXT,
    details TEXT,
    timestamp INTEGER NOT NULL
);
```

## API Specification

### REST Endpoints

```rust
// Project Management
GET    /api/projects                    // List all projects
POST   /api/projects                    // Create new project
GET    /api/projects/{id}               // Get project details
PUT    /api/projects/{id}               // Update project
DELETE /api/projects/{id}               // Delete project
GET    /api/projects/{id}/status        // Get project status
POST   /api/projects/{id}/analyze       // Analyze project structure

// AI Integration
POST   /api/ai/sessions                 // Start AI session
GET    /api/ai/sessions/{id}            // Get session status
POST   /api/ai/sessions/{id}/query      // Send query to AI
DELETE /api/ai/sessions/{id}            // End AI session

// System Management
GET    /api/system/status               // Get system status
GET    /api/system/services             // List system services
POST   /api/system/services/{name}/start // Start service
POST   /api/system/services/{name}/stop  // Stop service

// File System
GET    /api/files                       // Browse file system
POST   /api/files                       // Create file/directory
PUT    /api/files/{path}                // Update file content
DELETE /api/files/{path}                // Delete file/directory
```

### WebSocket Endpoints

```rust
// Real-time updates
WS /ws/projects/{id}                    // Project updates
WS /ws/ai-sessions/{id}                 // AI session communication
WS /ws/system                          // System status updates
WS /ws/logs                            // Real-time log streaming
```

## System Integration

### Systemd Service

```ini
# /etc/systemd/system/nocodo-manager.service
[Unit]
Description=Nocodo Manager Daemon
After=network.target

[Service]
Type=notify
User=nocodo
Group=nocodo
ExecStart=/usr/local/bin/nocodo-manager --config /etc/nocodo/manager.toml
Restart=on-failure
RestartSec=5

# Security
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/nocodo /var/log/nocodo /tmp
PrivateTmp=true

[Install]
WantedBy=multi-user.target
```

### Nginx Configuration

```nginx
# /etc/nginx/sites-available/nocodo-manager
server {
    listen 80;
    server_name manager.local;
    
    # Redirect to HTTPS
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name manager.local;
    
    ssl_certificate /etc/nocodo/ssl/cert.pem;
    ssl_certificate_key /etc/nocodo/ssl/key.pem;
    
    # Static files (Manager Web App)
    location / {
        root /var/www/nocodo-manager;
        index index.html;
        try_files $uri $uri/ /index.html;
    }
    
    # API proxy
    location /api/ {
        proxy_pass http://localhost:8081;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
    
    # WebSocket proxy
    location /ws/ {
        proxy_pass http://localhost:8081;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
    }
}
```

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

- Docker container support for project isolation
- Kubernetes integration for scalable deployments
- Advanced project templates and scaffolding
- Integration with external monitoring tools (Prometheus, Grafana)
- Multi-tenant support with user isolation
- Advanced caching and performance optimization
- Integration with CI/CD pipelines
- Backup and disaster recovery automation
- Advanced security scanning and vulnerability detection
