# nocodo

nocodo is a local AI-assisted development environment that provides guardrails and good software engineering practices for code generation. The system runs entirely on your Linux machine, providing a complete development workflow without cloud dependencies.

> **MVP Focus**: Currently implementing a minimal viable product that runs locally on Linux laptops, focusing on two core components: Manager daemon and Manager Web app.

> ⚠️ **Note**: The nocodo CLI has been removed as part of issue #80. The nocodo CLI is no longer included in this repository.

> [!NOTE]
> All paths are relative from the root of the project.

## MVP Architecture (Current Focus)

The nocodo MVP consists of two core components running locally on your Linux laptop:

```
┌─────────────────────────────────────────────────────────┐
│                 Linux Laptop (Local)                    │
├─────────────────────────────────┬────────────────────┤
│          Manager Daemon         │   Manager Web      │
│       (Rust + Actix)           │   (SolidJS) ⚡     │
└─────────────────────────────────┴────────────────────┘
```

### Core Components:

- **Manager Daemon**: Local orchestration service that manages projects and provides APIs (runs on localhost:8081)
- **Manager Web App**: Chat-based interface for AI interaction and project management (runs on localhost:3000)

### Communication:

- **AI Tools ↔ Manager Daemon**: Direct HTTP API integration (localhost:8081)
- **Manager Web App ↔ Manager Daemon**: HTTP/WebSocket via Vite proxy (localhost:3000 → localhost:8081)

## MVP Quick Start

### Prerequisites
- Linux laptop (tested on CachyOS Linux)
- Rust toolchain
- Node.js and npm
- AI coding tools (Claude Code, Gemini CLI, etc.)

### Installation
```bash
# Build Manager daemon (API server only)
cargo build --release --bin nocodo-manager
sudo cp target/release/nocodo-manager /usr/local/bin/

# Install Web app dependencies
cd manager-web
npm install

# Start Manager daemon (API on localhost:8081)
nocodo-manager --config ~/.config/nocodo/manager.toml

# Start Web app (dev server on localhost:3000)
cd manager-web && npm run dev
```

### Usage
```bash
# Access web interface at http://localhost:3000
# Web app proxies API requests to manager on localhost:8081
# Users access port 3000 for web interface, which communicates with API on 8081
```

---

## Future Features (Post-MVP)

> The following sections describe planned features that will be implemented after the MVP is complete.

### Bootstrap app (Future Enhancement)

The Bootstrap app will allow users to deploy nocodo to cloud servers. This is planned for post-MVP development.

**Planned Features**:
- Cloud provider integration (Scaleway, DigitalOcean, Vultr, Linode)
- Server provisioning and management
- Authentication with nocodo.com
- Encrypted API key storage
- Server image creation and reuse

### Bootstrap Web app (Future Enhancement)

Web interface for Bootstrap app management. This is planned for post-MVP development.

**Planned Features**:
- Cloud provider API key management
- Server dashboard and monitoring
- Remote server controls

## Manager app

The Manager app is a Linux daemon that provides project management and API services for AI-assisted development. It runs locally and manages projects, AI sessions, and file operations.

Features:

- Project CRUD operations and lifecycle management
- AI session orchestration and tracking
- File system operations and project structure management
- HTTP API and WebSocket communication
- SQLite database for data persistence
- Configuration management
- Process management for AI tool integration

See [MANAGER.md](./MANAGER.md) for detailed technical specifications.

## Manager Web app

The Manager Web app provides a Lovable-like chat interface for users to interact with AI coding tools and build software projects. It runs on the Operator server and communicates with the Manager daemon to orchestrate development workflows. Users can chat with AI, create projects, manage code generation, and deploy applications through this interface.

Features:

- AI chat interface for software development requests
- Real-time project management and file system browsing
- Code generation workflow orchestration
- Integration with AI tools
- Project templates and scaffolding options
- Live code preview and testing capabilities
- Deployment pipeline management
- Error handling and debugging assistance
- Version control integration
- Collaborative project sharing

See [MANAGER_WEB.md](./MANAGER_WEB.md) for detailed technical specifications.

## Overall technical stack preferences

- Rust with Actix Web for any daemon/backend
- `ts-rs` for generating TypeScript types for API communication (with Web apps to any API)
- Wherever we expect response from an LLM, the client should ask for JSON conforming to TypeScript types, which should also be generated using `ts-rs` since all our clients communicating with LLMs are in Rust
- SQLite for data storage in any daemon/backend
- Migration management should exist from the start
- Vite, SolidJS, TailwindCSS and Solid UI components for all Web interfaces

## MVP Workflow (Current)

1. **Startup**: User starts Manager daemon (API server) and Web app (dev server) separately
2. **Web Interface**: Vite dev server serves Web app at localhost:3000, proxies API to localhost:8081
3. **Project Creation**: User creates a new project via Web interface
4. **Project Setup**: Each project gets its own directory, Git repo, and basic scaffolding
5. **AI Integration**: User interacts with AI tools through:
   - Web chat interface for conversations
6. **Development**: AI tools interact directly with Manager Daemon via API
7. **Project Management**: Manager coordinates between projects, handles higher-level concerns

### Key Interactions:
- **User ↔ Web App**: Chat interface, project management (localhost:3000)
- **AI Tools ↔ Manager Daemon**: Direct HTTP API integration (localhost:8081)
- **Web App ↔ Manager Daemon**: HTTP/WebSocket via Vite proxy (localhost:3000 → localhost:8081)

---

## Future Workflow (Post-MVP)

> This describes the planned cloud-based workflow for post-MVP versions:

- User downloads or starts Bootstrap app
- Bootstrap handles authentication with nocodo.com
- Bootstrap provisions cloud server ("Operator")
- Manager loads on cloud server with full development environment
- Projects get public URLs at `random-slug.nocodo.dev`
- Full deployment pipeline and cloud integrations
