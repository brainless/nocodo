# Epic 008 notes: Engine, API and GUI

Only Task 8 (GUI check by an agent) is recorded so far. The checkpoint review by a person (clarity of the GUI, confusing parts, missed CLI features) is still to be added here.

## Task 8: GUI check by an agent

Script: `scripts/gui-check.sh` (run from the repo root). It builds `nocodo` and `nocodo-api`, starts the server on a free port with a temporary projects folder, runs the steps below with curl and jq, prints `ok`/`FAIL` per step and exits non-zero on any failure. It kills the server and deletes the folder on exit.

```bash
scripts/gui-check.sh                    # no model key needed
NOCODO_CHECK_LIVE=1 scripts/gui-check.sh  # also sends a real prompt (needs a key in .env)
```

Each GUI action and the call behind it:

| GUI action | Server side checked |
|---|---|
| Project dropdown, "New project..." | `GET /projects` (empty, then one), `POST /projects` (201; same name 409; `../x` 400), unknown project 404 |
| Opening an empty project | `GET /summary`, `GET /questions` (none), `GET /events?after=0` (only `project_created`), rows of an unknown list 404 |
| Reload after a change is applied | `GET /summary` shows the new lists, `GET /projects` table count, `GET /events` has `migration_applied`, `?after=<last>` is empty, rows of an empty list (`total` 0), unknown list 404 with the existing lists named |
| "Add sample rows" | `POST /sample-rows` 202, `GET /runs/{id}` until `finished`, rows `total` 4, paging (`offset=2&limit=2` gives 2 rows, past the end gives 0) |
| Apply | `POST /apply` 202, run `finished` |
| Removal confirmation modal | After the destructive change set, `GET /questions` has the system question; `POST /answers {"q1":99}` is 202 but the run record shows `failed` ("no question null" - see below); a real choice finishes and the grid no longer has the removed column while the rows stay |
| Prompt box, Send | Empty text 400; with no key the request is 202 |
| Retry | `POST /continue` while a run is going is 409 with the busy message |

Not covered by the script:

- **[NEEDS MODEL KEY]** The prompt, the questions the Schema agent asks, and answering them: `NOCODO_CHECK_LIVE=1` only sends one prompt and prints the run record and the questions. This session had no key, so the live path was not run. Without a key the prompt run stays `running` for the whole script (the model call keeps retrying), so Retry showed the busy `409` instead of a `202`. Both are what a person would see.
- The API has no route for `changes` (hand-written change sets), so the script sets up the schema with the `nocodo` CLI (`changes fixtures/change-sets/finance/01..04`). The GUI never does that; it only reads and sends prompts.
- Busy `409` while another process (the CLI) holds the write lock: it needs a long-running writer, so only `api/tests/api.rs` covers it.
- Drawing, clicks and layout: no headless window.

### Observed run (2026-10-12)

`scripts/gui-check.sh`: 34 passed, 0 failed. Server on `127.0.0.1:<random>`. Selected output:

```
ok    new project
ok    same name again is refused
ok    events show applied change
ok    rows now present
ok    paging (Next)
ok    removal confirmation is an open question (modal)
      question: {"asked_by":"system","blocking":true,"choices":["Yes, forget it","No, keep it"],"default":null,"id":"q1","text":"This will forget the notes you have entered on each transaction (...). Continue?",...}
ok    bad answer shows as failed run
ok    removed column is gone from the grid, rows kept
ok    prompt accepted
      continue (Retry button): HTTP 409 {"error":"This project is busy: `prompt` (r5) is running. Wait for it to finish (watch GET /projects/g-finance/events) and try again."}
passed: 34  failed: 0
```

### Findings

- **Bad answer is a 202, not a 400.** `POST /answers {"q1":99}` returns 202 and the error ("There is no question null.") only shows in the run record. This is the known Task 4 behaviour (the engine decides inside one locked call), and the message says "question null" instead of naming the question that was sent. The GUI avoids it because it only sends valid picks. Worth a clearer message in the engine.
- **Prompt messages name `nocodo-api`.** A refused prompt says "Answer them with `nocodo-api answer`, or see them with `nocodo-api questions`" (seen while a question was open). The command is `nocodo` now; Task 9 sweeps these.
- **Without a key a prompt hangs as `running`** for a long time instead of failing fast with a message about the missing key. The GUI would show "Working" with no explanation. Worth a check for a missing key before the call.
- The summary route returns text only (`applied`, `pending`, `backups`, `summary`), no list of table names; the list names for tabs have to come from elsewhere (the rows route's unknown-list error lists them, which is not a good source). Not a problem for the script (finance names are fixed), noted for tab building.
