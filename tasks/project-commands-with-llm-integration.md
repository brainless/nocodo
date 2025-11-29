# Project Commands with LLM Agent Integration

## Overview

This document outlines the design and implementation plan for adding **Project Commands** to the nocodo manager. Project Commands are general-purpose terminal operations for development tasks (e.g., `npm install`, `django runserver`, `cargo build`, `npm test`). The system will integrate with the existing LLM agent to enable intelligent command discovery.

### Distinction from Workflow Commands

**Project Commands** (this feature) vs **Workflow Commands** (existing):

| Aspect | Project Commands (NEW) | Workflow Commands (EXISTING) |
|--------|------------------------|------------------------------|
| **Purpose** | General development task commands | CI/CD workflow debugging |
| **Scope** | All project commands (install, build, run, test, etc.) | GitHub Actions workflow steps only |
| **Sources** | LLM discovery, package.json, manual, config files, AND workflows | `.github/workflows/*.yml` files |
| **Use Case** | "What commands can I run on this project?" | "Run GitHub Actions step 3 locally" |
| **User Intent** | Day-to-day development tasks | Debug failing CI/CD pipelines |

**Relationship**: The existing Workflow Command parser will provide commands for Project Commands. Both systems are complementary and will coexist.

## Current State Analysis

### Existing Infrastructure

1. **Workflow Commands** (`nocodo-github-actions/src/models.rs:84-94`)
   - **Purpose**: Parse GitHub Actions YAML files to extract CI/CD commands for local execution
   - **Use case**: Debug CI/CD workflows by running specific steps locally
   - **Scope**: Limited to `.github/workflows/*.yml` files
   - **Implementation**: `WorkflowCommand` struct with execution tracking
   - **Relationship to Project Commands**: This provides commands for CI/CD workflows. Project Commands will be more general-purpose.

2. **LLM Agent** (`manager/src/llm_agent.rs:1311-1433`)
   - 6 existing tools: `list_files`, `read_file`, `write_file`, `grep`, `apply_patch`, `bash`
   - Multi-provider support: Anthropic, OpenAI, XAI, Zhipu AI
   - Session management with conversation history
   - Native tool calling support

3. **Tool System** (`manager/src/tools.rs`)
   - Typed `ToolRequest` enum (`models.rs:281`)
   - Centralized `ToolExecutor` (`tools.rs:84`)
   - Permission checking and path validation

## Command Data Structure

### Core Command Model

```rust
/// Project command that can be executed for development tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectCommand {
    pub id: String,                          // UUID
    pub project_id: i64,
    pub name: String,                        // e.g., "dev", "build", "test"
    pub description: Option<String>,         // Human-readable description
    pub command: String,                     // The actual command to run

    // Execution context
    pub shell: Option<String>,               // "bash", "sh", "powershell", "cmd", etc.
    pub working_directory: Option<String>,   // Relative to project root (main or worktree)
    pub timeout_seconds: Option<u64>,        // Default timeout (120s default)

    // Environment
    pub environment: Option<HashMap<String, String>>, // Environment variables
    pub os_filter: Option<Vec<String>>,      // ["linux", "darwin", "windows"]

    // Metadata
    pub created_at: i64,
    pub updated_at: i64,
}

/// Request to execute a command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteCommandRequest {
    pub git_branch: Option<String>,  // null = main branch, Some(branch) = worktree
    pub environment: Option<HashMap<String, String>>, // Override environment
    pub timeout_seconds: Option<u64>, // Override timeout
}

/// Execution result for a command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandExecution {
    pub id: i64,
    pub command_id: String,
    pub git_branch: Option<String>,  // Which branch this was executed on
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    pub executed_at: i64,
    pub success: bool,
}
```

### Real-World Use Cases Covered

