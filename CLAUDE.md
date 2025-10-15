# nocodo

AI-assisted development environment deployed on cloud servers with secure desktop client access.

## Project Structure
- **manager/**: Rust daemon with Actix Web, SQLite (runs on cloud server, provides API)
- **desktop-app/**: Cross-platform desktop client with egui (SSH connection, port forwarding, GUI)

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
# Build Manager daemon (for cloud deployment)
cargo build --release --bin nocodo-manager

# Build Desktop App (for local distribution)
cargo build --release --bin nocodo-desktop-app
```

## Quick Start (Development)
1. Start Manager daemon: `nocodo-manager --config ~/.config/nocodo/manager.toml` (runs on http://localhost:8081)
2. Start Desktop app: `nocodo-desktop-app` (connects via SSH, opens GUI)
3. AI-powered development environment with cloud backend and native desktop UI

## Tech Stack
- **Backend**: Rust + Actix Web (API only, no web asset serving)
- **Database**: SQLite for data storage
- **Desktop UI**: Rust + egui/eframe (cross-platform native GUI)
- **Communication**: HTTP/WebSocket API over SSH tunnel
- Manager: Cloud server (API on port 8081)
- Desktop: Local machine (SSH client, port forwarding, native UI)
