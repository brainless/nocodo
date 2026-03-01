# App Template

Minimal fullstack template for typed feature development with coding agents.

## Stack

- Rust + Actix Web (`backend`)
- Rust shared types with TypeScript generation (`shared-types`)
- TypeScript + SolidJS + Solid Router + Tailwind + DaisyUI (`gui`, `admin-gui`)
- Bash scripts for server setup and deploy (`scripts`)

## Name Configuration

This template uses one config at project root:

- `project.conf`

Start by copying:

```bash
cp project.conf.template project.conf
```

Set at minimum:

- `PROJECT_NAME`
- `PROJECT_TITLE`
- `DB_KIND` (`sqlite` or `postgres`)
- `SERVER_IP`
- `SSH_USER`
- `DOMAIN_NAME`
- `LETSENCRYPT_EMAIL`

Then apply naming across crate/package/docs:

```bash
scripts/init-project.sh
```

`init-project.sh` also sets the backend default Cargo feature based on `DB_KIND`:
- `sqlite` -> `db-sqlite`
- `postgres` -> `db-postgres`

## What This Template Includes

- `GET /api/heartbeat` in backend
- Startup DB schema migration execution (feature-gated by backend DB feature)
- Shared `HeartbeatResponse` type defined in Rust
- Generated TypeScript type consumed by GUI
- `gui`: Hello World + heartbeat status
- `admin-gui`: Hello World admin placeholder
- `systemd` service template for backend
- `nginx` site template for GUI + `/api` reverse proxy
- certbot setup flow for TLS certificates
- pre-commit hook for Rust and frontend checks
- auth starter files for Casbin (`backend/authz`) and SQL migrations (`backend/migrations`)

## Project Layout

- `backend/`: Actix API crate
- `shared-types/`: canonical API types + TS generator
- `gui/`: main SolidJS app
- `admin-gui/`: admin SolidJS app
- `scripts/`: setup, init, deploy, and server config templates
  - `scripts/init-project.sh`
  - `scripts/setup-server.sh`
  - `scripts/deploy.sh`
  - `scripts/configs/backend.service.template`
  - `scripts/configs/nginx.conf.template`
  - `scripts/configs/nginx-temp-cert.conf.template`

## Local Run

1. Generate TypeScript API types:

```bash
cargo run -p shared-types --bin generate_api_types
```

2. Run backend:

```bash
cargo run -p app-backend
```

3. Run main GUI:

```bash
cd gui
npm install
npm run dev
```

4. Run admin GUI:

```bash
cd admin-gui
npm install
npm run dev
```

Open:

- main GUI: `http://127.0.0.1:3030`
- admin GUI: `http://127.0.0.1:3031`

## Git Hooks

Install repository-managed hooks:

```bash
scripts/install-git-hooks.sh
```

Pre-commit checks:

- `authz` model/migration sync check
- `cargo fmt --all --check`
- `cargo check --workspace`
- `cargo test --workspace`
- `gui`: `prettier --check .` and `npm run build`
- `admin-gui`: `prettier --check .` and `npm run build`

## Deploy Pattern

1. `scripts/setup-server.sh`
2. `scripts/deploy.sh`

Both scripts read `project.conf` by default. You can pass a custom config path if needed.

`deploy.sh` uploads full source via `scp`, builds backend on server, installs `systemd` and `nginx` config, and keeps certbot renew timer enabled.

## Development Model

Read `DEVELOP.md` for the type-driven feature workflow.