1. **Package Managers**: npm, yarn, pnpm, cargo, pip, poetry, go modules
2. **Development Servers**: Django, Rails, Next.js, Vite, Express
3. **Build Tools**: webpack, vite, cargo, go build, maven, gradle
4. **Testing**: pytest, jest, vitest, cargo test, go test
5. **Database Operations**: migrations, seeds, dumps
6. **Docker**: docker-compose up/down, docker build
7. **Deployment**: CI/CD commands, cloud deployments
8. **OS-Specific Commands**: Different commands for Linux/macOS/Windows

## LLM Agent Integration Design

### Automated Discovery Flow with LLM Conversation

Create an API endpoint that triggers a full LLM agent session to discover commands.

#### API Endpoint

**Location**: `manager/src/handlers.rs`

```rust
pub async fn discover_project_commands(
    project_id: web::Path<i64>,
    db: web::Data<Arc<Database>>,
    llm_agent: web::Data<Arc<LlmAgent>>,
) -> impl Responder {
    // 1. Create AI session for command discovery
    // 2. Send initial prompt to LLM agent
    // 3. LLM uses tools (list_files, read_file, bash) to explore
    // 4. LLM returns structured command suggestions
    // 5. Validate and store commands
    // 6. Return discovered commands to user
}
```

#### Discovery Prompt Template

```text
You are analyzing a software project to discover common development commands.

Project ID: {project_id}
Project Path: {project_path}

Your task:
1. Examine the project structure and configuration files
2. Identify the technology stack (language, framework, build tools)
3. Discover and suggest commands for:
   - Installing dependencies
   - Building the project
   - Running development server
   - Running tests
   - Linting/formatting
   - Database operations (if applicable)
   - Deployment (if applicable)

Use the following tools:
- list_files: To explore project structure
- read_file: To read configuration files (package.json, Cargo.toml, etc.)
- bash: To check tool versions or run --help commands if needed

Return your findings as a JSON array with this format:
[
  {
    "name": "install",
    "command": "npm install",
    "description": "Install project dependencies",
    "working_directory": null,
    "environment": null,
    "shell": "bash"
  },
  ...
]

Guidelines:
- Include environment variables when relevant (NODE_ENV, DEBUG, etc.)
- Specify working_directory if command must run in subdirectory (relative to project main or worktree root)
```

## Implementation Plan

### Phase 1: Database Schema

**Location**: `manager/src/database.rs` (schema initialization)

```sql
-- Project commands table
CREATE TABLE IF NOT EXISTS project_commands (
    id TEXT PRIMARY KEY,
    project_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    command TEXT NOT NULL,
    shell TEXT,
    working_directory TEXT,
    environment TEXT, -- JSON object
    timeout_seconds INTEGER DEFAULT 120,
    os_filter TEXT, -- JSON array
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

-- Command execution history
CREATE TABLE IF NOT EXISTS command_executions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    command_id TEXT NOT NULL,
    git_branch TEXT,  -- Which branch this was executed on
    exit_code INTEGER,
    stdout TEXT,
    stderr TEXT,
    duration_ms INTEGER NOT NULL,
    executed_at INTEGER NOT NULL,
    success INTEGER NOT NULL,
    FOREIGN KEY (command_id) REFERENCES project_commands(id) ON DELETE CASCADE
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_commands_project ON project_commands(project_id);
CREATE INDEX IF NOT EXISTS idx_executions_command ON command_executions(command_id);
CREATE INDEX IF NOT EXISTS idx_executions_time ON command_executions(executed_at);
```

**Database Functions**:

```rust
// In database.rs
pub fn create_project_command(&self, command: &ProjectCommand) -> Result<()>;
pub fn get_project_commands(&self, project_id: i64) -> Result<Vec<ProjectCommand>>;
pub fn get_project_command_by_id(&self, id: &str) -> Result<ProjectCommand>;
pub fn update_project_command(&self, command: &ProjectCommand) -> Result<()>;
pub fn delete_project_command(&self, id: &str) -> Result<()>;
pub fn create_command_execution(&self, execution: &CommandExecution) -> Result<i64>;
pub fn get_command_executions(&self, command_id: &str, limit: i64) -> Result<Vec<CommandExecution>>;
```

