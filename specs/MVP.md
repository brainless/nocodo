# nocodo MVP Specification for Linux Laptop

## Overview

This MVP specification defines the minimal viable product for nocodo running entirely on a Linux laptop, focusing on the three core components: Manager daemon, Manager Web app, and nocodo CLI. This version eliminates cloud dependencies and provides a fully self-contained development environment for AI-assisted coding.

## MVP Scope

### What's Included

1. **Manager Daemon** - Core orchestration service running locally
2. **Manager Web App** - Chat-based interface for AI interaction
3. **nocodo CLI** - Command-line companion for AI coding tools
4. **Local Development Environment** - Complete setup on your CachyOS Linux laptop


## System Architecture

```
┌─────────────────────────────────────────────────────────┐
│                 Linux Laptop (CachyOS)                  │
├─────────────────┬─────────────────┬────────────────────┤
│   nocodo CLI    │  Manager Daemon │   Manager Web      │
│   (Rust)        │  (Rust + Actix) │   (SolidJS)        │
├─────────────────┼─────────────────┼────────────────────┤
│                 │                 │                    │
│   AI Tools      │   Unix Socket   │   HTTP Server      │
│   Claude Code   │   Server        │   localhost:8081   │
│   Gemini CLI    │   SQLite DB     │   Static Files     │
│   etc.          │   File System   │   WebSocket        │
│                 │                 │                    │
└─────────────────┴─────────────────┴────────────────────┘
```

### Communication Flow

1. **User** ↔ **Manager Web** (HTTP/WebSocket on localhost:8081)
2. **nocodo CLI** ↔ **Manager Daemon** (Unix socket at `/tmp/nocodo-manager.sock`)
3. **AI Tools** → **nocodo CLI** → **Manager Daemon**
4. **Manager Web** ↔ **Manager Daemon** (Internal API calls)

## Core Components

### 1. Manager Daemon (Rust + Actix Web)

**Primary Responsibilities:**
- Local project management and orchestration
- Unix socket server for CLI communication
- HTTP API server for Web app communication
- Development environment setup and maintenance
- AI session coordination
- File system operations and project scaffolding

**Key Features for MVP:**
- Project creation, analysis, and management
- AI tool integration and session management
- Real-time communication with Web app via WebSocket
- Local SQLite database for project metadata
- Basic security and input validation
- System service management (optional nginx setup)

**Simplified Database Schema:**
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

-- Activity log
CREATE TABLE activity_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id TEXT,
    action TEXT NOT NULL,
    details TEXT,
    timestamp INTEGER NOT NULL
);
```

**API Endpoints (MVP):**
```rust
// Project Management
GET    /api/projects                    // List all projects
POST   /api/projects                    // Create new project
GET    /api/projects/{id}               // Get project details
DELETE /api/projects/{id}               // Delete project

// Work Management (formerly AI Integration)
POST   /api/work                        // Start work session
GET    /api/work/{id}                   // Get work session status
POST   /api/work/{id}/message           // Send message to work session
DELETE /api/work/{id}                   // End work session

// File System
GET    /api/files                       // Browse project files
POST   /api/files                       // Create file/directory
PUT    /api/files/{path}                // Update file content
DELETE /api/files/{path}                // Delete file/directory

// System
GET    /api/system/status               // Get system status
GET    /api/system/tools                // List available AI tools
```

**Configuration (~/config/nocodo/manager.toml):**
```toml
[server]
unix_socket_path = "/tmp/nocodo-manager.sock"
http_port = 8081
static_files_path = "./web/dist"

[development]
workspace_root = "~/projects/nocodo-workspace"
default_shell = "fish"

[database]
path = "~/.local/share/nocodo/manager.db"

[ai_tools]
claude_code_path = "claude"  # Assumes claude is in PATH
timeout_seconds = 300
max_concurrent_sessions = 3

[logging]
level = "info"
file = "~/.local/share/nocodo/manager.log"
```

### 2. Manager Web App (SolidJS + TypeScript)

**Primary Responsibilities:**
- Chat interface for AI-assisted development
- Project dashboard and file management
- Real-time updates via WebSocket
- Code editing and preview capabilities

**Key Features for MVP:**
- Simple chat interface with message history
- Project selection and basic file browser
- Monaco editor for code viewing/editing
- Real-time AI responses
- Basic project creation workflow
- System status display

**Core Components:**
```typescript
// Main App Structure
interface AppState {
  projects: Project[];
  currentProject: Project | null;
  chatMessages: Message[];
  isAiTyping: boolean;
  files: FileNode[];
  activeFile: ProjectFile | null;
}

