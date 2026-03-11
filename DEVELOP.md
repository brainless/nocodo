# DEVELOP

This is a minimal template for fullstack development (human or agent). Shared Rust types drive everything.

## Scope

Only maintain these parts: `backend`, `shared-types`, `gui`, `admin-gui`, `scripts`.
Do not add extra services or crates unless explicitly requested.

## Type-Driven Workflow

1. Define API/domain types in `shared-types/src/*.rs`
2. Regenerate TypeScript types: `cargo run -p shared-types --bin generate_api_types` → `gui/src/types/api.ts`
3. Implement backend handler using shared types
4. Implement UI in `gui`/`admin-gui` against generated types

A feature is complete only when backend + frontend compile against the same shared contract.
Start from `shared-types`, never UI-first. Keep endpoints small and explicit. Prefer strict enums/newtypes over free-text states.

## Project Naming

- Root config: `project.conf` (copy from `project.conf.template`)
- Apply names: `scripts/init-project.sh`
- Never hardcode app/repo names in scripts or configs — always parameterize by `PROJECT_NAME`.

## Configuration

Config is resolved in priority order: **env var → `project.conf` → `server.env`** (sibling to the binary on server).

- `project.conf` — local development; read by backend and vite apps at dev/build time
- env vars — override `project.conf`; injected by systemd on server
- `server.env` — server-only secrets (e.g. `DATABASE_URL`); written by `setup-server.sh` to `DEPLOY_ROOT`, permissions `600`; auto-discovered by backend binaries via `current_exe()` path lookup

Backend helper binaries (`src/bin/`) include both `config.rs` and `db.rs` via `#[path]` and resolve config through `read_project_conf`. No manual env setup needed on the server.

Vite apps (`gui`, `admin-gui`) read `project.conf` at build/dev time via `vite.config.ts` — they do not use `server.env`.

## Structure Rules

- `shared-types` is the source of truth for API payloads
- Avoid handwritten duplicate API types in frontend apps
- Avoid premature abstractions — keep code minimal and typed
