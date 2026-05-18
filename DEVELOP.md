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

- Active agents: `project_manager` (PM), `product_owner` (PO), `db_engineer`, `backend_engineer`, `frontend_engineer`, `ui_designer`
- Storage: SQLite (`nocodo.db`) with chat sessions, generated schemas, epics, tasks, comments
- Agent crate (`agents/`) — business logic only: agent implementations, storage traits, LLM integration
- Backend crate (`backend/`) — HTTP layer: API endpoints live in `backend/src/agents_api/`
- User chat API (PM + PO speak concurrently per user message):
  - `POST /api/user-chats` — create session + first message
  - `POST /api/user-chats/{session_id}/messages` — append message (text or structured response)
  - `GET /api/user-chats/{session_id}/messages` — fetch session history
  - `GET /api/user-chats?project_id=X` — list sessions
- Config: reads `AGENT_PROVIDER` and `AGENT_API_KEY` from env/project.conf
- Backend auto-initializes agent state on startup; runs DB migrations and ensures default project exists

## Structured Message Content

`user_chat_message` rows carry a `content_type` column alongside `content`:

| `content_type`        | `content`                          | Sender   |
|-----------------------|------------------------------------|----------|
| `text`                | plain string                       | user or agent |
| `structured_question` | JSON `StructuredQuestion`          | agent (PM) |
| `structured_response` | JSON `StructuredResponse`          | user (widget submit) |

Types live in `agents/src/storage/message_content.rs`. The `MessageContent` enum is the internal representation; storage and LLM history both go through it.

- `MessageContent::to_llm_text()` — compiles structured messages to plain text for LLM context
- `MessageContent::from_row(content_type, content)` — reconstructs from DB columns

`QuestionKind` is the extensibility point: add new variants (e.g. `Rating`, `Scale`) there; `StructuredQuestion` and `StructuredResponse` stay stable.

The `request_user_input` tool (`agents/src/user_input_tool.rs`) is shared between PM and PO. PM currently uses it; PO can be wired up the same way. When an agent calls this tool the backend stores a `structured_question` message and returns to the frontend, which renders radio buttons or checkboxes. The user's submission is stored as `structured_response`.

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
