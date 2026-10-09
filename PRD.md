# nocodo — Product Requirements (Experiment 1)

**Status:** Draft
**Date:** 2026-10-09

---

## 1. Problem

Non-technical people often want small tools that store and organize data over time, such as personal finance trackers, contact lists or inventory logs. Today they have two options:

- **Spreadsheets.** These break down when the data has structure, such as relations between tables, deduplication or history.
- **Hiring a developer.**

LLM coding assistants help developers but not this audience. They produce code the user cannot review, run or maintain. Their output is also non-deterministic, so asking the same thing twice can give a different app.

## 2. Hypothesis

> A set of narrow agents, each owning exactly one part of the software lifecycle, working against a **fixed tech stack**, and producing **typed specs** (not code) that deterministic code turns into real artifacts, can build a durable data tool for a non-technical user without that user ever answering a technical question and without a Rust toolchain on the user's machine.

The hypothesis breaks down into three claims we can test:

1. **Specs, not code.** The LLM output can be limited to typed, versioned specs. All SQL and file layouts are generated deterministically from those specs.
2. **Real relational data.** Specs as data still yield a real relational SQLite database: proper tables, foreign keys and indexes, not a key-value store.
3. **Questions, not jargon.** Agents can get what they need from a non-technical user through a small number of typed, multiple-choice, domain-language questions.

## 3. Smallest experiment

Experiment 1 tests claims 1–3 for **the storage layer only**. Import, rules and reports come in later experiments.

**Scenario.** A user describes their daily money flow in plain language and optionally drops sample bank CSV/XLSX files into the project's `inputs/` folder. For example: "I track my salary, household spending and the money I lend to my brother; we have two bank accounts". The system then:

1. Asks a handful of domain-level questions through the CLI.
2. Produces `schema.json`, a typed description of tables, columns and relations.
3. Generates Diesel-compatible SQL migrations from `schema.json` deterministically.
4. Checks the result deterministically. The generated SQL must dry-run on a copy of the DB.
5. Applies the migrations to `data.sqlite`.
6. Shows the user a plain-language summary of what the tool will remember.

The user can then add a follow-up ("I also want to track which family member a spend was for"). The system must evolve the schema with an **additive migration** and not rebuild it from scratch.

### In scope

| Component | Kind | Role in Experiment 1 |
|---|---|---|
| PM agent | LLM | Talks to the user: takes prompts, presents questions, records answers and summaries. **Does not plan or route.** |
| Schema agent | LLM | Turns requirement facts into `SchemaChange` lists. Asks schema-relevant questions in domain language. |
| Checker | Deterministic (+ optional LLM review) | Validates specs and dry-runs the generated SQL. Its deterministic checks gate every change. An LLM review may only produce advice or questions. |
| SQL generator | Deterministic | Turns a `schema.json` diff into `up.sql`/`down.sql`, including SQLite table rebuilds. |
| Orchestrator | Deterministic library | Handles the event log, subscriptions, single-writer artifact ownership and scheduling. |
| Runtime | Deterministic | Applies migrations, runs `inspect` (lists tables and row counts) and summarizes the schema. |
| API | Axum | Thin HTTP layer over the orchestrator and agents. Runs as a subprocess of the CLI. |
| CLI | Binary | The only user interface: prompt, answer questions, show summaries. |

### Out of scope (for Experiment 1)

- Import, Rules and Reports agents. These are the next experiments, in that order.
- Runtime AI: LLM calls inside the finished tool.
- GUI, multi-user access, auth, sync, cloud hosting.
- Any database other than SQLite.
- Generated Rust code of any kind.

## 4. Fixed decisions

These are settled and are not revisited in this experiment:

- **Stack:** Rust, SQLite3, Diesel 2.3, `diesel_migrations`, Axum, Rig 0.43.
- **No Rust toolchain on the user's machine.** We ship prebuilt `nocodo` (CLI) and `nocodo-api` binaries. User projects consist only of data and specs.
- **User data is relational.** Each table in `schema.json` becomes a real SQLite table.
  - Migrations are plain SQL, run at runtime with `FileBasedMigrations::from_path`.
  - Queries against user tables go through `diesel_dynamic_schema` or generated SQL.
  - Diesel's compile-time DSL is used only for nocodo's own internal tables, if any.
- **LLMs never write SQL.** They emit typed `SchemaChange` values through Rig's `Extractor`. Everything below that level is deterministic.
- **Build and run are separate.**
  - Builder agents *define* specs.
  - The runtime *processes* data according to those specs.
  - Agents never process user data row by row.
- **One writer per artifact.** Each spec file has exactly one owning agent.
- **Single user, local only.** "Family finance" means people stored *as data*, not as accounts that log in.

## 5. Project folder layout

Each user project lives in one folder. Everything nocodo knows about the project lives there.

```
<project>/
  project.json            # spec_version, project name, per-role model config
  events.jsonl            # append-only log: prompts, Q&A, decisions, spec changes
  questions.jsonl         # typed questions and their status
  specs/
    schema.json           # owner: Schema agent — rebuilt from events.jsonl
  migrations/
    <timestamp>_<name>/
      up.sql
      down.sql
  inputs/                 # user-supplied sample files (read-only for agents)
  data.sqlite
  backups/                # data.sqlite copied before every migration
  .agent-logs/            # every LLM request/response and tool call, JSONL; gitignored
```

## 6. Core concepts

### Events
Every state change is an event appended to `events.jsonl`:

- user prompt
- question asked or answered
- requirement fact recorded
- schema change proposed, checked, applied or rejected