**Location**: `manager/src/command_discovery.rs` (new file)

```rust
/// Intelligent command discovery engine
pub struct CommandDiscovery {
    project_path: PathBuf,
}

impl CommandDiscovery {
    pub fn new(project_path: PathBuf) -> Self {
        Self { project_path }
    }

    /// Detect project type and tech stack
    pub async fn detect_project_type(&self) -> Result<ProjectType>;

    /// Discover commands from package.json scripts
    pub async fn discover_npm_commands(&self) -> Result<Vec<SuggestedCommand>>;

    /// Discover Rust/Cargo commands
    pub async fn discover_cargo_commands(&self) -> Result<Vec<SuggestedCommand>>;

    /// Discover Python commands
    pub async fn discover_python_commands(&self) -> Result<Vec<SuggestedCommand>>;

    /// Parse Makefile targets
    pub async fn discover_make_commands(&self) -> Result<Vec<SuggestedCommand>>;

    /// Extract from GitHub Actions workflows (reuses existing workflow parser)
    /// Note: This leverages the existing nocodo-github-actions crate
    pub async fn discover_workflow_commands(&self) -> Result<Vec<SuggestedCommand>>;

    /// Detect framework-specific commands
    pub async fn detect_framework_commands(&self) -> Result<Vec<SuggestedCommand>>;

    /// Main discovery method
    pub async fn discover_all(&self) -> Result<DiscoverCommandsResponse>;
}

pub enum ProjectType {
    NodeJs { manager: PackageManager },
    Rust,
    Python { tool: PythonTool },
    Go,
    Java { build_tool: JavaBuildTool },
    Mixed(Vec<ProjectType>),
    Unknown,
}

pub enum PackageManager {
    Npm,
    Yarn,
    Pnpm,
    Bun,
}

pub enum PythonTool {
    Pip,
    Poetry,
    Pipenv,
    Conda,
}

pub enum JavaBuildTool {
    Maven,
    Gradle,
}
```

**Key Detection Strategies**:

1. **Package Managers**:
   - `package.json` → npm/yarn/pnpm (check lock files)
   - `Cargo.toml` → cargo
   - `pyproject.toml` → poetry
   - `requirements.txt` → pip
   - `go.mod` → go modules
   - `pom.xml` → maven
   - `build.gradle` → gradle

2. **Frameworks**:
   - Django: Check for `manage.py` → `python manage.py runserver`
   - Rails: Check for `Rakefile` + `bin/rails` → `rails server`
   - Next.js: Check `package.json` dependencies → `npm run dev`
   - Vite: Check for `vite.config.*` → `npm run dev`
   - FastAPI: Check imports → `uvicorn main:app --reload`

3. **Build Tools**:
   - `Makefile` → Parse targets with `make -qp`
   - `Justfile` → Parse recipes
   - `.github/workflows/*.yml` → Extract run commands

4. **Environment Detection**:
   - `.env.example` → Parse for environment variable templates
   - Common patterns: `NODE_ENV`, `DEBUG`, `DATABASE_URL`, etc.

### Phase 4: Security & Permissions

**Location**: `manager/src/main.rs` (route configuration)

```rust
// Command management routes
.route(
    "/api/projects/{id}/commands",
    web::get().to(handlers::get_project_commands)
)
.route(
    "/api/projects/{id}/commands",
    web::post().to(handlers::create_project_command)
)
.route(
    "/api/projects/{id}/commands/discover",
    web::post().to(handlers::discover_project_commands)
)
.route(
    "/api/projects/{id}/commands/{cmd_id}",
    web::get().to(handlers::get_project_command)
)
.route(
    "/api/projects/{id}/commands/{cmd_id}",
    web::put().to(handlers::update_project_command)
)
.route(
    "/api/projects/{id}/commands/{cmd_id}",
    web::delete().to(handlers::delete_project_command)
)
.route(
    "/api/projects/{id}/commands/{cmd_id}/execute",
    web::post().to(handlers::execute_project_command)
)
.route(
    "/api/projects/{id}/commands/{cmd_id}/execute",
    web::post().to(handlers::execute_project_command)
)
.route(
    "/api/projects/{id}/commands/{cmd_id}/executions",
    web::get().to(handlers::get_command_executions)
)
```

