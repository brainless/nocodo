# AGENTS.md — Guide for Coding Agents

This document defines how coding agents work on the nocodo codebase. Read it and `PRD.md` before touching anything.

## What nocodo is

nocodo is a set of narrow, close-to-deterministic agents that build data tools (CLI first) for **non-technical users**.

- Each agent owns exactly one part of the software lifecycle and one artifact.
- Agents produce **typed specs, not code**. Deterministic Rust code turns specs into real artifacts such as SQL migrations, and a prebuilt runtime runs them.
- User projects contain only data and specs. Users never need a Rust toolchain.

Experiment 1 covers only the storage layer. It has **one LLM agent, the Schema agent**. Everything else is deterministic: a SQL generator, a checker, an orchestrator (which also presents questions), a runtime, an engine library, and three thin front ends: a CLI, an HTTP API and a GUI. The PM agent is deferred; `nocodo-agent-pm` stays an empty skeleton. See `PRD.md` §3.

Epics 001–007 are done: the full storage pipeline runs from a plain-language prompt through questions, a checked change set and a migration to `data.sqlite`. Epic 008 (the `nocodo-engine` library with the `nocodo` CLI, the `nocodo-api` server and the `nocodo-gui` window over them) is built and waits for its manual checkpoint. Epic 009 (replay, the findings carried from epic 007 and the results write-up) comes after it.

## Before starting any task

1. Find the active epic in `epics/`: the lowest-numbered one without `Status: Done`. Read epic files directly; don't trust any other doc for epic status. If every epic is done, the next one hasn't been written yet: ask the user.
2. Read the full epic before touching any file, and the previous epic's notes in `epics-notes/` for findings carried forward.
3. Do only what the epic asks. Each epic ends with a **checkpoint** the user evaluates before the next epic starts. Do not build ahead: no extra crates, dependencies, agents or features "for later".
4. When an epic is complete, set `Status: Done` and add a short **Task Notes** section: what was built, what was tested, anything surprising or left open. The full checkpoint review (runs, findings, decisions) goes in `epics-notes/<NNN-short-name>.md`.

## Toolchain

- **Nightly + Cranelift for dev builds.**
  - `rust-toolchain.toml` pins `nightly` with the `rustc-codegen-cranelift` component.
  - `.cargo/config.toml` enables `codegen-backend = "cranelift"` for `profile.dev`.
- **Release builds use LLVM** (the default backend).
- **`debug = 0` and `strip = "none"`** in `profile.dev`, for disk space and speed. `strip = "none"` stops rustc calling the toolchain's `rust-objcopy`, which is broken on this machine: it can't find `libLLVM.dylib`. The user's global `~/.cargo/config.toml` also sets an sccache wrapper; don't override it.
- **If a dependency doesn't build under Cranelift,** don't switch the whole workspace to LLVM. Add a per-package override, and record it in the table below with the reason.

| Package | Override | Reason |
|---|---|---|
| `rig-core` (via `nocodo-agent`) | Feature `native-tls` instead of the default `rustls`, with `default-features = false` | Default `rustls` pulls `aws-lc-sys`. Under Cranelift on macOS its symbols link as `__aws_lc_*` but are defined as `_aws_lc_*` (double underscore), so any test or binary that links it fails with "symbol(s) not found". A per-package `codegen-backend = "llvm"` on `aws-lc-sys`/`aws-lc-rs` did not help. `native-tls` uses Security.framework and builds under Cranelift. Release builds are unaffected. All Rig providers in the registry build and pass under this override. |

