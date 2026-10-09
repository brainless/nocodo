# Epic 006: Runtime

**Status:** Not started
**Goal:** Accepted migrations reach a real `data.sqlite`. Before every migration the database is backed up into `backups/`. Every proposal is dry-run on a copy of the real data, so a follow-up is checked against real rows. A failed migration leaves `data.sqlite` exactly as it was. `inspect` shows what is stored and gives a plain-language summary of what the tool remembers. This epic tests PRD success criterion 6: a follow-up produces an additive migration, and existing data survives.

**Prerequisite:** Epic 005 (closed; checkpoint review in `epics-notes/005-schema-agent-hardening.md`).

---

## Context

- The PRD roadmap planned the runtime as epic 005, but 005 became Schema agent hardening, so the runtime has never been built. `nocodo-runtime` is an empty skeleton. No project has a `data.sqlite`, and every `change_set_accepted` event so far says "Ran N earlier migrations on an empty database (no data.sqlite yet)".
- The checker's dry run already copies `<project>/data.sqlite` when it exists (`nocodo-checker/src/dry_run.rs`), and the orchestrator already passes that path. Two things are wrong once the file exists:
  - The `earlier_migrations_applied` detail counts every migration folder, but Diesel only runs the ones not yet recorded in the copy. The detail would claim migrations ran that didn't.
  - Nothing checks that existing rows survive the new migration. `db_matches_spec_*` compares structure only.
- Migrations have `run_in_transaction = false` (`METADATA_TOML`), because the SQL turns `foreign_keys` off and manages its own `BEGIN`/`COMMIT`. Diesel's `run_migration` then records the version in `__diesel_schema_migrations` in a separate statement after the SQL has committed (`~/Projects/diesel/diesel_migrations/src/migration_harness.rs`). So the SQL's own transaction is not enough to guarantee "untouched on failure"; the backup is.
- The dependency direction holds: the orchestrator must not depend on the runtime. The orchestrator already solves this for the agent with the `Proposer` trait that `api` implements. Apply uses the same pattern.
- The tester must never write rows by hand, and the agent's table and column names change from run to run, so a hand-written row fixture can't fit an agent-built schema. Rows come from a small deterministic generator instead (Task 5).

## Decisions

- **Apply is automatic after an accepted change set.** `nocodo-api changes` and `prompt` apply the new migration right after it is accepted. A separate `nocodo-api apply` applies whatever is pending, for retrying after a failure and for catching up projects created before this epic. Why: the PRD scenario applies as part of one flow, and keeping `data.sqlite` in step with `schema.json` means every later dry run checks against the real current data.
- **The orchestrator records, the runtime applies.** The orchestrator defines a `trait Applier`; `api` implements it with `nocodo-runtime`. The orchestrator appends the events and runs the checker's post-apply comparison. Why: the orchestrator owns `events.jsonl` and depends on the checker, while the runtime depends only on `nocodo-types`.
- **One backup per migration, and every failure restores it.** Each pending migration is applied on its own: back up, run, check. If anything fails, the connection is closed and the backup is copied back over `data.sqlite`. Why: with `run_in_transaction = false`, a failure can leave the SQL committed but the version unrecorded. Copying the file back is the only state we can trust.
- **Rows for testing come from a deterministic sample generator**, not a data-entry feature. `nocodo-api sample-rows` fills every table with a few obviously fake rows (`sample 1`, `sample 2`, …) derived only from `schema.json`. It is a test aid for the checkpoint, kept narrow on purpose: no values from the user, no editing, no deleting.

## Tasks

### Task 1: Types (`nocodo-types`)

- Apply report, shared by the runtime (which produces it) and the orchestrator (which records it):
  - `AppliedMigration { migration: String, backup: Option<String>, rows_before: BTreeMap<String, u64>, rows_after: BTreeMap<String, u64> }`. `backup` is the path relative to the project (e.g. `backups/before_0002_add_family_member.sqlite`), or `None` when `data.sqlite` did not exist yet.
  - `FailedMigration { migration: String, backup: Option<String>, error: String, restored: bool }`. `error` is the SQLite or I/O error, verbatim.
  - `ApplyReport { applied: Vec<AppliedMigration>, failed: Option<FailedMigration> }`. Migrations after a failure are not attempted.
