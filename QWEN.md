# nocodo - Qwen Code Context

## Project Overview
nocodo is a local AI-assisted development environment that provides guardrails and good software engineering practices for code generation. It runs entirely on Linux machines without cloud dependencies.

> ⚠️ **Note**: The nocodo CLI has been removed as part of issue #80. The nocodo CLI is no longer included in this repository.

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

## Core Components (MVP)
1. **Manager Daemon** (Rust + Actix): Local orchestration service with:
   - HTTP API server (localhost:8081) for Web app
   - SQLite database for project metadata
   - AI session management

2. **Manager Web App** (SolidJS): Chat-based interface for AI interaction with:
   - Project dashboard and file management
   - Real-time WebSocket communication
   - Code editing capabilities

## Key Technologies
- **Backend**: Rust, Actix Web, SQLite, ts-rs (TypeScript type generation)
- **Frontend**: SolidJS, TypeScript, Vite, Tailwind CSS
- **Infrastructure**: HTTP/WebSocket for Web↔Manager

## Communication Flow
1. User ↔ Manager Web (HTTP/WebSocket on localhost:8081)
2. AI Tools ↔ Manager Daemon (Direct integration)
3. Manager Web ↔ Manager Daemon (Internal API calls)

## How to use nocodo MVP
1. Start Manager daemon: `nocodo-manager --config ~/.config/nocodo/manager.toml`
2. Access Web interface at http://localhost:8081
3. AI tools interact directly with Manager Daemon

## File Structure
```
nocodo/
├── manager/       # Manager daemon (Rust)
├── manager-web/   # Web app (SolidJS/TypeScript)
├── specs/         # Technical specifications
└── configs/       # Configuration templates
```
