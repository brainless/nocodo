# Epic 002: Core Types

**Status:** Not started
**Goal:** Define the minimal typed vocabulary in `nocodo-types` that epic 003 needs: `SchemaSpec`, `SchemaChange`, `ChangeSet`, `Event` and `ProjectConfig`. These types are what the Schema agent will be allowed to say (epic 004), so getting them right matters more than anything else in this epic.

**Prerequisite:** Epic 001.

---

## Context

- `PRD.md` §6 describes events, schema changes and the fixed schema conventions.
- The LLM will later emit `ChangeSet` values. The tighter these types are, the less the checker has to catch. So:
  - Encode conventions in the types wherever possible.
  - Leave out anything the system generates itself, such as `id`, `created_at`, `updated_at` and index names.
- `Question` is **not** in this epic. It arrives with the question epic.
- Types only, no I/O and no logic beyond trivial helpers such as `ChangeSet::is_destructive`.

## Tasks

### Task 1: Dependencies

- Add to `[workspace.dependencies]`:
  - `serde` with `derive`
  - `serde_json`
  - `jiff` with `serde`
- `nocodo-types` depends on `serde` and `jiff`. `serde_json` is only a dev-dependency.

### Task 2: Schema types (`src/schema.rs`)

```rust
pub const SPEC_VERSION: u32 = 1;

pub struct SchemaSpec { pub spec_version: u32, pub tables: Vec<Table> }   // ordered by creation

pub struct Table {
    pub name: String,          // snake_case plural, e.g. "transactions"
    pub description: String,   // plain language: why this table exists
    pub columns: Vec<Column>,  // user columns only; id/created_at/updated_at are implicit
    pub indexes: Vec<Index>,
}

pub struct Column {
    pub name: String,
    pub description: String,
    pub column_type: ColumnType,
    pub nullable: bool,              // default false
    pub unique: bool,                // default false
    pub references: Option<String>,  // table name; implies an Integer FK to that table's id
}

pub enum ColumnType { Text, Integer, Boolean, Date, DateTime, Money }

pub struct Index { pub columns: Vec<String>, pub unique: bool }  // name is derived by sqlgen
```

- There's no `Real`/float type. Money is `Money` (integer minor units), and dates are `Date`/`DateTime` (ISO-8601 text). Add more types only when a checkpoint shows they're needed.
- `Column`, `Table` and `Index` use `#[serde(deny_unknown_fields)]`. `nullable`, `unique` and `references` use `#[serde(default)]`, so hand-written JSON stays short. Serialization always writes every field, so output is stable and explicit.
- Enums serialize in `snake_case`.

### Task 3: Changes (`src/change.rs`)

```rust
#[serde(tag = "op", rename_all = "snake_case")]
pub enum SchemaChange {
    CreateTable   { table: String, description: String, columns: Vec<Column> },
    AddColumn     { table: String, column: Column },
    AddIndex      { table: String, columns: Vec<String>, unique: bool },
    AddForeignKey { table: String, column: String, references: String },
    RenameTable   { from: String, to: String },
    RenameColumn  { table: String, from: String, to: String },
    DropColumn    { table: String, column: String },
    DropTable     { table: String },
}

pub struct ChangeSet {
    pub name: String,      // snake_case; becomes the migration folder suffix
    pub summary: String,   // one or two plain-language sentences
    pub changes: Vec<SchemaChange>,
}
```

- `SchemaChange::is_destructive()` returns true for `DropColumn` and `DropTable`. `ChangeSet::is_destructive()` returns true if any of its changes is destructive.

### Task 4: Project and events (`src/project.rs`, `src/event.rs`)

```rust
pub struct ProjectConfig { pub spec_version: u32, pub name: String }

pub struct Event { pub seq: u64, pub at: jiff::Timestamp, pub actor: Actor, pub kind: EventKind }

#[serde(tag = "type", rename_all = "snake_case")]
pub enum Actor { Developer, System }        // SchemaAgent { run_id } and User come in epic 004

#[serde(tag = "type", rename_all = "snake_case")]
pub enum EventKind {
    ProjectCreated    { spec_version: u32, name: String },
    ChangeSetProposed { change_set: ChangeSet },
    ChangeSetRejected { proposal_seq: u64, findings: Vec<Finding> },
    ChangeSetAccepted { proposal_seq: u64, migration: String },  // e.g. "0002_add_family_member"
}

pub struct Finding { pub code: String, pub message: String }   // produced by the checker in 003
```

- `kind` is a nested object, not `#[serde(flatten)]`, so `deny_unknown_fields` and tagging behave predictably.
- `seq` starts at 1 and increases by one per event. `proposal_seq` points at the `ChangeSetProposed` event.

### Task 5: Example files for review

Create `fixtures/examples/` with hand-written files for the tester to read. Tests parse them, so they can't drift from the types:

- `schema.json`: a small finance schema (`accounts`, `people`, `categories`, `transactions` with foreign keys).
- `change-set.json`: the `ChangeSet` that would create it.
- `events.jsonl`: `project_created`, then a proposed change set that is rejected, then a proposed change set that is accepted.

### Task 6: Tests

- Round-trip tests (value → JSON → value) for every `SchemaChange` variant, `SchemaSpec`, `ChangeSet`, every `EventKind` and `ProjectConfig`.
- The `fixtures/examples/` files parse. Re-serializing `schema.json` with `serde_json::to_string_pretty` plus a trailing newline gives byte-identical output.
- Unknown fields are rejected, e.g. `"nulable": true` on a column.

## Out of scope

- `Question`, user prompts, agent actors, model config (epics 004 and 006).
- Any validation of names or references (checker, epic 003).
- `schemars::JsonSchema` derives (epic 004).

## Checkpoint

- [ ] `cargo build/test/clippy/fmt` pass.
- [ ] The tester reads `src/schema.rs`, `src/change.rs` and `fixtures/examples/` and confirms the vocabulary is enough for the three scenarios in `PRD.md` §7: personal finance, household inventory, small client and invoice log.
- [ ] Task Notes added below, and `Status: Done`.