- New `EventKind` variants. Each carries a `detail` sentence a reviewer can read on its own:
  - `MigrationApplied { migration, backup, rows_before, rows_after, detail }`, e.g. "Applied 0002_add_family_member to data.sqlite after backing it up to backups/before_0002_add_family_member.sqlite. Rows in existing lists: 12 before, 12 after."
  - `MigrationFailed { migration, backup, error, restored, detail }`, e.g. "0003_clarify_dates_and_transfers failed: <error>. data.sqlite was restored from backups/before_0003_clarify_dates_and_transfers.sqlite and is exactly as it was before. Later migrations were not attempted; `nocodo-api apply` retries."
  - `DatabaseChecked { check: CheckStep }`: the comparison after an apply run (Task 3).
  - `SampleRowsAdded { rows: BTreeMap<String, u64>, detail }`.
- Apply events have actor `System`; `SampleRowsAdded` has actor `Developer`. Old event logs load unchanged.
- Add `rows_kept` and `db_matches_spec_after_apply` to the documented `CheckStep` codes.

### Task 2: Apply with backups (`nocodo-runtime`)

- Dependencies: `diesel` and `diesel_migrations` (SQLite, same versions and features as the checker), `libsqlite3-sys` bundled.
- `apply_pending(project_dir: &Path) -> Result<ApplyReport, RuntimeError>`:
  1. Open (or create) `data.sqlite` and read `migrations/` with `FileBasedMigrations::from_path`. Take `pending_migrations`, in order.
  2. For each pending migration:
     - If `data.sqlite` already existed, back it up with `VACUUM INTO 'backups/before_<migration>.sqlite'`, creating `backups/` if needed. A backup is never overwritten: if the name is taken (a retry), use `before_<migration>-2.sqlite`, and so on.
     - Count rows in every user table (`rows_before`), run the migration with `run_migration`, and count again (`rows_after`).
     - On error: drop the connection, copy the backup over `data.sqlite` and remove any `-journal`/`-wal` file. With no backup (the first migration of a new database), delete `data.sqlite`. Set `restored`, stop, and return the report with `failed`.
  3. Return the report. With nothing pending, the report is empty and no file is touched.
