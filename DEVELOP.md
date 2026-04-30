# DEVELOP

Minimal template for fullstack development. Shared Rust types drive everything.

## Scope

Maintain: `backend`, `shared-types`, `agents`, `schema-codegen`, `gui`, `admin-gui`, `tauri`, `scripts`.

## Workspace

- `backend` — Actix-web API with auto-migrations, CORS for gui/admin-gui origins
- `agents` — LLM agent crate with SQLite-backed storage (schema designer agent active)
- `shared-types` — API contract types with TypeScript generation
- `schema-codegen` — Deterministic `SchemaDef` → Rust structs + SQLite DDL generator

## Type-Driven Workflow

1. Define types in `shared-types/src/*.rs`
2. Regenerate TypeScript: `cargo run -p shared-types --bin generate_api_types` → `gui/src/types/api.ts`
3. Implement backend handler using shared types
4. Implement UI in `gui`/`admin-gui` against generated types

A feature is complete only when backend + frontend compile against the same shared contract. Start from `shared-types`, never UI-first.

## Project Naming

- Root config: `project.conf` (copy from `project.conf.template`)
- Apply names: `scripts/init-project.sh`
- Never hardcode app/repo names — always parameterize by `PROJECT_NAME`.

## Configuration

Resolved priority: **env var → `project.conf` → `server.env`** (sibling to binary on server).

- `project.conf` — local development; read by backend and vite apps
- env vars — override `project.conf`; injected by systemd on server
- `server.env` — server-only secrets (e.g. `DATABASE_URL`); written by `setup-server.sh` to `DEPLOY_ROOT`, permissions `600`

## Agents

- Active agent: `schema_designer` — designs SQLite schemas via LLM
- Storage: SQLite (`nocodo.db`) with chat sessions and generated schemas
- Agent crate (`agents/`) — business logic only: agent implementations, storage traits, LLM integration
- Backend crate (`backend/`) — HTTP layer: API endpoints live in `backend/src/agents_api/`
- API endpoints:
  - `POST /api/agents/schema-designer/chat` — send message, returns `{session_id, message_id}`
  - `GET /api/agents/schema-designer/messages/{id}/response` — long-poll for response (text/schema/stopped)
  - `GET /api/agents/schema-designer/sessions/{id}/messages` — fetch session history
  - `GET /api/agents/schema-designer/sessions/{id}/codegen` — generate Rust structs + SQLite DDL from the session's latest schema
- Config: reads `AGENT_PROVIDER` and `AGENT_API_KEY` from env/project.conf
- Backend auto-initializes agent state on startup; runs DB migrations and ensures default project exists

## Desktop (Tauri)

Tauri wraps `admin-gui` and manages `nocodo-backend` as a sidecar binary.

**Dev run from repo root:**
```bash
NOCODO_BACKEND_PATH="$(pwd)/target/debug/nocodo-backend" npm --prefix tauri run dev
```

- Tauri `beforeDevCommand` builds backend, copies to `tauri/bin/` with target triple, starts admin-gui dev server
- Sidecar detection: `NOCODO_BACKEND_PATH` → `DWATA_API_PATH` (compat) → bundled binary → `target/debug/nocodo-backend`
- Backend runs in its own process group (Unix) for clean shutdown on app exit
- `tauri.conf.json` devUrl: `http://127.0.0.1:6626` (admin-gui dev server)

## Structure Rules

- `shared-types` is the source of truth for cross-service API payloads (Rust ↔ TypeScript)
- Agent-specific API types live in `backend/src/agents_api/{agent}/` close to their handlers
- Avoid handwritten duplicate API types in frontend apps
- Avoid premature abstractions — keep code minimal and typed
- Agent processing runs in background tasks with in-memory response storage for long-polling
- Business logic stays in `agents` crate; HTTP layer stays in `backend` crate