### Commands

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
```

All four must pass before an epic is marked done. `cargo test` never makes live LLM calls; use a scripted `Proposer` or model instead.

### Manual test loop

Each checkpoint ends with a manual session:

1. **Setup.** The tester creates a project with `cargo run --bin nocodo -- init --project .projects/<name>` (or with "New project..." in the GUI). `.projects/` is gitignored. Provider API keys go in a gitignored `.env` at the repo root; see `.env.example`. Per-agent model choices (provider, model, `strict`) live in `project-base.json` at the repo root; `init` copies them into the new project's `project.json` (`--base <file>` uses another file).
2. **Run.** The tester runs the epic's commands against the project with the `nocodo` CLI (`cargo run --bin nocodo -- <command> --project <dir>`), or clicks through the GUI (`cargo run --bin nocodo-gui`; it starts a local `nocodo-api` on `.projects/` unless given `--server <url>`). The GUI offers the project dropdown, `prompt`, the open questions with "Use defaults" and "Answer", Retry (`continue`), the removal confirmation, the summary, one tab per list with its rows, and "Add sample rows". The CLI keeps every command and also has `changes`, `apply` and `inspect`. Both go through `nocodo-engine`, so they give the same results:
   - `changes <file>` proposes a hand-written change set (`fixtures/change-sets/`), with no LLM. `prompt --file <file>` (or `prompt "<text>"`) asks the Schema agent (`fixtures/prompts/`).
   - When the Schema agent asks questions, `prompt` stops and records them in `events.jsonl`. `questions` shows them, `answer q1=2 q3=1` answers them (choices numbered from 1; `--defaults` accepts the default for open non-blocking ones), and `continue` retries a failed model call or resumes an answered round. Answering the last open question resumes the run on its own.
   - A change set that removes data asks for confirmation the same way. `--confirm-destructive` skips the question, for scripted runs.
   - An accepted change set is applied to `data.sqlite` at once, after a backup. `apply` applies anything still pending.
   - A project accepts one writer at a time. A second write (from the CLI while the GUI or API is running a prompt) is refused as busy; reads (`questions`, `inspect`, rows in the GUI) still work.
   - `scripts/gui-check.sh` replays the server side of each GUI action with curl against a real `nocodo-api`, so an agent can check them without a window (`NOCODO_CHECK_LIVE=1` also sends a real prompt).
   - `sample-rows` fills every list with a few fake rows, so a follow-up can be tested on top of data. `inspect` shows the tables, row counts and the plain-language summary.
3. **Review.** A reviewer (the user, or Claude when asked) reads the project folder: `events.jsonl`, `.agent-logs/*.jsonl`, `specs/schema.json`, `migrations/`, `backups/`, and `data.sqlite` through `sqlite3`. Findings go in `epics-notes/`.

Design every command so this works:

- Output goes to files in the project folder, not only to the terminal.
- Every event and log line is self-explanatory without the terminal session: say what happened, why, and what to run next.
- Never ask the tester to create or edit project files by hand. Anything a project needs is created by a command.

## Repository layout

```
Cargo.toml               # workspace
rust-toolchain.toml
.cargo/config.toml
PRD.md  AGENTS.md
project-base.json        # model settings copied into every new project
.env.example             # provider API keys; copy to .env (gitignored)
epics/                   # NNN-short-name.md: the plan and Task Notes
epics-notes/             # NNN-short-name.md: checkpoint reviews and findings
crates/                  # libraries
  nocodo-types/
  nocodo-sqlgen/
  nocodo-checker/
  nocodo-orchestrator/
  nocodo-runtime/
  nocodo-agent/          # shared agent base
  nocodo-agent-schema/
  nocodo-agent-pm/       # empty skeleton
  nocodo-engine/         # every operation, the Proposer/Applier implementations, the write lock
api/                     # nocodo-api: Axum HTTP server over the engine (library + binary)
cli/                     # nocodo: clap subcommands over the engine (the user-facing CLI)
gui/                     # nocodo-gui: akar window that talks to the API over HTTP only
scripts/                 # gui-check.sh: curl check of the server side of the GUI actions
fixtures/
  change-sets/<s>/       # hand-written change sets; bad/ holds ones that must be rejected
  prompts/<s>/           # 0N-*.md detailed prompts (regression), plain-0N.md plain-language prompts
  examples/              # example JSON for the core types
.projects/               # local test projects (gitignored)
```

Scenarios `<s>`: `finance`, `inventory`, `invoices`, `farm`.

## Crate responsibility boundaries

| Crate | Owns | Must NOT |
|---|---|---|
| `nocodo-types` | Shared, versioned, serde types: events, `SchemaSpec`, `SchemaChange`, `ChangeSet`, `Question`, `Answer`, project config, the implicit columns, `TECH_WORDS`, and the helpers that turn names into plain words | Do I/O, use an LLM, contain business logic |
| `nocodo-sqlgen` | Pure functions: apply `SchemaChange`s to a `SchemaSpec`, and generate `up.sql`/`down.sql` from a spec plus a change list (including SQLite table rebuilds) | Do I/O, use an LLM. **This is the determinism core.** |
| `nocodo-checker` | Deterministic validation: static rules on change sets, a dry run of the generated SQL on a copy of the DB (up → down → up), checking an applied DB against the spec, and the question rules (`check_questions`) | Let an LLM decide pass or fail. An LLM review layer, if added later, only produces advice or questions. |
| `nocodo-orchestrator` | The project folder (`init`, `project.json`), the event log (`events.jsonl`), and the pipeline: proposer reply → checker → write `schema.json` and the migration → apply. The question presenter: IDs, ordering, duplicates, answers, resuming (`continue_pending`), and the destructive confirmation. It reaches the agent and the runtime only through its `Proposer` and `Applier` traits, which `nocodo-engine` implements. It writes artifacts only on behalf of their owner. | Use an LLM, or depend on an agent or runtime crate. Routing is plain code. |
| `nocodo-runtime` | Applies migrations (`FileBasedMigrations::from_path`) with a backup first and a restore on failure, `inspect`, schema summaries, sample rows; later the spec interpreter and runtime AI | Write specs |
| `nocodo-agent` | Shared agent base: Rig wiring and the provider registry (including providers nocodo adds), per-role model config, the extraction retry and repair loop, provider retries, `.agent-logs` JSONL logging | Contain any role-specific prompt or logic |
| `nocodo-agent-schema` | Its preamble and input, turning a prompt plus the answers so far into a typed `SchemaReply` (questions or a change set, plus assumptions). It is the only source of schema changes outside developer commands, so it owns `specs/schema.json`. The orchestrator writes that file for it. | Write SQL, touch `data.sqlite`, write any other spec, assign question IDs or decide when to ask the user |
| `nocodo-agent-pm` | **Deferred.** Empty skeleton. Built only if a checkpoint shows plain code can't present questions well. | Plan, route or schedule other agents. Write specs. |
| `nocodo-engine` | The only home of operations: one synchronous function per operation (`init`, `changes`, `prompt`, `questions`, `answer`, `continue_run`, `apply`, `inspect`, `sample_rows`, `list_projects`, `read_rows`) returning typed results from `nocodo-types`. The `Proposer` and `Applier` implementations (it adapts the Schema agent and the runtime). The per-project write lock. | Print, parse arguments, speak HTTP, or duplicate logic that belongs to the orchestrator, checker or runtime |
| `api` (`nocodo-api`) | Axum server over the engine: routes, background runs (one per project), JSON errors, the in-memory run record. Localhost by default. | Contain business logic: no validation, ordering or wording |
| `cli` (`nocodo`) | The user-facing CLI: `clap` subcommands (`init`, `changes`, `prompt`, `questions`, `answer`, `continue`, `apply`, `inspect`, `sample-rows`) that call the engine directly and print its results | Call agents, the runtime or the DB directly; contain business logic |
| `gui` (`nocodo-gui`) | The window (akar), two columns: left sidebar with the project dropdown, prompt box and questions; main column with the summary and the rows grid. Talks HTTP to a `nocodo-api` (spawned locally, or `--server <url>`) | Depend on the engine or any library but `nocodo-types`; call agents or the DB; contain business logic |

### Dependency direction

```
nocodo-types  ←  everything
nocodo-sqlgen       → types
nocodo-checker      → types, sqlgen
nocodo-orchestrator → types, sqlgen, checker
nocodo-runtime      → types, sqlgen
nocodo-agent        → types
nocodo-agent-schema → types, agent
nocodo-agent-pm     → types, agent
nocodo-engine       → types, orchestrator, runtime, agent, agent-schema
api                 → engine, types (orchestrator in tests)
cli                 → engine, types
gui                 → types (and the API over HTTP)
```

- No cycles.
- Rig and any other LLM dependency may appear **only** in `nocodo-agent` and `nocodo-agent-*` crates.
- Diesel may appear only in `nocodo-runtime` and `nocodo-checker` (and the engine, front ends through them).

## Core rules

- **LLMs never write SQL, Rust or file layouts.** They emit typed values (Rig extractor), and deterministic code does the rest.
- **User data is relational.** Each table in `schema.json` becomes a real SQLite table. Never introduce key-value or JSON-blob tables to avoid modeling.
- **Questions to users never mention technology.** Any text shown to the user must pass the shared `TECH_WORDS` check. The agent writes question text and typed choices; plain code does everything else. Choices are numbered from 1 everywhere.
- **Removing data needs the user's confirmation**, asked by plain code in domain words, after every other check passes.
- **Never edit an applied migration.** Schema evolution is always a new migration.
- **Before every migration:** dry-run it on a copy of the DB, then back up `data.sqlite`.
- **`events.jsonl` is the source of truth.** `schema.json` and question status are derived from it. Never record the same prompt or proposal twice; retries continue the existing one.
- **Every LLM call is logged** to `<project>/.agent-logs/` as JSONL, with run ID, role and model, so sessions can be replayed (`rig-cassette`, epic 009).
- **Every spec has `spec_version`.** Old event logs and projects must keep loading.

## Local source access

Dependencies and references are cloned locally. Read them instead of searching the web or browsing docs.rs. They are **reference-only**: never use them as Cargo path dependencies.

| Project | Local path | Use it for |
|---|---|---|
| **Rig** (0.43) | `~/Projects/rig/` (https://github.com/0xPlaygrounds/rig) | `crates/rig-core` (providers, completion), `crates/rig-agent/src/extractor.rs` (typed output), `crates/rig-cassette` (record/replay) |
| **Diesel** (2.3) | `~/Projects/diesel/` (https://github.com/diesel-rs/diesel) | `diesel_migrations/src/file_based_migrations.rs` (runtime migrations), `diesel_dynamic_schema/` (queries on runtime-defined tables), SQLite backend |
| **akar** | `~/Projects/akar/` (https://github.com/brainless/akar) | Reference for epic format and workspace/toolchain conventions. `nocodo-gui` uses its crates as git dependencies, not path dependencies. |
| **protopie** | `~/Projects/protopie/` (https://github.com/brainless/protopie) | Model for the GUI shell (`crates/protopie-gui`: winit + wgpu, `ureq` client, spawned server) |

## What NOT to do

- Don't add agent roles that aren't in the active epic. A new role is a smell: first ask whether a tool call, a checker rule or a preamble rule can do the job.
- Don't generate Rust code for user projects.
- Don't ask users about the tech stack.
- Don't add multi-user access, auth, TLS, row editing in the GUI or non-SQLite storage in Experiment 1.
- Don't commit `.agent-logs/`, `target/`, `.env`, or user project folders.
