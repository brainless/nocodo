# DEVELOP

## Type-Driven Feature Workflow

This template enforces a typed, shared-contract-first flow:

1. Define API/domain types in `shared-types/src/*.rs`
2. Export TypeScript types from the same Rust types
3. Implement backend handlers using shared types
4. Implement frontend features against generated types

A feature is complete only when backend + frontend apps compile against the same shared contract.

## Structure Rules

- `shared-types` is the source of truth for API payloads.
- Avoid handwritten duplicate API types in frontend apps.
- Backend request/response bodies should use shared types whenever practical.
- Prefer strict enums/newtypes for domain states over free-text values.

## TypeScript Generation

Generate frontend types:

```bash
cargo run -p shared-types --bin generate_api_types
```

Output file:

- `gui/src/types/api.ts`

Regenerate types whenever shared Rust API types change.

## Project Naming

- Root config: `project.conf` (copy from `project.conf.template`)
- Apply configured names: `scripts/init-project.sh`
- Keep scripts/configs name-agnostic and parameterized by `PROJECT_NAME`.

## Adding Future Features

For each new feature:

1. Add or extend types in `shared-types`.
2. Regenerate TypeScript API types.
3. Add backend endpoint in `backend` using shared types.
4. Add UI and state in `gui` and/or `admin-gui` using generated types.
5. Update docs for behavior and constraints.

Use strict shared contracts as the first design boundary, then implement backend and UI around those contracts.
