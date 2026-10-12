# Epic 008: Engine, API and GUI

**Status:** Not started
**Goal:** Make what nocodo already does visible and easy to try. The operations move into one library, `nocodo-engine`. Three thin front ends use it: `nocodo` (CLI, calls the engine directly), `nocodo-api` (HTTP server, calls the engine) and `nocodo-gui` (calls the API over HTTP only, so it works against a local or a remote server). The GUI is a single column: a project dropdown, a prompt box, the open questions, and the tables as a spreadsheet-like view.

**Prerequisite:** Epic 007 (closed; checkpoint review in `epics-notes/007-questions.md`).

---

## Context

- Today `api/src/main.rs` holds the clap commands, the `Proposer` and `Applier` implementations and all printing. Nothing else can reuse it.
- Trying a prompt means switching projects with `--project` on every command. There is no way to list projects, and no way to look at rows except `sqlite3`.
- `inspect` returns row counts and a summary, never rows.
- The GUI is built on akar (`~/Projects/akar`), following `~/Projects/protopie/crates/protopie-gui` (winit + wgpu, `ureq` client). akar already has `dropdown`, `textarea`, `data_grid`, `tabs`, `modal`, `toast`, `alert`.
- Replay, the findings carried from epic 007 and the results write-up were planned for this epic. They move to epic 009 (Task 9 updates `PRD.md`).

## Decisions

- **`nocodo-engine` is the only home of operations.** One function per operation, returning typed results (from `nocodo-types`). It owns the `Proposer` and `Applier` implementations. The CLI, the API and any later front end call it. Why: no logic duplicated between front ends.
- **Front ends only parse input and present results.** CLI: clap → engine → text. API: HTTP → engine → JSON. GUI: HTTP → JSON → widgets. No validation, ordering or wording in any of them.
- **`nocodo-gui` talks HTTP only,** to a server URL (default: spawn a local `nocodo-api`, as protopie does; `--server <url>` for a remote one). It depends on `nocodo-types` and not on the engine. Why: the server may live anywhere.
- **Binaries:** `nocodo` (was the `nocodo-api` subcommands), `nocodo-api` (server), `nocodo-gui`. Why: `nocodo` is the user-facing name.
- **The API runs slow work in the background.** `prompt`, `answer` (when it resumes) and `continue` return a run ID at once; progress is read from events. The engine stays synchronous in its contract: one call, one result. Background workers and the event stream belong to the API.
- **Per-project write lock, in the engine.** A lock file in the project folder is held by any operation that writes (`prompt`, `changes`, `answer`, `continue`, `apply`, `sample-rows`). A second writer gets a plain "this project is busy" error, from the CLI and the API alike. Reads never take it. A lock left by a dead process is detected and ignored.
- **Projects root.** The API lists projects under one configured root (`--projects-dir`, default `.projects/`): each folder with a `project.json`. The CLI keeps `--project <dir>`.
- **Row reading is read-only, paginated and lives in `nocodo-runtime`.** Why: the runtime owns `data.sqlite` access; the engine only calls it.
- **Auth, TLS and multi-user access are out.** The API binds to localhost by default. A remote server is the tester's own setup for now.

## Tasks

### Task 1: Extract the engine (`nocodo-engine`, `api` → `cli`)

- Add `crates/nocodo-engine`. Move into it the `Proposer`/`Applier` implementations and the logic behind each command: `init`, `changes`, `prompt`, `questions`, `answer`, `continue`, `apply`, `inspect`, `sample_rows`.
- Operations return typed outcomes; printing stays out of the engine. Move any outcome type the front ends share into `nocodo-types`.
- Add the `nocodo` binary (`cli/`) with today's subcommands and output, unchanged.
- Existing tests pass without changes to what they assert.

### Task 2: Write lock (`nocodo-engine`)

- Lock file in the project folder, taken by every writing operation and released on drop.
- Stale lock detection (owner process gone). The busy message says what is running and what to do.
- Tests: a second writer is refused; reads work meanwhile; a stale lock is taken over.

### Task 3: Project list and row reader (`nocodo-engine`, `nocodo-runtime`)