**Handler Implementations** (`handlers.rs`):

```rust
/// GET /api/projects/{id}/commands
pub async fn get_project_commands(
    project_id: web::Path<i64>,
    query: web::Query<CommandFilterQuery>,
    db: web::Data<Arc<Database>>,
) -> impl Responder {
     // Filter commands as needed.
}

/// POST /api/projects/{id}/commands
pub async fn create_project_command(
    project_id: web::Path<i64>,
    command: web::Json<CreateCommandRequest>,
    db: web::Data<Arc<Database>>,
) -> impl Responder {
    // Validate and create command
}

/// POST /api/projects/{id}/commands/discover
pub async fn discover_project_commands(
    project_id: web::Path<i64>,
    db: web::Data<Arc<Database>>,
    llm_agent: web::Data<Arc<LlmAgent>>,
) -> impl Responder {
    // Trigger LLM-based discovery
}

/// POST /api/projects/{id}/commands/{cmd_id}/execute
pub async fn execute_project_command(
    path: web::Path<(i64, String)>,
    request: web::Json<ExecuteCommandRequest>,
    db: web::Data<Arc<Database>>,
) -> impl Responder {
    // 1. Get command from database (project-scoped only)
    // 2. Determine execution path from request.git_branch:
    //    - If git_branch is None: project.main_path + working_directory
    //    - If git_branch is Some(branch): git::get_worktree_path(project_path, branch) + working_directory
    // 3. Execute command and track result
}
```

### Phase 5: Polish & Testing

**Integration with Existing Systems**:

1. **Bash Permissions** (`bash_permissions.rs`):
   - Reuse existing permission checks
   - Add command-specific rules
   - Support allow/deny patterns per project

2. **Permission Middleware** (`middleware.rs`):
   - Require "project:write" for command execution
   - Require "project:read" for listing commands

3. **Approval Flow**:
   - Commands marked as "pending approval"
   - User must approve before execution
   - Add `approved` boolean to `project_commands` table

4. **Audit Trail**:
   - All executions logged in `command_executions`
   - Track stdout, stderr, exit code, duration
   - Retain execution history

## Example Usage Scenarios

### Scenario 1: Automatic Discovery on Project Add

```javascript
// User adds a project
POST /api/projects
{
  "name": "My Next.js App",
  "path": "/home/user/projects/nextjs-app"
}

// System automatically triggers discovery
POST /api/projects/1/commands/discover

// LLM agent analyzes:
// 1. Finds package.json with Next.js dependencies
// 2. Reads scripts section
// 3. Detects Vercel/Next.js patterns

// Returns:
{
  "commands": [
    {
      "name": "install",
      "command": "npm install",
      "description": "Install project dependencies"
    },
    {
      "name": "dev",
      "command": "npm run dev",
      "description": "Start development server",
      "environment": { "NODE_ENV": "development" }
    },
    {
      "name": "build",
      "command": "npm run build",
      "description": "Build for production",
      "environment": { "NODE_ENV": "production" }
    },
    {
      "name": "start",
      "command": "npm start",
      "description": "Start production server"
    }
  ],
  "reasoning": "Detected Next.js project with package.json scripts. Commands extracted from scripts section."
}
```

### Scenario 2: LLM Agent Uses Commands in Conversation

```javascript
// User: "Can you start the development server?"

// LLM agent:
 // 1. Searches project commands for running applications
// 2. Finds "dev" command
// 3. Executes: bash({ command: "npm run dev", working_directory: "." })
// 4. Returns: "Development server started on http://localhost:3000"
```

