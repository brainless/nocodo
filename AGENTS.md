# AGENTS.md — Guide for Coding Agents

This document defines how coding agents work on the nocodo codebase. Read it and `PRD.md` before touching anything.

## What nocodo is

nocodo is a set of narrow, close-to-deterministic agents that build data tools (CLI first) for **non-technical users**.

- Each agent owns exactly one part of the software lifecycle and one artifact.
- Agents produce **typed specs, not code**. Deterministic Rust code turns specs into real artifacts such as SQL migrations, and a prebuilt runtime runs them.
- User projects contain only data and specs. Users never need a Rust toolchain.

Experiment 1 covers only the storage layer. It has **one LLM agent, the Schema agent**. Everything else is deterministic: a SQL generator and checker, an orchestrator, a runtime, an API and a CLI. The PM agent is deferred; `nocodo-agent-pm` stays an empty skeleton. See `PRD.md` §3.

## Before starting any task

1. Find the active epic in `epics/`: the lowest-numbered one without `Status: Done`. Read epic files directly; don't trust any other doc for epic status.
2. Read the full epic before touching any file.
3. Do only what the epic asks. Each epic ends with a **checkpoint** the user evaluates before the next epic starts. Do not build ahead: no extra crates, dependencies, agents or features "for later".
4. When an epic is complete, set `Status: Done` and add a short **Task Notes** section: what was built, what was tested, anything surprising.

## Toolchain

- **Nightly + Cranelift for dev builds.**
  - `rust-toolchain.toml` pins `nightly` with the `rustc-codegen-cranelift` component.
  - `.cargo/config.toml` enables `codegen-backend = "cranelift"` for `profile.dev`.
- **Release builds use LLVM** (the default backend).
- **`debug = 0` and `strip = "none"`** in `profile.dev`, for disk space and speed. `strip = "none"` stops rustc calling the toolchain's `rust-objcopy`, which is broken on this machine: it can't find `libLLVM.dylib`. The user's global `~/.cargo/config.toml` also sets an sccache wrapper; don't override it.
- **If a dependency doesn't build under Cranelift,** don't switch the whole workspace to LLVM. Add a per-package override, and record it in the table below with the reason.

| Package | Override | Reason |
|---|---|---|
| `rig-core` (via `nocodo-agent`) | Feature `native-tls` instead of the default `rustls`, with `default-features = false` | Default `rustls` pulls `aws-lc-sys`. Under Cranelift on macOS its symbols link as `__aws_lc_*` but are defined as `_aws_lc_*` (double underscore), so any test or binary that links it fails with "symbol(s) not found". A per-package `codegen-backend = "llvm"` on `aws-lc-sys`/`aws-lc-rs` did not help. `native-tls` uses Security.framework and builds under Cranelift. Release builds are unaffected. |

Cranelift check for epic 005 Task 6 (all registry providers via `rig-core` 0.43 with `native-tls`): no further override needed; tests that link the registry build and pass.

