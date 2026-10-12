# nocodo — Product Requirements (Experiment 1)

**Status:** Draft. Epics 001–007 are done; epic 008 is built and waits for its manual checkpoint; epic 009 is next.
**Date:** 2026-10-09, updated 2026-10-12 for epic 008.

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

**Agent count.** Experiment 1 uses **one LLM agent, the Schema agent**. Everything else is plain code. We add another agent only when a checkpoint shows that plain code can't do the job (see §8). Epics 004–007 confirmed this so far: question handling, destructive confirmation and retries all work as plain code.

**Scenario.** A user describes their daily money flow in plain language, for example: "I want to keep an eye on my family's money. My salary comes in every month, we spend on the household, and sometimes I lend money to my brother. We have two bank accounts." The system then:

1. Asks at most one round of domain-level questions (at most 8), each with numbered choices.
2. Produces a change set, which updates `schema.json`, a typed description of tables, columns and relations.
3. Generates SQL migrations from the change set deterministically.
4. Checks the result deterministically: static rules, a dry run on a copy of the DB (up → down → up), and a comparison of the resulting DB with the spec.
5. Backs up `data.sqlite` and applies the migration to it.
6. Shows the user a plain-language summary of what the tool will remember.

The user can then add a follow-up ("I'd also like to know who in the family each spend was for"). The system must evolve the schema with an **additive migration**, keep existing data, and not ask again what was already answered. A follow-up that only names new entries ("We opened a business account") is data, not schema, and must change nothing.

### In scope

| Component | Kind | Role in Experiment 1 |
|---|---|---|
| Schema agent | LLM — the only agent in Experiment 1 | Turns a prompt, plus any answers so far, into a `SchemaReply`: either questions for the user or a change set, with assumptions. |
| Checker | Deterministic | Validates change sets (names, references, conventions, destructive changes), dry-runs the generated SQL, checks the applied DB against the spec, and checks questions (technical words, choices, defaults, count, re-asking). Its findings go back to the Schema agent for repair. An optional LLM review, if added later, only produces advice or questions. |
| SQL generator | Deterministic | Applies a change set to `schema.json` and generates `up.sql`/`down.sql`, including SQLite table rebuilds. |
| Orchestrator | Deterministic library | Owns the project folder and event log, and runs the pipeline: prompt → reply → check → write `schema.json` and the migration → apply. Also the question presenter: IDs, ordering, duplicates, recording answers, resuming the run, and the destructive confirmation. This replaces a PM agent. |
| Runtime | Deterministic | Backs up and applies migrations, checks the result, runs `inspect` (tables, row counts and a plain-language summary) and fills sample rows for testing. |
| Engine | Deterministic library (`nocodo-engine`, epic 008) | The only home of operations (init, changes, prompt, questions, answer, continue, apply, inspect, sample rows, project list, row reading). Implements the orchestrator's `Proposer` and `Applier` over the Schema agent and the runtime, and holds a per-project write lock so one writer runs at a time. |
| CLI | Binary `nocodo` (epic 008) | Prompt, answer questions, apply, inspect. Parses arguments, calls the engine directly and prints the result. |
| API | Binary `nocodo-api` (Axum, epic 008) | The same operations over HTTP on localhost: slow ones run in the background and return a run ID, progress is read from events. Calls the engine and nothing else. |
| GUI | Binary `nocodo-gui` (epic 008) | One column: project dropdown, prompt box, open questions, summary and the tables as a grid. Talks to the API over HTTP only, a local one it starts or a remote one (`--server`). |

### Out of scope (for Experiment 1)

- A PM agent. It is deferred; see §8.
- Import, Rules and Reports agents. These are the next experiments, in that order.
- Reading sample files from `inputs/`. `init` creates the folder, but no agent reads it yet; see §8.
- Runtime AI: LLM calls inside the finished tool.
- Multi-user access, auth, TLS, sync, cloud hosting, editing rows in the GUI.
- Any database other than SQLite.
- Generated Rust code of any kind.

## 4. Fixed decisions

These are settled and are not revisited in this experiment:

