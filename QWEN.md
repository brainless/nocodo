# nocodo - Qwen Code Context

## Project Overview
nocodo is a local AI-assisted development environment that provides guardrails and good software engineering practices for code generation. It runs entirely on Linux machines without cloud dependencies.

# Development Workflow
- Create a new branch for each task
- Branch names should start with chore/ or feature/ or fix/
- Please add tests for any new features added, particularly integration tests
- Please run formatters, linters and tests before committing changes
- When finished please commit and push to the new branch
- Please mention GitHub issue if provided
- After working on an issue from GitHub, update issue's tasks and open PR

## Core Components (MVP)
1. **Manager Daemon** (Rust + Actix): Local orchestration service with:
   - Unix socket server for CLI communication
   - HTTP API server (localhost:8081) for Web app
   - SQLite database for project metadata
   - AI session management

2. **Manager Web App** (SolidJS): Chat-based interface for AI interaction with:
   - Project dashboard and file management
   - Real-time WebSocket communication
   - Code editing capabilities

3. **nocodo CLI** (Rust): Command-line companion for AI coding tools with:
   - Project analysis and validation
   - Context-aware prompt generation
   - AI tool integration (Claude Code, Gemini CLI, etc.)

## Key Technologies
- **Backend**: Rust, Actix Web, SQLite, ts-rs (TypeScript type generation)
- **Frontend**: SolidJS, TypeScript, Vite, Tailwind CSS
- **Infrastructure**: Unix sockets for CLI↔Manager, HTTP/WebSocket for Web↔Manager

## Communication Flow
1. User ↔ Manager Web (HTTP/WebSocket on localhost:8081)
2. nocodo CLI ↔ Manager Daemon (Unix socket at `/tmp/nocodo-manager.sock`)
3. AI Tools → nocodo CLI → Manager Daemon
4. Manager Web ↔ Manager Daemon (Internal API calls)

## How to use nocodo MVP
1. Start Manager daemon: `nocodo-manager --config ~/.config/nocodo/manager.toml`
2. Access Web interface at http://localhost:8081
3. Use CLI commands like `nocodo analyze` or `nocodo session claude "add authentication"`
4. AI tools interact with nocodo CLI for context and guardrails

## File Structure
```
nocodo/
├── manager/       # Manager daemon (Rust)
├── cli/           # nocodo CLI (Rust)
├── manager-web/   # Web app (SolidJS/TypeScript)
├── specs/         # Technical specifications
└── configs/       # Configuration templates
```
