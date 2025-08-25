# nocodo – OpenAI (Codex/ChatGPT) Context

## Project Overview
nocodo is a local AI-assisted development environment that provides guardrails and good software engineering practices for code generation. It runs entirely on your Linux machine and works with CLI-based coding tools (Claude Code, Gemini CLI, Qwen Code, OpenAI/Codex/ChatGPT CLIs).

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
1. Manager Daemon (Rust + Actix): HTTP API (localhost:8081), WebSocket, SQLite, Unix socket for CLI
2. Manager Web App (SolidJS): Project dashboard, sessions list/detail, real-time updates
3. nocodo CLI (Rust): Repository-focused companion to invoke coding tools with context/guardrails

## Quick Start with OpenAI/Codex
1. Start Manager daemon:
   - `nocodo-manager --config ~/.config/nocodo/manager.toml`
2. Access Web interface:
   - http://localhost:8081
3. Use CLI to start an AI session with OpenAI/Codex/ChatGPT:
   - `nocodo session openai "add unit tests for the API"`
   - `nocodo session openai "refactor the user service for clarity"`

Notes:
- Ensure your OpenAI CLI/tooling is installed and in PATH. Configure credentials securely via environment variables as per the tool’s instructions (avoid printing secrets).
- nocodo enhances your prompt with project context fetched from the Manager and records the session/outputs.

## How nocodo integrates OpenAI/Codex
- Context-aware prompt: nocodo CLI builds an enhanced prompt using project metadata from the Manager.
- Tool execution: nocodo invokes the configured tool (e.g., `openai`) non-interactively and captures stdout/stderr.
- Session tracking: Manager records session start, outputs, and completion/failure for viewing in Manager Web.

## Recommended Practices
- Keep prompts concrete and reference files/paths relative to the project root.
- Ask the tool to validate changes against repository constraints (tests, lint, formatting).
- Use small, iterative requests; review diffs and run tests between steps.

## Troubleshooting
- "Tool not found": confirm `openai` CLI is installed and on PATH.
- Authentication errors: ensure credentials are set via environment variables (do not echo secrets in terminals or logs).
- No outputs recorded: check Manager daemon logs and session status in the web app.

## File Structure (context)
- manager/: daemon (Rust, Actix, SQLite)
- cli/: nocodo CLI (Rust)
- manager-web/: web app (SolidJS/TypeScript)
- specs/: technical docs

## See also
- README.md for architecture, quick start, and workflows
- specs/NOCODO_CLI.md, specs/MANAGER.md, specs/MANAGER_WEB.md for technical details
