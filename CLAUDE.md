# nocodo

Local AI-assisted development environment providing guardrails and software engineering practices.

## Project Structure
- **manager/**: Rust daemon with Actix Web, SQLite, Unix socket server
- **manager-web/**: SolidJS web interface served at localhost:8081

## Development Workflow

* Create a new branch for each task
* Branch names should start with `feature/`, `chore/`, or `fix/`
* Add tests for any new features added, particularly integration or end-to-end tests
* Run formatters, linters, and tests before committing changes
* When finished, please commit and push to the new branch
* Please mention the GitHub issue if provided
* Commit small chunks
* Selectively add files to git; maintain `.gitignore`
* If working on a GitHub issue: create a PR, update the task in the end
* If working on a GitHub issue: do not close the issue until I manually test

## Build Commands
```bash
# Build all Rust components
cargo build --release

# Build Manager daemon
cargo build --release --bin nocodo-manager

# Build Web app
cd manager-web && npm install && npm run build
```

## Quick Start
1. Start Manager daemon: `nocodo-manager --config ~/.config/nocodo/manager.toml`
2. Access Web interface at http://localhost:8081
3. AI-powered development environment with integrated tools

## Tech Stack
- Rust + Actix Web for backends
- SQLite for data storage
- SolidJS + TailwindCSS for web interfaces
- Internal API communication
- HTTP/WebSocket for Web app ↔ Manager communication