// Core Pages
- /dashboard - Project overview
- /project/{id}/chat - AI chat interface
- /project/{id}/files - File browser and editor
- /settings - Basic configuration
```

**Build Setup:**
- Vite for development server and building
- Tailwind CSS for styling
- SolidJS with TypeScript
- Generated types from Manager via ts-rs
- Hot reload for development

### 3. nocodo CLI (Rust)

**Primary Responsibilities:**
- Integration with AI coding tools
- Context-aware prompt generation
- Project analysis and recommendations
- Code validation against basic guardrails
- Communication bridge with Manager daemon

**Key Commands for MVP:**
```bash
# Project analysis
nocodo analyze                          # Analyze current project
nocodo analyze --path ./my-project      # Analyze specific project

# Prompt generation
nocodo prompt "add authentication"      # Generate context-aware prompt
nocodo prompt --file src/main.rs "refactor this"

# Project management
nocodo init rust-web-api my-project    # Initialize new project
nocodo status                          # Show project status

# AI integration
nocodo session claude "create a REST API" # Start AI session
nocodo validate --file src/lib.rs      # Validate code quality
```

**Configuration (~/.config/nocodo/cli.toml):**
```toml
[general]
default_project_path = "~/projects"
editor = "code"

[ai_tools]
preferred_tool = "claude"
timeout = 300

[communication]
manager_socket = "/tmp/nocodo-manager.sock"
connection_timeout = 10

[guardrails]
enabled = true
severity_threshold = "warning"
```

## Development Environment Setup

### Prerequisites
- CachyOS Linux (your current setup)
- Rust toolchain (stable)
- Node.js and npm
- Git
- AI coding tools (Claude Code, Gemini CLI, etc.)

### Installation Process

1. **Clone and Build Manager:**
```bash
cd ~/Projects/nocodo
cargo build --release --bin nocodo-manager
sudo cp target/release/nocodo-manager /usr/local/bin/
```

2. **Build and Install CLI:**
```bash
cargo build --release --bin nocodo-cli
sudo cp target/release/nocodo-cli /usr/local/bin/nocodo
```

3. **Build Web App:**
```bash
cd manager-web
npm install
npm run build
# Built files will be served by Manager daemon
```

4. **Create Configuration:**
```bash
mkdir -p ~/.config/nocodo ~/.local/share/nocodo
cp configs/manager.toml ~/.config/nocodo/
cp configs/cli.toml ~/.config/nocodo/
```

5. **Start Manager Daemon:**
```bash
nocodo-manager --config ~/.config/nocodo/manager.toml
```

6. **Access Web Interface:**
Navigate to http://localhost:8081

## MVP Workflow

### 1. Project Creation
```bash
# Via CLI
nocodo init rust-web-api my-awesome-app
cd my-awesome-app

# Or via Web interface
# Navigate to http://localhost:8081
# Click "New Project" → Select template → Enter name
```

### 2. AI-Assisted Development
```bash
# Start AI session via CLI
nocodo session claude "I want to add user authentication to this Rust web API"

# Or via Web interface
# Navigate to project chat → Type message → Get AI response with context
```

### 3. Code Validation
```bash
# Validate code quality
nocodo validate --file src/auth.rs

# Analyze project structure
nocodo analyze
```

## File Structure

```
nocodo/
├── Cargo.toml              # Workspace configuration
├── manager/                # Manager daemon source
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs
│   │   ├── api/            # HTTP API handlers
│   │   ├── socket/         # Unix socket server
│   │   ├── project/        # Project management
│   │   ├── ai/             # AI integration
│   │   └── db/             # Database operations
│   └── migrations/         # SQLite migrations
├── cli/                    # nocodo CLI source
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs
│   │   ├── commands/       # CLI command implementations
│   │   ├── analysis/       # Project analysis
│   │   ├── prompts/        # Prompt generation
│   │   └── client/         # Manager communication
│   └── templates/          # Project templates
├── manager-web/            # Web app source
│   ├── package.json
│   ├── vite.config.ts
│   ├── src/
│   │   ├── App.tsx
│   │   ├── components/     # Reusable components
│   │   ├── pages/          # Route components
│   │   ├── stores/         # State management
│   │   ├── api/            # API client
│   │   └── types/          # TypeScript types (generated)
│   └── dist/               # Built static files
├── configs/                # Configuration templates
│   ├── manager.toml
│   └── cli.toml
└── docs/                   # Documentation
    └── MVP.md              # This file
