# nocodo

AI-assisted development environment enabling teams to build and maintain business software with natural language. The system runs on cloud infrastructure with secure remote access via desktop client.

## Architecture

nocodo consists of two core components:

```
┌──────────────────────┐           ┌──────────────────────┐
│   Desktop App        │  SSH +    │  Manager (Cloud)     │
│   (Local Machine)    │  HTTP     │  (Cloud Server)      │
│                      │ ────────> │                      │
│  - SSH Client        │           │  - API Server        │
│  - Port Forwarding   │           │  - AI Agent          │
│  - Key Management    │           │  - Project Database  │
│  - GUI (egui)        │           │  - Git Integration   │
└──────────────────────┘           └──────────────────────┘
```

### Manager (Cloud Daemon)

**Purpose**: Orchestrates AI-assisted development on cloud infrastructure

**Location**: Runs on user's cloud server (future: will run as daemon)

**Key Features**:
- HTTP API server (port 8081) for project management
- Integrated AI coding agent for autonomous development
- SQLite database for projects, works, and session tracking
- Git repository management and version control
- File system operations and project scaffolding
- WebSocket communication for real-time updates
- Multi-user coordination (future)

**Tech Stack**: Rust, Actix Web, SQLite, tokio

**See**: `specs/pre_desktop_app/MANAGER.md` for detailed specifications

### Desktop App (Client)

**Purpose**: Secure remote access to Manager from any user's computer

**Location**: Runs locally on user's machine (Linux, macOS, Windows)

**Key Features**:
- SSH connection with automatic key management
- HTTP port forwarding (localhost → Manager:8081)
- Project browser and management UI
- Work history and message viewer
- Configuration management
- Simple, user-friendly interface (no technical knowledge required)

**Tech Stack**: Rust, egui/eframe, russh, reqwest

## User Workflow

1. **Manager Setup**: User deploys Manager to their cloud server
2. **Desktop Connection**: User opens desktop app, enters server details (hostname, username)
3. **SSH Authentication**: Desktop app handles SSH keys automatically (tries default locations or custom path)
4. **Port Forwarding**: Desktop app establishes encrypted tunnel to Manager API
5. **Development**: User accesses all projects, AI agent, and codebases through desktop UI
6. **Collaboration** (future): Multiple team members connect from their desktop apps to same Manager

## Key Benefits

### Security & Privacy
- All code and data stays on user's cloud infrastructure
- Encrypted SSH tunnels for all communication
- SSH key management hidden behind simple UI
- No vendor access to user projects

### Collaboration
- Multiple users can connect to same Manager instance
- Shared access to projects and AI agent
- Future: Real-time coordination and team features

### Accessibility
- Non-technical users can participate
- Desktop app handles all SSH complexity
- Cross-platform support (Linux, macOS, Windows)
- No manual port forwarding or terminal commands

## Development Commands

```bash
# Build Manager (deploy to cloud)
cargo build --release --bin nocodo-manager

# Build Desktop App (distribute to users)
cargo build --release --bin nocodo-desktop-app

# Run Manager on cloud server
nocodo-manager --config ~/.config/nocodo/manager.toml

# Run Desktop App locally
nocodo-desktop-app

# Test Desktop App CLI mode
nocodo-desktop-app --test [server] [username] [keypath]
```

## Technical Stack Preferences

- **Backend/Daemons**: Rust + Actix Web
- **Databases**: SQLite with migration management
- **Type Safety**: `ts-rs` for TypeScript type generation (if web UI added)
- **Desktop UI**: egui/eframe for cross-platform native GUI
- **Async Runtime**: tokio
- **SSH**: russh for secure connections

## Migration Notes

- **Deprecated**: manager-web (SolidJS web app) - replaced by desktop-app
- **Deprecated**: Bootstrap apps - not planned for development
- **Moved**: Previous specs to `specs/pre_desktop_app/` for reference
- **Focus**: Cloud Manager + Desktop App architecture

## Future Enhancements

- Manager daemon mode with systemd/supervisord
- Multi-user real-time coordination
- Team management and permissions
- Desktop app auto-updates
- Manager deployment automation
- Known hosts verification for SSH
- Multiple Manager support (switch between cloud servers)
