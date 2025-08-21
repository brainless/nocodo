# nocodo

Local AI-assisted development environment providing guardrails and software engineering practices for CLI-based coding tools.

## Project Structure
- **manager/**: Rust daemon with Actix Web, SQLite, Unix socket server
- **cli/**: Rust CLI tool that communicates with manager daemon via Unix socket  
- **manager-web/**: SolidJS web interface served at localhost:8081

## Development Workflow
- Create a new branch for each task
- Branch names should start with chore/ or feature/ or fix/
- Please add tests for any new features added, particularly integration tests
- Please run formatters, linters and tests before committing changes
- When finished please commit and push to the new branch
- Please mention GitHub issue if provided
- After working on an issue from GitHub, update issue's tasks and open PR

## Build Commands
```bash
# Build all Rust components
cargo build --release

# Build Manager daemon
cargo build --release --bin nocodo-manager

# Build CLI
cargo build --release --bin nocodo-cli

# Build Web app
cd manager-web && npm install && npm run build
```

## Quick Start
1. Start Manager daemon: `nocodo-manager --config ~/.config/nocodo/manager.toml`
2. Access Web interface at http://localhost:8081
3. Use CLI commands like `nocodo analyze` or `nocodo session claude "add authentication"`
4. AI tools interact with nocodo CLI for context and guardrails

## Tech Stack
- Rust + Actix Web for backends
- SQLite for data storage
- SolidJS + TailwindCSS for web interfaces
- Unix sockets for CLI ↔ Manager communication
- HTTP/WebSocket for Web app ↔ Manager communication