- **Stack:** Rust, SQLite3, Diesel 2.3, `diesel_migrations`, Axum, Rig 0.43.
- **Any model provider Rig supports.** Each project picks a provider and model per agent role in `project.json`. nocodo adds providers Rig lacks (such as `neuralwatt`) itself.
- **No Rust toolchain on the user's machine.** We ship prebuilt `nocodo` (CLI), `nocodo-api` (server) and `nocodo-gui` binaries. User projects consist only of data and specs.
- **User data is relational.** Each table in `schema.json` becomes a real SQLite table.
  - Migrations are plain SQL, run at runtime with `FileBasedMigrations::from_path`.
  - Queries against user tables go through `diesel_dynamic_schema` or generated SQL.
  - Diesel's compile-time DSL is used only for nocodo's own internal tables, if any.
- **LLMs never write SQL.** They emit a typed `SchemaReply` through a Rig extractor. Everything below that level is deterministic.
- **Build and run are separate.**
  - Builder agents *define* specs.
  - The runtime *processes* data according to those specs.
  - Agents never process user data row by row.
- **One writer per artifact.** Each spec file has exactly one owning agent.
- **Single user, local only.** "Family finance" means people stored *as data*, not as accounts that log in.

## 5. Project folder layout

Each user project lives in one folder. Everything nocodo knows about the project lives there. `nocodo init` (or "New project..." in the GUI) creates it.

```
<project>/
  project.json            # spec_version, project name, per-role model config (provider, model, strict)
  events.jsonl            # append-only log: prompts, questions and answers, proposals, checks, applies
  specs/
    schema.json           # owner: Schema agent — must match the accepted change sets in events.jsonl
  migrations/
    <NNNN>_<name>/        # zero-padded sequence number, not a timestamp, so replay is byte-identical
      up.sql
      down.sql
      metadata.toml
  inputs/                 # user-supplied sample files (read-only for agents; not read yet)
  data.sqlite
  .lock                   # write lock, held while a writing operation runs
  backups/                # before_<NNNN>_<name>.sqlite, copied before every migration
  .agent-logs/            # <run_id>.jsonl: every LLM request/response, retry and tool call; gitignored
```

New projects copy their model settings from `project-base.json` at the repo root.

## 6. Core concepts

### Events
Every state change is a typed event appended to `events.jsonl`. Each event has a sequence number, an actor (developer, system, user or Schema agent with its run ID) and a self-explanatory `detail`:

- project created; user prompt; prompt failed (with the run ID to look up)
- questions asked, rejected (with the checker findings), dropped as duplicates; question answered
- change set proposed, rejected (with findings) or accepted (with the checks it passed)
- migration applied or failed; database checked against the spec; sample rows added

`schema.json` is a materialized view of this log: the orchestrator rebuilds it from the accepted change sets and refuses to work if the file on disk differs. A question's status is derived from the log the same way.

### Questions
The Schema agent writes the text and choices; plain code does everything else. A `Question` has:

| Field | Meaning |
|---|---|
| `id` | `q1`, `q2`, … one sequence per project, assigned by the orchestrator |
| `asked_by` | `schema` (the agent) or `system` (plain code, such as the destructive confirmation) |
| `text` | The question, in domain language |
| `choices` | 2 to 5 options, numbered from 1 everywhere: in the command, the events and the stored answer |
| `default` | Choice used when the user accepts the defaults; absent only for blocking questions |
| `blocking` | Must be answered explicitly |
| `why` | For the reviewer, not shown to the user: what in the change set depends on the answer |

Rules for questions:

- **One agent round per prompt.** A Schema agent call returns either questions or a change set. Once a round is answered, the next call must return a change set. The destructive confirmation is a separate `system` round.
- **Choices only.** Every answer is one of the agent's choices; the user cannot type a free answer.
- **No re-asking.** The agent sees every earlier answer in the project; the checker rejects a question that was already answered.
- **No technology.** Text and choices must not contain a word from `TECH_WORDS` ("table", "column", "SQL", "database", "ID", …). The checker rejects them, and the agent repairs the round.
- Answers can arrive over several commands. Answering the last open question resumes the run.
- Plain code orders questions (blocking first), drops duplicates and records everything in `events.jsonl`.
- Minor guesses not worth a question go into `assumptions`. Assumptions describe what will be remembered, never what will be worked out or entered for the user.