- `list_projects(root)`: name, path, number of tables, last event time. Folders without a `project.json` are skipped; a broken one is listed with an error, not hidden.
- Runtime `read_rows(project, table, offset, limit)`: column names from `schema.json` (in plain words via the shared helpers), rows as typed values, total count. Read-only connection. Unknown tables are refused.
- Tests: empty project, project with sample rows, paging, unknown table.

### Task 4: The API server (`nocodo-api`)

- Axum, async, localhost by default. Routes call the engine and nothing else:
  - `GET /projects`, `POST /projects` (init)
  - `GET /projects/{p}/summary` (tables, counts, plain-language summary)
  - `GET /projects/{p}/tables/{t}/rows?offset=&limit=`
  - `POST /projects/{p}/prompt`, `POST /projects/{p}/continue`, `POST /projects/{p}/apply`, `POST /projects/{p}/sample-rows`
  - `GET /projects/{p}/questions`, `POST /projects/{p}/answers` (`{"q1": 2}` or `{"defaults": true}`)
  - `GET /projects/{p}/events?after=<n>` (events from `events.jsonl`, for polling)
- Slow calls return `202` with a run ID and run on a background worker, one at a time per project; a second one gets `409` busy. A run's end shows up as an event.
- Errors are JSON with a plain-language message.

### Task 5: API tests

- In-process server on a random port, scripted `Proposer` (no live LLM).
- Flows: init → list → prompt → questions → answer → rows; destructive confirmation; busy `409`; unknown project and table.

### Task 6: GUI shell (`nocodo-gui`)

- New crate on akar, modelled on protopie-gui: window, server process (spawned unless `--server`), `ureq` client in its own thread so the UI never blocks.
- Project dropdown filled from `GET /projects`, with "New project…" at the end (asks for a name, then `POST /projects`). The choice is remembered between runs.
- Prompt box and Send. A status strip shows the latest events in plain words, updated by polling. Errors show as an alert with the next step.

### Task 7: GUI questions and data view

- Open questions appear under the prompt box as numbered choices with the default marked; blocking ones first. "Use defaults" and "Answer" send `POST /answers`. A destructive confirmation shows as a modal.
- Retry button when the last run failed (`continue`).
- Below: the plain-language summary, tabs per table, a `data_grid` with the rows (paged). It reloads after every applied change. A "Add sample rows" button calls `sample-rows`.

### Task 8: GUI check by an agent

- A scripted run against the API with curl: the same flows a person clicks through, so an agent can verify the server side of each GUI action without a window. Record the commands in the epic notes.

### Task 9: Docs

- `AGENTS.md`: new crate and dependency table (`nocodo-engine`; `cli`, `api`, `gui` rows), binaries, commands and the manual test loop in terms of `nocodo` and the GUI.
- `PRD.md`: components table and epic table (008 as above, 009 for replay, carried findings and the write-up).
- `.env.example`/README only if the new binaries need it. Sweep `nocodo-api -- <command>` mentions.

## Not in this epic

- Replay with `rig-cassette`, the findings carried from epic 007, and the results write-up (epic 009).
- Auth, TLS, multiple users, hosting the server for anyone but the tester.
- Editing rows in the GUI, filters, sorting, charts.
- Streaming (SSE/WebSocket) events; polling is enough.
- New agent roles, new Schema agent behaviour.

## Checkpoint

Manual session:

1. `cargo build --workspace`; `cargo run --bin nocodo -- init --project .projects/g-finance`.
2. Start the GUI (`cargo run --bin nocodo-gui`). Pick the project in the dropdown, send a plain prompt from `fixtures/prompts/finance/plain-01.md`, answer the questions, and see the tables appear.
3. Add sample rows, send a follow-up, then the destructive follow-up and confirm it in the modal.
4. While a prompt is running, try a write from the CLI: it must be refused as busy. Run a CLI read (`nocodo inspect`) and see it work.
5. Start `nocodo-api` separately and point the GUI at it with `--server`.
6. Review: what is clear in the GUI, what is still confusing, which CLI features were missed. Findings go in `epics-notes/008-engine-api-gui.md`.

## Task Notes

### Task 1: Extract the engine