### Scenario 3: Same Command in Multiple Contexts

```javascript
// Single command definition for the project
{
  "name": "dev",
  "command": "npm run dev", 
  "description": "Start development server",
  "working_directory": null
}

// Execute in main branch
POST /api/projects/1/commands/cmd-123/execute
{
  "git_branch": null
}
// Runs at: /project/main/ + npm run dev

// Execute in feature-branch worktree  
POST /api/projects/1/commands/cmd-123/execute
{
  "git_branch": "feature-branch"
}
// Runs at: /project/worktrees/feature-branch/ + npm run dev

// Execute in another worktree
POST /api/projects/1/commands/cmd-123/execute
{
  "git_branch": "another-feature"
}
// Runs at: /project/worktrees/another-feature/ + npm run dev
```

### Scenario 4: Multi-Component Project

```javascript
// Monorepo with backend (Django) and frontend (React)

// Discovery finds:
{
  "commands": [
    // Backend commands
    {
      "name": "backend-install",
      "command": "pip install -r requirements.txt",
      "description": "Install backend dependencies",
      "working_directory": "backend"
    },
    {
      "name": "backend-migrate",
      "command": "python manage.py migrate",
      "description": "Run database migrations",
      "working_directory": "backend"
    },
    {
      "name": "backend-runserver",
      "command": "python manage.py runserver",
      "description": "Start Django development server",
      "working_directory": "backend"
    },

    // Frontend commands
    {
      "name": "frontend-install",
      "command": "npm install",
      "description": "Install frontend dependencies",
      "working_directory": "frontend"
    },
    {
      "name": "frontend-dev",
      "command": "npm run dev",
      "description": "Start frontend development server",
      "working_directory": "frontend"
    }
  ]
}
```

## Integration with Existing Features

### 1. Workflow Commands (Complementary, Not Replacement)

**Important**: Project Commands and Workflow Commands serve different but complementary purposes:

- **Workflow Commands** (existing):
  - Focused on CI/CD debugging
  - Parse GitHub Actions YAML for local execution
  - Specific use case: "Run step 3 of the build workflow locally"

- **Project Commands** (new):
  - General development task commands
   - Commands discovered through LLM analysis, package.json scripts, etc.
  - Broader use case: "What can I run on this project?"

**Integration Strategy**:

The existing Workflow Command parser can provide commands for Project Commands:

```rust
/// Import workflow commands as project commands
pub fn import_workflow_commands_as_project_commands(
    db: &Database,
    project_id: i64
) -> Result<Vec<ProjectCommand>> {
    // 1. Use existing workflow scanner to parse .github/workflows/*.yml
    // 2. Convert WorkflowCommand -> ProjectCommand
    // 3. Store in project_commands table

    // This makes workflow commands discoverable through the unified command interface
    // while preserving the specialized workflow debugging functionality
}
```

**Both systems can coexist**:
- Keep Workflow Commands for detailed CI/CD workflow debugging
- Project Commands provide a unified interface for ALL project commands
- Users can discover workflow commands through Project Commands UI

### 2. Git Worktrees

Commands are project-scoped but can be executed in both main branch and any worktree:

**Execution Context:**
- **Single API**: `/api/projects/{id}/commands/{cmd_id}/execute`
- **Branch specified in request body**: `git_branch: Option<String>`

**Path Resolution Logic:**
- **Main Branch** (`git_branch: None`): `project.main_path + working_directory`
- **Worktree** (`git_branch: Some("feature-branch")`): `git::get_worktree_path(project_path, "feature-branch") + working_directory`

**Use Cases:**
```rust
// Same command definition
ProjectCommand {
    name: "dev",
    command: "npm run dev",
    working_directory: Some("frontend"),
}

// Executed in main branch
POST /api/projects/1/commands/cmd-123/execute
{
  "git_branch": null
}
// Path: /project/main/frontend/ + npm run dev

// Executed in worktree
POST /api/projects/1/commands/cmd-123/execute  
{
  "git_branch": "feature-branch"
}
// Path: /project/worktrees/feature-branch/frontend/ + npm run dev
```