### Schema changes
A closed enum: `CreateTable`, `AddColumn`, `AddIndex`, `AddForeignKey`, `RenameTable`, `RenameColumn`, `DropColumn`, `DropTable`. A change set adds a snake_case `name` and a plain-language `summary`.

Destructive variants (`DropColumn`, `DropTable`) need the user's explicit confirmation. Plain code builds the question from the descriptions in `schema.json`, for example: "This will forget the notes you have entered on each expense. Continue?" It is asked only once the change set passes every other check, and nothing is written until the answer is yes.

### Fixed schema conventions
Every table generated by the system follows these conventions:

- Implicit `id` (integer primary key), `created_at` and `updated_at` columns, defined once in `nocodo-types` and never listed in the spec.
- Column types: `text`, `integer`, `boolean`, `date`, `date_time`, `money` and `decimal`.
- Money stored as integer minor units, never floats; `decimal` only for fractions that are not money.
- Dates stored as ISO-8601 text.
- `snake_case` plural table names; references named `<singular_table>_id`.
- Every table and column has a plain-language description.

The Schema agent's preamble adds modeling rules that the checker enforces where it can: real references instead of text links, lookup tables for repeating values, facts rather than computed results, only what was asked, new entries are data rather than schema, and additive follow-ups.

### Determinism
| Layer | Deterministic? | How |
|---|---|---|
| LLM to `SchemaReply` | No | Constrained by typed output, validated by the checker with a bounded repair loop, logged to `.agent-logs` |
| Change set to `schema.json` | Yes | Pure function (`nocodo-sqlgen`) |
| Change set to SQL | Yes | Pure function (`nocodo-sqlgen`) |
| Applying migrations | Yes | Dry run on a copy, then backup, apply and check the DB against the spec; restored on failure |
| Replaying a recorded session | Yes (epic 009) | `.agent-logs` and `rig-cassette` replay give byte-identical specs |

## 7. Success criteria

Experiment 1 succeeds if, across at least three different scenarios (personal finance, a household inventory and a small client and invoice log), plus the farm scenario for a data-only follow-up:

1. **No technical questions.** Every question shown to the user is in domain language and offers choices.
2. **Few questions.** No more than 8 questions per initial scenario.
3. **No LLM-written SQL.** 100% of migrations come from `nocodo-sqlgen`.
4. **Nothing invalid gets applied.** No invalid migration ever reaches `data.sqlite`; the checker catches everything.
5. **Relational output.** The schema has real tables and foreign keys. A developer reviewing it would call it reasonable: no key-value tables and no JSON-blob columns standing in for structure.
6. **Additive evolution.** A follow-up requirement produces an additive migration, and existing data survives.
7. **Replay is exact.** Replaying a recorded session gives byte-identical `schema.json` and migrations.
8. **Debuggable.** A developer can reconstruct why each table exists by reading `events.jsonl` and `.agent-logs`.

Results are written up in `docs/experiment-1-results.md`.

## 8. Risks and open questions

- **Question quality.** In epic 007 the questions stayed within budget and used no technical words, but they were mostly shallow yes/no questions; the inventory prompt asked none, and follow-ups asked nothing even where choices were open. Asking too few gives a poor schema; asking too many loses the user. Measure both.
- **Preamble rules the agent still misses.** Epic 007 saw a data-only follow-up change the schema, and a change-set summary promise a computed value. These are prompt-quality problems; fix them in the preamble and checker before adding agents.
- **Schema churn across LLM runs.** The same prompt may produce different schemas on different runs. Record how much they vary. Consider stronger models or few-shot examples for the Schema agent before adding more agents.
- **Provider failures.** Retryable provider errors are retried 3 times per run; anything else ends in `prompt_failed`, which `continue` retries. The `continue` path after a real failed call has only been tested with scripted proposers.
- **Cranelift compatibility.** Some dependencies (crypto and TLS with assembly, SIMD-heavy crates) may not build or may misbehave under the Cranelift backend. Release builds use LLVM. Per-crate fallbacks are tracked in `AGENTS.md`.
- **Is a PM agent needed at all?** Decision: no, not until a checkpoint shows otherwise. Epic 007 built the question presenter as plain code in the orchestrator, and it worked. The `nocodo-agent-pm` crate stays an empty skeleton.
- **Sample files in `inputs/`.** No epic reads them yet. Decide whether the Schema agent should read them in Experiment 1, or leave them to the Import experiment.
- **Detailed vs. plain prompts.** The detailed fixtures (`fixtures/prompts/<s>/0N-*.md`) stay for regression runs of spec → SQL → database. Claim 3 is tested only with the plain ones (`plain-0N.md`).

