# Epic 006 notes

Review of the implementation and the checkpoint session, run by Claude on 2026-10-10. Lists only what the software could do better; the checkpoint itself went well.

Gate: `cargo build`, `test`, `clippy -D warnings` and `fmt --check` all pass.

## Runs

Model: `neuralwatt/glm-5.3-flash` (from `project-base.json`). Projects are in `.projects/`.

| Project | Path | Result |
|---|---|---|
| `rt-finance-dev` | fixture change sets 01, sample rows, 02, 03 (table rebuild) | pass |
| `rt-farm-income-projection` | tester's own farm prompts | pass (follow-up needed 3 calls) |
| `rt-invoices` | invoices initial + follow-up | pass |
| `rt-inventory` | inventory initial + follow-up | pass |
| `rt-farm` | farm initial + follow-up | pass |
| `rt-finance` | finance initial + follow-up | pass (initial 2 calls; follow-up 1 call, 1 `repaired_output`, no retries) |
| `cp-finance` | catch-up with `apply` | pass |

In every finished project: sample rows unchanged after the follow-up (compared with the backup), follow-up migrations only add, new details are empty on old rows, the dry run reports a copy of `data.sqlite` with rows and a passed `rows_kept`, one backup per migration on an existing database, `database_checked` passed, `foreign_key_check` clean, no `migration_failed`. The `03-rebuild` table rebuild kept every `transactions` row.

## Improvements

### Event log

- **A failed model call leaves no trace in `events.jsonl`.** `user_prompt` is appended before the call, so a provider error (`HttpError`, `429 concurrent_budget_exceeded`) leaves a lone `user_prompt`, and the retry adds a second identical one. Only `.agent-logs` says why. Seen in `rt-farm-income-projection` (seq 8 and 9). Proposal: append a `prompt_failed` event with the error and the agent log run ID, or append `user_prompt` only once the run produces a proposal.
- **Retryable provider errors are not retried.** A `429` marked `retryable: true` ends the command. A short backoff retry in `nocodo-agent` would hide transient limits from the tester (and later the user).

These two items move to epic 007 as Task 0.

### Schema agent

- **Assumptions promise things the system doesn't do.** These were accepted without comment:
  - Computed values: "the unpaid amount of an invoice is worked out by subtracting all its payments" (`rt-invoices`); "totals per category and what your brother still owes are worked out from these records" (`rt-finance`). Nothing computes them yet.
  - Starting entries: "the three kinds the user mentioned are added as the first kinds" (`rt-farm-income-projection`). `cost_categories` is created empty; nothing adds entries.
  - Pre-filled people: "The family members such as you, your wife, your son and your mother are entered into the existing list of people" (`rt-finance` follow-up). Nothing adds them.
  - Proposal: tighten the prompt (assumptions describe what is stored, not what will be calculated or pre-filled), and/or a checker advice rule. Same family as the epic 005 carry-over about storage wording in assumptions.
- **First instincts on a follow-up can be off.** In `rt-farm-income-projection`, "also track medical, supplement or misc costs per batch" first gave an empty change set ("already covered"), then one that dropped `costs.description` to replace it with a category. Both were rejected by the checker (`empty_change_set`, `destructive_not_confirmed`), and the third call was additive. The checker did its job; the prompt could say that a follow-up asking for more should add, never replace.
- **Stringified tool arguments on every call.** The epic 005 repair fired on every `glm-5.3-flash` call in `rt-farm` and `rt-invoices` (`repaired_outputs: 1` each). It works, but for this model it is the normal path, not a fallback.
- **New lists that other details point at start empty.** `currencies` (`rt-invoices`), `items` (`rt-farm`) and `cost_categories` have no entries, so the new optional choice can't be used until entries exist. Expected without data entry; worth keeping in mind for the questions and data-entry epics.

### Runtime and checker

- **`rows_kept` detail repeats the total.** "Every existing list kept all its rows (21 across 7 lists compared, 21 rows in the copy)." The first number is the total in the copy, not the rows in compared lists. Suggest: "Every existing list kept all its rows: 21 rows across 7 lists."
  - Fixed: the detail now reads "Every existing list kept all its rows: N rows across M lists." and counts only the compared tables.
- **First apply reports "Rows in existing lists: 0 before, 0 after."** On a new database there are no existing lists; the sentence could say so.
  - Fixed: the first apply on a new database says "There were no existing lists yet." (older events stay as written; the log is append-only).
- **`inspect` builds its read-only URI unescaped** (`file:{path}?mode=ro` in `crates/nocodo-runtime/src/inspect.rs`). A project path with `?`, `#` or `%` would break it. `post_apply.rs` already has a `file_uri` helper that escapes them; reuse it.
  - Fixed: `inspect` escapes `?`, `#` and `%` with a small `read_only_uri` helper in the runtime (the runtime does not depend on the checker). New test `inspect_works_when_the_path_has_uri_characters`.

### Checkpoint wording

- **`__diesel_schema_migrations` stores the version prefix** (`0001`, `0002`), not the folder name. The epic's "lists exactly the folders in `migrations/`" should say "lists the version prefix of each folder".
  - Fixed: the epic's Structure bullet now says so.