### 3. Work Sessions

Link commands to work items:

```rust
// User: "Run the tests for this work item"
// System: Finds test commands, executes in work session context
```

## Configuration File Support (Future Enhancement)

Support `nocodo.yml` in project root:

```yaml
version: 1
commands:
  install:
    description: "Install all dependencies"
    command: "npm install"
    timeout: 300

  dev:
    description: "Start dev server"
    command: "npm run dev"
    environment:
      NODE_ENV: development
      DEBUG: "true"
    background: true

  test:
    description: "Run test suite"
    command: "npm test"
    timeout: 600

  build:
    description: "Build for production"
    command: "npm run build"
    environment:
      NODE_ENV: production
    os: [linux, darwin]  # Not available on Windows
```

## Testing Strategy

### Unit Tests

1. **Command Discovery**:
   - Test detection for each project type
   - Test parsing of various config files

2. **Tool Execution**:
   - Test `discover_commands` tool request/response
   - Test error handling

### Integration Tests

1. **LLM Agent**:
   - Test discovery with mock LLM responses
   - Test tool calling flow
   - Test command storage

2. **API Endpoints**:
   - Test CRUD operations
   - Test execution tracking
   - Test permission checks

### End-to-End Tests

1. **Sample Projects**:
   - Create fixture projects for each tech stack
   - Test automatic discovery
   - Verify command accuracy

## Performance Considerations

1. **Caching**:
   - Cache discovered commands per project
   - Invalidate on file changes (watch package.json, etc.)

2. **Parallel Discovery**:
   - Analyze multiple config files concurrently
   - Use async/await throughout

3. **Rate Limiting**:
   - Limit LLM API calls for discovery
   - Option to use local analysis vs. LLM

## Migration Path

### Phase 1: Core Infrastructure (Week 1-2) ✅ COMPLETE
- [x] Database schema
- [x] Models and types
- [x] Basic CRUD operations

### Phase 2: Discovery Engine (Week 2-3) ✅ COMPLETE
- [x] CommandDiscovery implementation
- [x] Tech stack detection (Node.js, Rust, Python, Go, Java)
- [x] Config file parsing (package.json, Cargo.toml, pyproject.toml, go.mod, pom.xml, Makefile)
- [x] Discovery API endpoint implementation (basic version without LLM)
- [x] Command management API endpoints (CRUD operations)
- [x] Command execution API endpoint

### Phase 3: LLM Integration (Week 3-4) ✅ COMPLETE
- [x] Enhanced LLM session management for discovery
- [x] Prompt engineering and testing with LLM
- [x] Integration with existing LlmAgent for intelligent discovery
- [x] Hybrid discovery strategy (rule-based + LLM)
- [x] Query parameter support for enabling/disabling LLM (`use_llm=true|false`)

### Phase 4: API & UI (Week 4-5)
- [ ] WebSocket updates for execution status
- [ ] Desktop app UI integration
- [ ] Real-time command output streaming

### Phase 5: Polish & Testing (Week 5-6)
- [ ] Security audit
- [ ] Performance optimization
- [ ] Documentation
- [ ] End-to-end tests

## Open Questions

1. **Command Versioning**: Should we track command changes over time?
2. **Sharing**: Can users share command configurations across projects?
3. **Templates**: Should we have pre-built command templates for common stacks?
4. **Monitoring**: Real-time output streaming for long-running commands?
5. **Pipelines**: Support chaining commands (install → migrate → runserver)?

## References

- Existing workflow command implementation: `nocodo-github-actions/src/`
- LLM agent: `manager/src/llm_agent.rs`
- Tool system: `manager/src/tools.rs`
- Database: `manager/src/database.rs`
- API handlers: `manager/src/handlers.rs`