## 9. Roadmap

We build in small steps. Each epic does the minimum work needed to test one thing and ends with a **checkpoint**. From epic 003 on, every checkpoint is a manual session: the tester runs real commands against a project folder under `.projects/` (gitignored), and a reviewer, human or Claude, reads `events.jsonl`, `.agent-logs/`, `specs/schema.json`, `migrations/`, `backups/` and `data.sqlite`. Review details go in `epics-notes/<epic>.md`. We look at the results before planning more epics. If a checkpoint fails, we fix that layer or rethink the hypothesis; we don't move on and pile more on top.

Only the next epic is planned in detail. Rows marked *tentative* get written as epics after the checkpoints before them.

| Epic | Status | Scope | Checkpoint (go / no-go) |
|---|---|---|---|
| 001 | Done | Workspace and crate layout, toolchain. No behavior. | Builds and passes clippy with Cranelift. |
| 002 | Done | `nocodo-types`, minimal: `SchemaSpec`, `SchemaChange`, `ChangeSet`, `Event`, `ProjectConfig`. | Types round-trip through JSON; the tester reviews the type vocabulary and example JSON. |
| 003 | Done | `nocodo-sqlgen` + deterministic `nocodo-checker` + minimal orchestrator (project folder, event log). `nocodo init` and `changes` take hand-written change sets. No LLM. | Hand-written change sets produce correct, reversible SQLite migrations, including table rebuilds; invalid ones are rejected and logged. |
| 004 | Done | Schema agent: minimal `nocodo-agent` base (Rig + `.agent-logs`). `nocodo prompt` turns a prompt into a change set and runs it through the 003 pipeline, feeding checker errors back to the agent. | On the three scenarios plus a follow-up each, the schemas are good relational designs and the follow-up is additive. |
| 005 | Done | Schema agent hardening: modeling rules, one definition of the implicit columns, reference rules, `decimal`, our own extraction retry loop with argument repair, any Rig provider, opt-in strict tools, checks recorded in events. | Every Schema agent run is trustworthy and inspectable. |
| 006 | Done | `nocodo-runtime`: backup, apply to `data.sqlite`, check against the spec, `inspect`, plain-language summary, `sample-rows`. | A follow-up change applies on top of real rows and the data survives. |
| 007 | Done | Questions: one typed round per prompt, the question checks, `questions` / `answer` / `continue`, the destructive confirmation, provider retries, `prompt_failed`, plain-language prompt fixtures. | Plain-language prompts on the four scenarios: ≤ 8 questions, none technical. Passed, with findings carried forward (§8). |
| 008 | Built, checkpoint pending | `nocodo-engine` as the one home of operations, with a per-project write lock, a project list and a row reader. Three thin front ends: the `nocodo` CLI (engine directly), the `nocodo-api` Axum server and the `nocodo-gui` window (API over HTTP only): project dropdown, prompt, questions, summary, tables as a grid. | A person drives the whole flow in the GUI (also against a separate `nocodo-api` with `--server`); a CLI write during a run is refused as busy; findings go in `epics-notes/008-engine-api-gui.md`. |
| 009 | *Tentative* | Replay of recorded sessions through `.agent-logs` and `rig-cassette`; the findings carried from epic 007 and epic 008 (shallow questions, preamble misses, a missing model key hanging a run, clearer bad-answer wording); the results write-up. | Success criteria 1–8 in §7. Write up `docs/experiment-1-results.md`. |

Later, only if Experiment 1 passes: an optional LLM review layer for the checker, then the Import, Rules and Reports agents, **each in its own epic with its own checkpoint**.
