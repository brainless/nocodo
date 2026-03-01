# AGENTS

## Purpose

This repository is a minimal template for coding-agent-driven fullstack development.

## Non-Negotiable Pattern

- Shared contracts are defined in Rust inside `shared-types`.
- TypeScript types are generated from Rust.
- Backend and frontend apps consume the same shared contract.
- Keep code minimal and typed.

## Working Agreements

- Add new features through `shared-types` first, not UI-first.
- Keep endpoints small and explicit.
- Use strict typed states for API/domain behavior.
- Avoid premature abstractions in template stage.

## Naming and Init

- Project naming comes from root `project.conf`.
- Do not hardcode repo/app names in scripts/configs.
- Use `scripts/init-project.sh` to apply configured names.

## Current Scope

Only maintain these parts:

- `backend`
- `shared-types`
- `gui`
- `admin-gui`
- `scripts`

Do not add extra services/crates unless explicitly requested.