```

## Key Technologies

### Manager Daemon
- **Rust** with **Actix Web** for HTTP server
- **Tokio** for async runtime
- **SQLite** with **rusqlite** for local database
- **ts-rs** for TypeScript type generation
- **serde** for serialization
- **tracing** for logging

### Manager Web App
- **SolidJS** with **TypeScript**
- **Vite** for development and building
- **Tailwind CSS** for styling
- **Solid Query** for API state management
- **Monaco Editor** for code editing
- **WebSocket** for real-time communication

### nocodo CLI
- **Rust** with **clap** for command-line interface
- **Tokio** for async operations
- **Unix sockets** for Manager communication
- **Handlebars** for template rendering
- **serde** for JSON handling

## Security Considerations (MVP)

1. **Local-only access** - All services bind to localhost only
2. **File system restrictions** - CLI operates only within designated project directories
3. **Input validation** - Basic validation of all user inputs
4. **Process isolation** - AI tools run as separate processes
5. **Socket permissions** - Unix socket with appropriate file permissions

## Testing Strategy

### Unit Tests
- Manager daemon: API endpoints, project operations, AI integration
- CLI: Command parsing, project analysis, communication client
- Web app: Component behavior, API client, state management

### Integration Tests
- End-to-end project creation workflow
- AI session management and communication
- File operations and project structure
- CLI-to-Manager communication via Unix socket

### Manual Testing
- Web interface functionality
- AI tool integration
- Project template generation
- Error handling and recovery

## Deployment (Local)

### Development Mode
```bash
# Terminal 1: Start Manager daemon
cd ~/Projects/nocodo
cargo run --bin nocodo-manager -- --config ~/.config/nocodo/manager.toml

# Terminal 2: Start Web development server (optional, for hot reload)
cd manager-web
npm run dev

# Terminal 3: Use CLI
nocodo --help
```

### Production Mode
```bash
# Build all components
./scripts/build-all.sh

# Install binaries
sudo ./scripts/install.sh

# Start as system service (optional)
sudo systemctl enable --now nocodo-manager
```

## Success Metrics

The MVP will be considered successful when:

1. ✅ **Manager daemon** starts successfully and serves the Web app
2. ✅ **Web interface** loads and allows basic project management
3. ✅ **CLI** can analyze projects and generate prompts
4. ✅ **AI integration** works with at least one tool (Claude Code)
5. ✅ **Project creation** workflow completes end-to-end
6. ✅ **Real-time communication** works between Web app and daemon
7. ✅ **File operations** work correctly (create, read, update, delete)
8. ✅ **Basic guardrails** validate simple code quality issues


## Implementation Timeline

**Week 1-2: Core Infrastructure**
- Set up Rust workspace and basic project structure
- Implement Manager daemon with basic HTTP server
- Create SQLite database schema and migrations
- Basic Unix socket communication

**Week 3-4: Project Management**
- Implement project CRUD operations
- File system operations and project scaffolding  
- Basic project templates (Rust, Python, Node.js)
- Project analysis and structure detection

**Week 5-6: AI Integration**
- nocodo CLI basic structure and commands
- AI tool integration framework
- Context-aware prompt generation
- Basic guardrails and code validation

**Week 7-8: Web Interface**
- SolidJS app setup with basic routing
- Chat interface and real-time WebSocket communication
- Project dashboard and file browser
- Monaco editor integration

**Week 9-10: Polish & Testing**
- End-to-end testing and bug fixes
- Documentation and installation scripts
- Performance optimization
- Security review and hardening

This MVP provides a complete AI-assisted development environment entirely on your Linux laptop.