- Row counts ignore `sqlite_*` and `__diesel_schema_migrations`.
- `RuntimeError` covers problems that stop the run before any migration starts (unreadable `migrations/`, can't create `backups/`). Errors once a migration has started go in `FailedMigration`.

### Task 3: Record the apply, check the result (`nocodo-orchestrator`, `nocodo-checker`)

- Orchestrator: `trait Applier { fn apply_pending(&self, project_dir: &Path) -> Result<ApplyReport, ApplierError>; }`, mirroring `Proposer`.
- `Project::apply_pending(&applier) -> Result<ApplyOutcome>`:
  - appends one `MigrationApplied` per applied migration, then `MigrationFailed` if one failed;
  - when at least one migration was applied and none failed, runs the checker's post-apply comparison against `current_schema()` and appends `DatabaseChecked`;
  - appends nothing when nothing was pending.
- Checker: `pub fn check_db_matches_spec(db: &Path, spec: &SchemaSpec) -> CheckStep`, with code `db_matches_spec_after_apply`. It opens the real database read-only, reuses `introspect`/`diff`, and runs `PRAGMA foreign_key_check`. Detail example: "data.sqlite matches the current schema, and every row that points at another row points at one that exists."
- If that check fails, the event records it and the command exits with an error. Nothing is restored: the same comparison passed on a copy moments earlier, so a mismatch is a bug to investigate, and the backups are in place.

### Task 4: Dry run on the real data (`nocodo-checker`)

- `earlier_migrations_applied` reports what really ran: how many earlier migrations were already recorded in the `data.sqlite` copy, how many pending ones ran on it, and how many rows the copy holds. Example: "On a copy of data.sqlite (12 rows in 4 lists): 1 earlier migration was already applied; ran 0 pending." With no `data.sqlite` the detail is unchanged.
- New step `rows_kept`, right after `up`: every table that is in both the current and the new schema has the same number of rows after `up` as before. A table renamed by the change set is compared under its new name. Dropped tables are not compared (destructive changes already need confirmation). Finding code `rows_lost`, naming each table with its counts before and after.
- The real `data.sqlite` is still never opened for writing by the dry run.

### Task 5: Sample rows (`nocodo-runtime`, `api`)

- `add_sample_rows(project_dir: &Path, spec: &SchemaSpec, per_table: u32) -> Result<BTreeMap<String, u64>, RuntimeError>`:
  - refuses if a migration is pending or the schema has no tables;
  - fills tables in an order that respects references (a referenced table first); a nullable reference that can't be filled yet is left empty;
  - row `n` (1-based, counting from the rows the table already has) gets: `text` → `"sample n"` (`"<column> n"` when the column is `unique`), `integer`/`money` → `n`, `decimal` → `n + 0.5`, `boolean` → `n` is odd, `date` → `2026-01-DD` with day `n`, capped at 28, `date_time` → that date at `T12:00:00Z`;
  - a reference points at row `((n - 1) mod rows_in_target) + 1` of the target table;
  - all rows go in one transaction, and `PRAGMA foreign_key_check` must be clean before commit.
- The same schema always gives the same rows. Values come from the generator, never from user input; quote them anyway.
- `nocodo-api sample-rows --project <dir> [--per-table 3]` calls it, appends `SampleRowsAdded` through `Project::append`, and prints the counts.
- Not a data-entry feature. Keep it to this one function and command.

### Task 6: Inspect and the plain-language summary (`nocodo-runtime`, `api`)

- `inspect(project_dir: &Path) -> Result<Inspection, RuntimeError>`: user tables with row counts, the applied and pending migration names, and the number of files in `backups/`. It works without a `data.sqlite` (every migration pending, no rows).
- `summary(spec: &SchemaSpec, rows: &BTreeMap<String, u64>) -> String`, a pure function. It lists each kind of thing the tool remembers, in schema order:
  - the name in words (`expense_categories` → "Expense categories") and how many entries are stored so far;
  - the table's description;
  - "Each one has:" followed by its details in words. A reference drops `_id` (`paid_by_person_id` → "paid by person"). Optional details are marked "(optional)".
  - It ends with one line covering the implicit columns: "Every entry also remembers when it was added and when it last changed."
  - No types, keys or column names, and none of the words "table", "column", "row", "SQL", "database", "schema", "ID" in the parts the function writes. Descriptions come from the Schema agent and are shown as written.
- `nocodo-api inspect --project <dir>` prints two parts: for the reviewer, the table names with row counts, applied/pending migrations and backup count; for the user, the summary. It writes nothing: both parts are derived from `schema.json` and `data.sqlite`, so the reviewer can rerun it at any time.

### Task 7: Wire apply into the commands (`api`)

- `api` implements `Applier` with `nocodo_runtime::apply_pending`.
- `changes` and `prompt`: after `Outcome::Accepted`, call `Project::apply_pending` and print one line, e.g. `Applied to data.sqlite: 0002_add_family_member (backup: backups/before_0002_add_family_member.sqlite). Rows in existing lists: 12 before, 12 after.` On a failure, print the `MigrationFailed` detail and exit 1. A rejected change set applies nothing.
- New `nocodo-api apply --project <dir>`: applies everything pending, prints one line per migration, or "Nothing to apply: data.sqlite is up to date."
- The `api` stays thin: no SQL, no counting, no file copying.

### Task 8: Tests

No live LLM calls in `cargo test`.

- Runtime, apply:
  - A fresh project with `finance/01-initial.json` applied creates `data.sqlite` with no backup.
  - With sample rows, applying `02-family-member.json` and then `03-rebuild.json` (a table rebuild) makes one backup each. Each backup holds the rows from before its migration, and `rows_before == rows_after` for every existing table.
  - A test-written migration whose `up.sql` fails partway (valid statements, then an invalid one) is reported in `failed` with `restored: true`. `data.sqlite` has the same structure, rows and recorded versions as before; the backup is kept; a later pending migration is not attempted.
  - The same failure as the first migration of a new database leaves no `data.sqlite`.
  - A retry never overwrites a backup (`-2` suffix).
- Runtime, sample rows: two runs on the finance spec in fresh databases give identical contents. The foreign key check is clean, `unique` columns stay distinct, and the function refuses when a migration is pending.
- Runtime, summary: snapshot for the finance spec after `02-family-member.json`. A test asserts that the generated parts of the summary contain none of the banned words, for every fixture spec.
- Checker: a dry run on a `data.sqlite` with rows reports the copy and the already-applied count, and passes `rows_kept` for `02` and `03`. A test-written migration that deletes rows fails `rows_kept` with `rows_lost`. The real `data.sqlite` is byte-identical after the dry run. `check_db_matches_spec` passes on an applied database and fails after a test drops a column out of band.
- Orchestrator, with a fake `Applier`: applied migrations give `migration_applied` events and one `database_checked`; a failure gives `migration_failed` and no `database_checked`; nothing pending gives no events. Old event logs still load.
- End to end through the binary (`env!("CARGO_BIN_EXE_nocodo-api")` and `std::process::Command`, no new dependencies): `init`, `changes 01`, `sample-rows`, `changes 02`, `changes 03`, `inspect`. The `inspect` output shows the sample row counts unchanged.

## Not in this epic

- Questions to the user (epic 007).
- Axum, the `nocodo` CLI over HTTP, and `rig-cassette` replay (epic 008).
- Data entry, editing or import of real user data. `sample-rows` is the only way rows get in.
- Keeping `updated_at` current when a row is edited, and computing derived values such as balances.
- Commands to restore a backup, revert a migration or prune `backups/`.
- Destructive changes on top of rows (`fixtures/change-sets/finance/04-drop-notes.json`): asking the user to confirm them comes with questions, so the checkpoint doesn't use this fixture.
- Carried over from epic 005:
  - Checker advice for a redundant text column kept next to a new reference (farm `description` beside `item_kind_id`).
  - Checker advice or tighter rules for storage wording in assumptions ("short code", "kept as a list").
  - A live run of OpenAI strict mode (no key yet).
  - Seeing the repaired-value error path (missing `assumptions`) live.

## Open questions for the tester

All three were decided by the tester on 2026-10-09: the recommended default in each case.

1. **Automatic apply, or a separate step?** Recommended default: automatic after acceptance, plus `apply` for retries and old projects (see Decisions). The alternative, apply only on `nocodo-api apply`, lets `schema.json` run ahead of `data.sqlite` between commands.
2. **Sample rows, or a fixture rows file?** Recommended default: the generator (Task 5), because it fits any schema the agent produces and needs no hand-written files. A JSON rows file for `changes` fixtures would give more realistic values but only fits hand-written schemas.
3. **Agent-written descriptions in the summary.** Recommended default: show them as written, and have the reviewer note any technical wording at the checkpoint. The alternative, a summary built from names only, can't leak technical words but reads poorly.

## Checkpoint

Manual session. First, the deterministic path (no LLM), which includes a table rebuild on top of rows:

```bash
P=.projects/rt-finance-dev
cargo run -q --bin nocodo-api -- init --project $P
cargo run -q --bin nocodo-api -- changes --project $P fixtures/change-sets/finance/01-initial.json
cargo run -q --bin nocodo-api -- sample-rows --project $P
cargo run -q --bin nocodo-api -- inspect --project $P
cargo run -q --bin nocodo-api -- changes --project $P fixtures/change-sets/finance/02-family-member.json
cargo run -q --bin nocodo-api -- changes --project $P fixtures/change-sets/finance/03-rebuild.json
cargo run -q --bin nocodo-api -- inspect --project $P
```

Then the agent path, with the model in `project-base.json`, for each `<s>` in `finance inventory invoices farm`:

```bash
cargo run -q --bin nocodo-api -- init --project .projects/rt-<s>
cargo run -q --bin nocodo-api -- prompt --project .projects/rt-<s> --file fixtures/prompts/<s>/01-initial.md
cargo run -q --bin nocodo-api -- sample-rows --project .projects/rt-<s>
cargo run -q --bin nocodo-api -- prompt --project .projects/rt-<s> --file fixtures/prompts/<s>/02-follow-up.md
cargo run -q --bin nocodo-api -- inspect --project .projects/rt-<s>
```

Finally, catch up one epic 005 project that has no `data.sqlite`: `cargo run -q --bin nocodo-api -- apply --project .projects/cp-finance`.

- [ ] `cargo build/test/clippy/fmt` pass.
- [ ] The reviewer confirms, from the project folders alone:
  - **Data survives:** in every project, the sample rows are still there after the follow-up. `sqlite3 <project>/data.sqlite` shows the same row counts as `sample_rows_added`, and the `sample n` values are unchanged, including in `transactions` after the `03-rebuild` table rebuild.
  - **Additive follow-ups:** each agent follow-up's migration only adds; new details on existing lists are empty for the sample rows.
  - **Dry run on real data:** the follow-up's `change_set_accepted` checks say the dry run used a copy of `data.sqlite` with rows, and include a passed `rows_kept`.
  - **Backups:** `backups/` has one file per migration applied to an existing `data.sqlite`, named after it. Opening the latest one with `sqlite3` shows the data as it was before that migration.
  - **Events:** `events.jsonl` tells the story without the terminal: proposal, acceptance, `migration_applied` with backup path and row counts, `database_checked` passed, `sample_rows_added`.
  - **Structure:** `sqlite3 data.sqlite .schema` matches `specs/schema.json`, and `__diesel_schema_migrations` lists exactly the folders in `migrations/`.
  - **Summary:** each `inspect` summary reads as plain language: no technical words in the generated parts, and any in agent-written descriptions are noted.
  - **Catch-up:** `cp-finance` now has a `data.sqlite` matching its schema, with `migration_applied` events for both migrations (the second backs up the first's empty database).
  - **Failures:** if any migration failed during the session, its `migration_failed` event says `restored: true` and `data.sqlite` matches the backup. If none failed, say so; the Task 8 tests cover the path.
- [ ] Task Notes added below, and `Status: Done`.

---

## Task Notes