Spec files are **materialized views** of this log, and an agent can rebuild them at any time.

### Questions
Agents ask questions through a typed `Question`:

| Field | Meaning |
|---|---|
| `id`, `asked_by`, `artifact` | Who asked, and about which artifact |
| `text` | The question, in domain language |
| `choices` | Options the agent generated in advance (free text only when unavoidable) |
| `default` | Choice used if the user doesn't pick |
| `blocking: bool` | Whether work stops until it's answered |
| `depends_on` | Other questions that must be answered first |
| `status` | Open, answered, and so on |

Rules for questions:

- Before asking, an agent checks whether recorded facts already answer the question.
- The PM agent batches questions, removes duplicates and puts blocking ones first.
- Questions never mention technology ("table", "column", "SQL", "database").

### Schema changes
A closed enum: `CreateTable`, `AddColumn`, `AddIndex`, `AddForeignKey`, `RenameTable`, `RenameColumn`, `DropColumn`, `DropTable`. Destructive variants require the user's explicit confirmation, worded in domain terms: "This will forget the notes you have entered on each expense. Continue?"

### Fixed schema conventions
Every table generated by the system follows these conventions:

- Integer primary key `id`.
- `created_at` and `updated_at` columns.
- Money stored as integer minor units, never floats.
- Dates stored as ISO-8601 text.
- `snake_case` plural table names.

### Determinism
| Layer | Deterministic? | How |
|---|---|---|
| LLM to `SchemaChange` | No | Constrained by typed output, validated by the Checker, recorded for replay |
| `SchemaChange` to `schema.json` | Yes | Pure function |
| `schema.json` diff to SQL | Yes | Pure function (`nocodo-sqlgen`), property-tested |
| Applying migrations | Yes | Backup, then dry-run on a copy, then apply |
| Replaying a recorded session | Yes | `.agent-logs` and `rig-cassette` replay give byte-identical specs |

## 7. Success criteria

Experiment 1 succeeds if, across at least three different scenarios (personal finance, a household inventory and a small client and invoice log):

1. **No technical questions.** Every question shown to the user is in domain language and offers choices.
2. **Few questions.** No more than 8 questions per initial scenario.
3. **No LLM-written SQL.** 100% of migrations come from `nocodo-sqlgen`.
4. **Nothing invalid gets applied.** No invalid migration ever reaches `data.sqlite`; the Checker catches everything.
5. **Relational output.** The schema has real tables and foreign keys. A developer reviewing it would call it reasonable: no key-value tables and no JSON-blob columns standing in for structure.
6. **Additive evolution.** A follow-up requirement produces an additive migration, and existing data survives.
7. **Replay is exact.** Replaying a recorded session gives byte-identical `schema.json` and migrations.
8. **Debuggable.** A developer can reconstruct why each table exists by reading `events.jsonl` and `.agent-logs`.

Results are written up in `docs/experiment-1-results.md`.

## 8. Risks and open questions

- **Schema quality vs. question budget.** Asking too few questions gives a poor schema; asking too many loses the user. Measure both.
- **Schema churn across LLM runs.** The same prompt may produce different schemas on different runs. Record how much they vary. Consider stronger models or few-shot examples for the Schema agent before adding more agents.
- **Cranelift compatibility.** Some dependencies (crypto and TLS with assembly, SIMD-heavy crates) may not build or may misbehave under the Cranelift backend. Release builds use LLVM. Track any per-crate fallbacks in `AGENTS.md`.
- **Is the PM agent needed at all?** If the Schema agent's questions can go straight to the CLI with a deterministic batcher, the PM agent may be unnecessary. Earlier experiments found coordination roles added overhead. Measure whether the PM adds value beyond intake.

## 9. Roadmap

We build in small steps. Each epic does the minimum work needed to test one thing and ends with a **checkpoint**. We look at the results before starting the next epic. If a checkpoint fails, we fix that layer or rethink the hypothesis; we don't move on and pile more agents on top.

| Epic | Scope | Checkpoint (go / no-go) |
|---|---|---|
| 001 | Workspace and crate layout, toolchain. No behavior. | Builds and passes clippy with Cranelift. |
| 002 | `nocodo-types`, minimal: `SchemaSpec`, `SchemaChange`, `Event`, `Question`. Only what Epic 003 needs. | Types round-trip through JSON. |
| 003 | `nocodo-sqlgen` + deterministic `nocodo-checker`. No LLM. | Hand-written `SchemaChange` lists produce correct, reversible SQLite migrations, including table rebuilds. |
| 004 | **Schema agent alone.** Minimal `nocodo-agent` base (Rig + `.agent-logs`). Inputs come from fixture requirement files through a dev-only CLI command. No PM, no orchestrator. | **First real test of the hypothesis:** on the three scenarios, does it produce good relational schemas and domain-language questions? |
| 005 | **PM agent alone.** Intake plus question presentation in the CLI, with answers written to `events.jsonl`. | Are the questions few, non-technical and useful? Does the PM add value over a deterministic question batcher? |
| 006 | Minimal `nocodo-orchestrator`: connects PM and Schema through the event log. | A follow-up prompt produces an additive migration end to end. |
| 007 | `nocodo-runtime` (apply, backup, inspect) + API + CLI end to end, with replay through recorded logs. | Success criteria 1–8 in §7. Write up `docs/experiment-1-results.md`. |

Later, only if Experiment 1 passes: an optional LLM review layer for the Checker, then the Import, Rules and Reports agents, **each in its own epic with its own checkpoint**.