- **Built.** `crates/nocodo-engine` holds the `Proposer`/`Applier` implementations and one function per operation: `init`, `changes`, `questions`, `prompt`, `answer`, `continue_run`, `apply`, `inspect`, `sample_rows`. Each takes the project folder, is synchronous (it builds its own tokio runtime for model calls), prints nothing and returns a typed result or `EngineError`. An accepted change set or prompt run is applied inside the call; the result carries the `ApplyOutcome`.
- **Types moved.** `Outcome`, `PromptOutcome`, `AnswerOutcome`, `ContinueOutcome` and `ApplyOutcome` moved from `nocodo-orchestrator` to `nocodo_types::outcome` (the orchestrator re-exports them, so its tests are unchanged). New there: `InitOutcome`, `ChangesRun`, `PromptRun`, `ContinueRun`, `AnswerRun`, `InspectOutcome`, `SampleRowsOutcome`.
- **Binaries.** `cli/` is now the `nocodo` binary with today's subcommands (clap, `.env` loading, printing); it depends on `nocodo-types` and `nocodo-engine` only. `api/` is a skeleton again (`nocodo-api`, depends on the engine) until Task 4. The `agent-pm` workspace dependency entry was dropped, as nothing depends on it now.
- **Tests.** `api/tests/e2e.rs` moved to `cli/tests/e2e.rs`; only the binary name changed (`CARGO_BIN_EXE_nocodo`). Build, test, clippy `-D warnings` and fmt all pass.
- **Left as is, for Task 9.** Messages naming `nocodo-api <command>` (orchestrator errors, the CLI's "Answer with ..." line, `.env.example`) are unchanged because tests assert them; they now point at a command that is `nocodo`. Behaviour change: `answer` prints "Answers recorded. Continuing." after the resumed run finishes, not before the model call, since the engine returns one result per call.

### Task 2: Write lock

- **Built.** `crates/nocodo-engine/src/lock.rs`: `WriteLock::acquire(dir, operation)` takes an OS file lock (`File::try_lock`, std, no `unsafe`, no new dependency) on `<project>/.lock` and releases it on drop. `changes`, `prompt`, `answer`, `continue_run`, `apply` and `sample_rows` take it for the whole call; `init`, `questions` and `inspect` do not. Internal steps (the resume inside `answer`, the apply inside `prompt`) run under the one lock and never re-acquire it.
- **Stale locks.** The OS drops the lock when the owning process dies, so a leftover `.lock` file never blocks anyone and no PID probing is needed. The file's text (operation, process ID, start time) is rewritten by each new holder and cleared on release.
- **Busy error.** New `EngineError::Busy { running, pid, since_secs }`, shown as: "This project is busy: `prompt` is running (process 123), started 12 seconds ago. Wait for it to finish and try again. Reading (inspect, questions) still works." Task 4 can map it to `409`.
- **Tests.** Lock unit tests: second writer refused with the message naming the running operation; released on drop; stale lock file taken over. Engine test: `sample_rows`, `apply` and `continue_run` refused while held, `questions` and `inspect` work. Build, test, clippy `-D warnings` and fmt pass.
- **Open (fixed in Task 3).** A writer on a nonexistent folder failed with "opening .../.lock"; it now gets the project-not-found message. The `.lock` file is not in the project's `.gitignore` handling since `.projects/` is already ignored.

### Task 3: Project list and row reader

- **Built.** Engine `list_projects(root)` returns `ProjectInfo` (name, path, table count, last event time, error) for each subfolder with a `project.json`, sorted by folder name. Other folders and files are skipped; a project that cannot be read is listed under its folder name with the reason in `error`; a missing root gives an empty list (the default `.projects/` may not exist yet). Engine `read_rows(dir, table, offset, limit)` opens the project, takes the schema and calls the runtime; it never takes the write lock.
- **Runtime `read_rows(project_dir, spec, table, offset, limit)`.** Opens `data.sqlite` read-only (the same `mode=ro` URI as `inspect`), orders by `id`, caps `limit` at 1000 (`MAX_PAGE`). It takes the spec as an argument, like `add_sample_rows`, so the runtime still reads no specs. A table not in the schema is refused with a message listing the lists that exist. No `data.sqlite` yet: refused with "run `apply`".
- **Types.** `ProjectInfo`, `CellValue` (null, bool, integer, real, text; serialises as the bare JSON value), `RowColumn` (name, plain-word label, type) and `RowPage` (columns, rows, total, offset, limit) are in `nocodo_types::outcome`. The columns are the implicit ones first, then the schema's, so a grid can show them as is. Values are read as text and typed from the schema (money and integers as integers, decimals as reals, booleans as bool, dates as text), so a stray value that does not parse is returned as text instead of failing.
- **Task 2 follow-up.** `WriteLock::acquire` now checks the folder first, so a writer on a missing project says it is not a nocodo project (run init first) and creates nothing.
- **Tests.** Runtime: empty table (columns and labels, no rows), sample rows typed and paged (first, middle, last, beyond the end, NULL value, JSON form), unknown table refused. Engine: list (skip, order, broken project, missing root), read_rows on a project with no tables and on a missing folder. Lock: missing folder. Build, test, clippy `-D warnings` and fmt pass.
- **Open.** `last_event` is a timestamp, not text; Task 4 decides the JSON shape (the type already serialises). The engine has no read_rows test over a project with applied sample rows; the runtime tests cover that path and the engine function only adds the schema lookup.

### Task 4: The API server

- **Built.** `api/` is now a library (`nocodo_api`: `router`, `serve`, `AppState`) plus the `nocodo-api` binary (`--addr`, default `127.0.0.1:8787`; `--projects-dir`, default `.projects`; loads `.env`). Axum 0.8, handlers call the engine and nothing else. Routes are the ones in the task, plus `GET /projects/{p}/runs/{id}` (not in the plan: the in-memory status of a background run, so a failure such as a busy project or a model error that left no event is not lost). A project is addressed by its folder name under the projects folder. `GET /projects` returns `{projects:[{folder,name,path,tables,last_event,error}]}`, `/events?after=n` returns `{events,last}`, `/questions` returns `{questions}`, rows return the `RowPage` as is.
- **Background runs.** `prompt`, `continue`, `apply`, `sample-rows` and `answers` return `202 {run_id, operation, status, next}` and run on a plain `std::thread` (the engine builds its own tokio runtime, which cannot nest inside a server thread). One run per project: a second gets `409` from an in-memory flag, and a write held by another process (the CLI) is refused at request time with `409` by trying the engine's `WriteLock` first. `answers` always returns `202`, also when the round stays open, because the engine decides that inside one locked call; a bad answer (unknown question, bad choice) therefore shows in the run record, not as a `400`. Body shape errors are `400` at once.
- **Errors.** `{"error": "<message>"}`, with the engine's own wording. `404` unknown project or list, `409` busy / questions waiting / folder exists, `400` malformed input or `InvalidAnswer`, `500` otherwise.
- **Engine changes (needed for injection).** New `prompt_with`, `answer_with`, `continue_with` take a `FnOnce(&Project) -> impl Proposer`; `prompt`, `answer`, `continue_run` call them with `schema_proposer`. The engine re-exports `Project`, `Proposer`, `OrchestratorError`, `SchemaProposer`, `schema_proposer`. New `EngineError::NotFound` for an unknown list in `read_rows`.
- **Tests injection (for Task 5).** `AppState::new(projects_dir, |project| scripted_proposer)`, then bind `127.0.0.1:0`, read `listener.local_addr()` and `tokio::spawn(serve(listener, state))`.
- **Tested.** Manual curl smoke: init, list, summary, questions, events, unknown project (404), unknown list (404), bad answer (400), `apply` (202, then run finished), `prompt` (202). No automated API tests yet (Task 5). Build, test, clippy `-D warnings` and fmt pass.
- **Open.** `limit` is capped at 1000 by the runtime without telling the caller. Run records live in memory only. Project name check rejects slashes and leading dots only.

### Task 5: API tests

- **Built.** `api/tests/api.rs`: the real router on `127.0.0.1:0` via `AppState::new` + `serve`, with a scripted `Proposer` (a queue of `Reply` values shared by every operation, plus a hold switch to keep a run in progress). Change sets come from `fixtures/change-sets/finance/`. Requests are plain HTTP/1.1 over a tokio `TcpStream` (`Connection: close`), so no HTTP client dependency; dev-dependencies added to `api`: `nocodo-orchestrator`, `tempfile`, tokio `io-util`/`time`.
- **Tests (7).** Full flow (init → list → prompt that asks → questions → bad answers `400` → answer resumes → tables in list and summary → events polling with `after` → empty rows → sample rows → paging); destructive confirmation (decline keeps the column and data, a second ask confirmed removes the column, rows kept); `confirm_destructive: true` skips the question; busy `409` for every slow route while a run is held, reads still `200`, writes work again after release; busy `409` while another holder has the `WriteLock` (the CLI case); a failed run (empty script) shows `failed` in the run record and a `prompt_failed` event, then `continue` succeeds; unknown project, run, table (message lists existing lists), empty or malformed bodies, unknown page.
- **Surprising.** Body shape is checked before project existence for routes with a bad body, but `404` comes first for a well-formed body; the tests use well-formed bodies per route. Sample-row counts are exact per table (`per_table`).
- **Not covered.** Real model path (`schema_proposer`) and the binary's flags; no live LLM calls. Build, test, clippy `-D warnings` and fmt pass.

### Task 6: GUI shell

- **Built.** `gui/` is the `nocodo-gui` crate (workspace member `gui`), on akar (git dependencies on `https://github.com/brainless/akar` for `akar-core`, `-layout`, `-components`, `-winit`; `wgpu 29`, `winit 0.30`, `glyphon 0.11`, as protopie-gui). It depends on `nocodo-types` and talks to the API with `ureq` 2 only. `--server <url>` (or `NOCODO_SERVER`) uses a running server; otherwise it starts `nocodo-api` from the GUI's own folder (`NOCODO_API_BIN` overrides) with `--addr 127.0.0.1:0 --projects-dir <dir>` (`--projects-dir`, default `.projects`) and kills it on exit. Window: project dropdown (usable projects, broken ones greyed with "cannot be opened", "New project..." last, which opens a name modal and then `POST /projects`), alert for errors (dismissable), prompt box with Send, and a status strip with the latest event lines. Send is disabled as "Working" while a run is followed.
- **Structure.** `client.rs` (blocking typed calls plus a worker-thread wrapper with a channel drained once per frame), `model.rs` (all state and poll timing, no drawing), `words.rs` (event to plain sentence), `settings.rs` (remembered project, `~/.config/nocodo/gui.json` or `NOCODO_GUI_SETTINGS`), `server.rs`, `view.rs` (drawing), `main.rs` (window, as protopie).
- **Polling.** Events every 3 s, every 0.8 s while a run is followed; the run record is polled alongside. First load fills the strip without alerts; later `prompt_failed`/`migration_failed` events raise an alert; a failed run shows the server's message. The project list reloads when a run ends or lists change.
- **API change (small, outside the GUI).** `nocodo-api` printed the requested address; with `--addr ...:0` it now prints the real one (`listener.local_addr()`), which is how the GUI learns the port.
- **Tested.** Unit tests (21): client against a fake HTTP server (list, prompt POST body, server error text such as busy passed through, events and run status, unreachable server message, path encoding), event-to-words, remembered choice (file round trip, broken file; selected on load only when present and usable), poll/run-follow logic, startup-line parsing. Build, test, clippy `-D warnings` and fmt pass. No Cranelift override needed: akar, wgpu and winit build and link under it. Started the GUI once by hand: the window opens and the spawned server runs.
- **Not covered / open.** Drawing and clicks are not tested (no headless window). If the GUI is killed with a signal (not closed), the spawned server keeps running (protopie's `--exit-on-stdin-close` is not copied). While the project popup is open the content below is not drawn (akar draws text above quads). Questions, the data view and Retry are Task 7.

### Task 7: GUI questions and data view

- **Built.** Under the status strip (now the latest 3 lines, one line each) the window shows: open questions as numbered choices (blocking first, "Needs your answer:" prefix, the default marked "(suggested)", a picked choice filled), "Use defaults" and "Answer" (sends the picks as `{"q1": 2}`); a Retry button beside the strip title when the last run or model call failed (`POST /continue`); the plain-language summary; a `tab_bar` with a tab per list; an akar `data_grid` with the rows, paged by 50 with Previous/Next and "Rows 1-50 of 120"; "Add sample rows" (`POST /sample-rows`). The removal confirmation (a question asked by `system`) is a modal with one button per choice; closing it does nothing, so a choice is the only way out. The modal and the question list hide while a request is in flight.
- **Structure.** `model.rs` holds all new state and the polling (questions, summary, rows fetched when dirty, one request of each kind at a time). `client.rs` gained `questions`, `summary`, `rows` and a generic `start_run`. New `grid.rs`: cell text (money in minor units shown as `12.50`, booleans Yes/No, NULL empty) and column widths. `view.rs` draws; its only logic is wording helpers.
- **Reloads.** The summary and the open page reload on a new `migration_applied` or `sample_rows_added` event (not for history on first load), when a run ends, and on selecting a project. The chosen tab is kept, and falls back to the first list if it disappears. A page past the end after rows were removed moves to the last page. Questions refetch on every new event.
- **Tested.** 15 new unit tests (36 in the crate): client routes and bodies against the fake server, question order and picks, answer bodies, removal confirmation, double-send guard, reload after an applied change (and not for history), run end, tab switch with a late reply ignored, paging and the ends, shrinking list, vanishing tab, retry flag, cell text, column widths, wording helpers. Build, test, clippy `-D warnings` and fmt pass. Started the GUI by hand for 15 s on a project with sample rows: it drew every frame without a panic. Clicks and layout were not checked by a person (no headless window).
- **Open.** Many lists will overflow the tab bar (not handled). The summary is cut at 3 lines; the questions panel scrolls but the page does not. The grid is not editable or sortable (out of scope). Existing model tests needed one change: `polling_follows_a_run_until_it_ends` clears the new initial data requests first; what it asserts is unchanged. A spawned `nocodo-api` outlives a GUI killed by signal (from Task 6).

### Task 8: GUI check by an agent

- **Built.** `scripts/gui-check.sh`: starts `nocodo-api` on a free port with a temporary projects folder, replays the server side of each GUI action with curl and jq (new project, list, empty and filled data view, events polling, sample rows with run polling, paging, apply, removal confirmation, prompt and Retry), prints ok/FAIL per step, exits non-zero on failure and kills the server on exit. Commands, the action-to-call table and the observed run are in `epics-notes/008-engine-api-gui.md`.
- **Tested.** Ran it: 34 passed, 0 failed. Build, test, clippy `-D warnings` and fmt pass.
- **Open.** The API has no `changes` route, so the schema is set up with the `nocodo` CLI. Steps needing a model key are marked and skipped without `NOCODO_CHECK_LIVE=1`; the live path was not run (no key). Without a key a prompt run stays `running`, so Retry showed `409`. Findings (bad-answer wording, `nocodo-api` in messages, missing-key hang) are in the notes.


### Task 9: Docs

- **Built.** `AGENTS.md`: intro and epic status line, manual test loop in terms of `nocodo` and the GUI (busy refusal, `scripts/gui-check.sh`), repository layout, crate table (`nocodo-engine`, `api`, `cli`, `gui` rows), dependency direction, local source table (GitHub URLs for Rig, Diesel, akar, protopie; the GUI uses akar as git dependencies), the "what not to do" line. `PRD.md`: components table (Engine, CLI, API, GUI), binaries, project layout (`.lock`), determinism table, epic table (008 built with checkpoint pending; 009 for replay, carried findings and the write-up). `.env.example` now says `nocodo` and `nocodo-api` load it. Past epic files and notes keep their `nocodo-api <command>` text, as history.
- **Message sweep.** `nocodo-api continue|apply|answer|questions` in orchestrator errors, the `prompt_failed` detail, the `PromptFailed` doc comment and the CLI's "Answer with ..." line now say `nocodo ...`. Two test assertions changed with them (`orchestrator.rs` expected string, `cli/tests/e2e.rs` "nocodo answer"); what they check is unchanged. Two doc comments saying "the api adapts" now say the engine does.
- **Not done, by design.** Status stays "Not started": the checkpoint is a manual session. Findings from Task 8 (bad-answer wording, missing-key hang) are listed under epic 009 in `PRD.md`.
