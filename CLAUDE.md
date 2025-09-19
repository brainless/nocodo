# nocodo

Local AI-assisted development environment providing guardrails and software engineering practices.

## Project Structure
- **manager/**: Rust daemon with Actix Web, SQLite, Unix socket server (API only)
- **manager-web/**: SolidJS web interface with Vite dev server (proxies to manager API)

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
# Build Manager daemon only (no web embedding)
cargo build --release --bin nocodo-manager

# Build Web app (for production deployment)
cd manager-web && npm install && npm run build
```

## Quick Start
1. Start Manager daemon: `nocodo-manager --config ~/.config/nocodo/manager.toml` (runs on http://localhost:8081)
2. Start Web app: `cd manager-web && npm run dev` (runs on http://localhost:3000 with API proxy)
3. Access Web interface at http://localhost:3000
4. AI-powered development environment with separate frontend and backend

## Tech Stack
- Rust + Actix Web for backends (API only, no web asset serving)
- SQLite for data storage
- SolidJS + TailwindCSS for web interfaces (separate Vite dev server)
- HTTP/WebSocket API communication with Vite proxy
- Manager: localhost:8081 (API server), Web: localhost:3000 (dev server)