### Commands

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
```

All four must pass before an epic is marked done.

### Manual test loop

From epic 003 on, each checkpoint ends with a manual session:

1. The tester creates a project with `cargo run --bin nocodo-api -- init --project .projects/<name>` and then runs the epic's commands against it. `.projects/` is gitignored. `ANTHROPIC_API_KEY` goes in a gitignored `.env` at the repo root; see `.env.example`. Per-agent model choices (and future project settings) live in `project-base.json` at the repo root; `init` copies them into the new project's `project.json` (`--base <file>` uses another file).
2. A reviewer (the user, or Claude when asked) reads the project folder: `events.jsonl`, `.agent-logs/*.jsonl`, `specs/schema.json`, `migrations/`, and `data.sqlite` through `sqlite3`.

Design every command so this works:

- Output goes to files in the project folder, not only to the terminal.
- Every event and log line is self-explanatory without the terminal session.
- Never ask the tester to create or edit project files by hand. Anything a project needs is created by a command.

## Repository layout

```
Cargo.toml               # workspace
rust-toolchain.toml
.cargo/config.toml
PRD.md  AGENTS.md
epics/                   # NNN-short-name.md
crates/                  # libraries
  nocodo-types/
  nocodo-sqlgen/
  nocodo-checker/
  nocodo-orchestrator/
  nocodo-runtime/
  nocodo-agent/          # shared agent base
  nocodo-agent-schema/
  nocodo-agent-pm/
api/                     # nocodo-api binary (subcommands now, Axum later)
fixtures/                # hand-written change sets and prompts for manual tests
.projects/               # local test projects (gitignored)
cli/                     # nocodo binary (user-facing)
```

## Crate responsibility boundaries

| Crate | Owns | Must NOT |
|---|---|---|
| `nocodo-types` | Shared, versioned, serde types: events, `SchemaSpec`, `SchemaChange`, `ChangeSet`, project config (questions later) | Do I/O, use an LLM, contain business logic |
| `nocodo-sqlgen` | Pure functions: apply `SchemaChange`s to a `SchemaSpec`, and generate `up.sql`/`down.sql` from a spec plus a change list (including SQLite table rebuilds) | Do I/O, use an LLM. **This is the determinism core.** |
| `nocodo-checker` | Deterministic validation of change sets and generated SQL: dry-run on a copy of the DB, up → down → up, and checking the resulting DB against the spec | Let an LLM decide pass or fail. An LLM review layer, if added later, only produces advice or questions. |
| `nocodo-orchestrator` | The project folder (`init`, `project.json`), the event log (`events.jsonl`), and the pipeline that takes a proposed `ChangeSet` through sqlgen and the checker, then writes `schema.json` and the migration. It writes artifacts only on behalf of their owner. | Use an LLM. Routing is plain code. |
| `nocodo-runtime` | Applies migrations (`FileBasedMigrations::from_path`), backups, `inspect`, schema summaries; later the spec interpreter and runtime AI | Write specs |
| `nocodo-agent` | Shared agent base: Rig wiring, per-role model config, `.agent-logs` JSONL logging, reading documents from allowed folders, emitting `Question`s | Contain any role-specific prompt or logic |
| `nocodo-agent-schema` | Turns prompts (and later, answers) into a typed `ChangeSet`. It is the only source of schema changes outside developer commands, so it owns `specs/schema.json`. The orchestrator writes that file for it. | Write SQL, touch `data.sqlite`, write any other spec |
| `nocodo-agent-pm` | **Deferred.** Empty skeleton. Built only if a checkpoint shows plain code can't present questions well. | Plan, route or schedule other agents. Write specs. |
| `api` | Thin layer over orchestrator, agents and runtime. For now, `clap` subcommands (`init`, `changes`, `prompt`, …); Axum comes in a later epic. | Contain business logic |
| `cli` | User interface; spawns `nocodo-api` as a subprocess and talks HTTP to it | Call agents or the DB directly |

### Dependency direction

```
nocodo-types  ←  everything
nocodo-sqlgen       → types
nocodo-checker      → types, sqlgen
nocodo-orchestrator → types, sqlgen, checker
nocodo-runtime      → types
nocodo-agent        → types
nocodo-agent-schema → types, agent
nocodo-agent-pm     → types, agent
api                 → all library crates
cli                 → types
```

- No cycles.
- Rig and any other LLM dependency may appear **only** in `nocodo-agent` and `nocodo-agent-*` crates.
- Diesel may appear only in `nocodo-runtime` and `nocodo-checker` (and `orchestrator` and `api` through them).

## Core rules

- **LLMs never write SQL, Rust or file layouts.** They emit typed values (Rig `Extractor`), and deterministic code does the rest.
- **User data is relational.** Each table in `schema.json` becomes a real SQLite table. Never introduce key-value or JSON-blob tables to avoid modeling.
- **Questions to users never mention technology** ("table", "column", "SQL", "database", "API"). Every question has typed choices produced by the agent.
- **Never edit an applied migration.** Schema evolution is always a new migration.
- **Before every migration:** back up `data.sqlite`, then dry-run the migration on a copy.
- **Every LLM call is logged** to `<project>/.agent-logs/` as JSONL, with run ID, role and model, so sessions can be replayed (`rig-cassette`).
- **Every spec has `spec_version`.**

## Local source access

Dependencies and references are cloned locally. Read them instead of searching the web or browsing docs.rs. They are **reference-only**: never use them as Cargo path dependencies.

| Project | Local path | Use it for |
|---|---|---|
| **Rig** (0.43) | `~/Projects/rig/` | `crates/rig-core` (providers, completion), `crates/rig-agent/src/extractor.rs` (typed output), `crates/rig-cassette` (record/replay) |
| **Diesel** (2.3) | `~/Projects/diesel/` | `diesel_migrations/src/file_based_migrations.rs` (runtime migrations), `diesel_dynamic_schema/` (queries on runtime-defined tables), SQLite backend |
| **akar** | `~/Projects/akar/` | Reference for epic format and workspace/toolchain conventions |

## What NOT to do

- Don't add agent roles that aren't in the active epic. A new role is a smell: first ask whether a tool call or a checker rule can do the job.
- Don't generate Rust code for user projects.
- Don't ask users about the tech stack.
- Don't add multi-user access, auth, GUI or non-SQLite storage in Experiment 1.
- Don't commit `.agent-logs/`, `target/`, or user project folders.
