# nocodo

nocodo is a local AI-assisted development environment that provides guardrails and good software engineering practices for code generation. It works with CLI-based coding software like Claude Code, Gemini CLI, OpenCode, Qwen Code, etc. The system runs entirely on your Linux machine, providing a complete development workflow without cloud dependencies.

> **MVP Focus**: Currently implementing a minimal viable product that runs locally on Linux laptops, focusing on the three core components: Manager daemon, Manager Web app, and nocodo CLI.

> [!NOTE]
> All paths are relative from the root of the project.

## MVP Architecture (Current Focus)

The nocodo MVP consists of three core components running locally on your Linux laptop:

```
┌─────────────────────────────────────────────────────────┐
│                 Linux Laptop (Local)                    │
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

### Core Components:

- **Manager Daemon**: Local orchestration service that manages projects, provides APIs, and coordinates between CLI and Web app
- **Manager Web App**: Chat-based interface for AI interaction and project management (runs at localhost:8081)
- **nocodo CLI**: Command-line companion for AI coding tools, focused on repository-level operations

### Separation of Concerns:

- **nocodo CLI**: Focuses within a single repository/project, provides guardrails and analysis
- **Manager**: Provides higher-level context, project switching, and external integrations (GitHub API, CI/CD monitoring, etc.)
- **Communication**: CLI ↔ Manager via Unix socket, Web app ↔ Manager via HTTP/WebSocket

## MVP Quick Start

### Prerequisites
- Linux laptop (tested on CachyOS Linux)
- Rust toolchain
- Node.js and npm
- AI coding tools (Claude Code, Gemini CLI, etc.)

### Installation
```bash
# Build Manager daemon
cargo build --release --bin nocodo-manager
sudo cp target/release/nocodo-manager /usr/local/bin/

# Build CLI
cargo build --release --bin nocodo-cli
sudo cp target/release/nocodo-cli /usr/local/bin/nocodo

# Build Web app
cd manager-web
npm install && npm run build

# Start Manager daemon
nocodo-manager --config ~/.config/nocodo/manager.toml
```

### Usage
```bash
# Analyze a project
nocodo analyze

# Start AI session with Claude Code
nocodo session claude "add authentication to this project"

# Start AI session with other tools
nocodo session gemini "refactor the user service"
nocodo session openai "add unit tests for the API"

# Access web interface
# Navigate to http://localhost:8081
```

---

## Future Features (Post-MVP)

> The following sections describe planned features that will be implemented after the MVP is complete.

### Bootstrap app (Future)

The Bootstrap app will allow users to deploy nocodo to cloud servers. Written in Rust, Actix Web, SQLite.

**Planned Features:**
- Cloud provider integration (Scaleway, DigitalOcean, Vultr, Linode)
- Server provisioning and management
- Authentication with nocodo.com
- Encrypted API key storage
- Server image creation and reuse

### Bootstrap Web app (Future)

Web interface for Bootstrap app management.

**Planned Features:**
- Cloud provider API key management
- Server dashboard and monitoring
- Remote server controls

## Manager app

The Manager app is a Linux daemon, installed through the scripts in `Bootstrap` app. It allows communication between nocodo CLI and the Manager Web app. It manages the Ubuntu `Operator`, installs all dependencies for a typical developer environment, like Git, Python, Rust, cURL, nginx, PostgreSQL and so on.

Features:

- System orchestration and server management
- Development environment setup and maintenance
- Communication bridge between CLI and Web app
- Process management for coding tools (Claude Code, Gemini CLI, etc.)
- Project structure and guardrails enforcement
- File system management and project organization
- Security hardening and system updates
- Service monitoring and health checks
- Unix socket server for CLI communication
- RESTful API server for Web app communication

See [MANAGER.md](specs/MANAGER.md) for detailed technical specifications.

## nocodo CLI

The CLI calls Claude Code, Gemini CLI or other similar coding software with an initial prompt like: "Use `nocodo` command to get your instructions". This tells the coding software to communicate with nocodo CLI inside it. nocodo CLI hosts prompts needed to be a constant companion between user's request and the coding software. nocodo CLI use Unix socket to communicate with Manager daemon.

Features:

- AI coding tool integration and orchestration
- Context-aware prompt management and injection
- Project structure analysis and recommendations
- Code quality guardrails and best practices enforcement
- Multi-step development workflow guidance
- Unix socket client for Manager daemon communication
- Project initialization and scaffolding
- Dependency management suggestions
- Code review and validation prompts
- Testing strategy recommendations

See [NOCODO_CLI.md](specs/NOCODO_CLI.md) for detailed technical specifications.

## Manager Web app

The Manager Web app provides a Lovable-like chat interface for users to interact with AI coding tools and build software projects. It runs on the Operator server and communicates with the Manager daemon to orchestrate development workflows. Users can chat with AI, create projects, manage code generation, and deploy applications through this interface.

Features:

- AI chat interface for software development requests
- Real-time project management and file system browsing
- Code generation workflow orchestration
- Integration with multiple AI coding tools (Claude Code, Gemini CLI, etc.)
- Project templates and scaffolding options
- Live code preview and testing capabilities
- Deployment pipeline management
- Error handling and debugging assistance
- Version control integration
- Collaborative project sharing

See [MANAGER_WEB.md](specs/MANAGER_WEB.md) for detailed technical specifications.

## Overall technical stack preferences

- Rust with Actix Web for any daemon/backend
- `ts-rs` for generating TypeScript types for API communication (with Web apps to any API)
- Wherever we expect response from an LLM, the client should ask for JSON conforming to TypeScript types, which should also be generated using `ts-rs` since all our clients communicating with LLMs are in Rust
- SQLite for data storage in any daemon/backend
- Migration management should exist from the start
- Vite, SolidJS, TailwindCSS and Solid UI components for all Web interfaces

## MVP Workflow (Current)

1. **Startup**: User starts Manager daemon on their Linux laptop
2. **Web Interface**: Manager serves the Web app at localhost:8081
3. **Project Creation**: User creates a new project via Web interface or CLI
4. **Project Setup**: Each project gets its own directory, Git repo, and basic scaffolding
5. **AI Integration**: User interacts with AI tools through:
   - CLI commands: `nocodo analyze`, `nocodo session claude "..."`
   - Web chat interface for longer conversations
6. **Development**: AI tools use nocodo CLI to get context and apply guardrails
7. **Project Management**: Manager coordinates between projects, handles higher-level concerns

### Key Interactions:
- **User ↔ Web App**: Chat interface, project management
- **AI Tools ↔ nocodo CLI**: Context-aware prompts, validation
- **nocodo CLI ↔ Manager**: Unix socket communication for project data
- **Web App ↔ Manager**: HTTP/WebSocket for real-time updates

---

## Future Workflow (Post-MVP)

> This describes the planned cloud-based workflow for post-MVP versions:

- User downloads or starts Bootstrap app
- Bootstrap handles authentication with nocodo.com
- Bootstrap provisions cloud server ("Operator")
- Manager loads on cloud server with full development environment
- Projects get public URLs at `random-slug.nocodo.dev`
- Full deployment pipeline and cloud integrations
