# Epic 005 notes

Review of the implementation and the checkpoint session, run by Claude on 2026-10-09. `epics/005-schema-agent-hardening.md` has since been closed (`Status: Done`, Task Notes added). The go/no-go below is the proposal the tester evaluated at the checkpoint.

Gate: `cargo build`, `test`, `clippy -D warnings` and `fmt --check` all pass.

## Runs

Base file: `claude-haiku-5-5` (Anthropic) unless noted. Non-strict (`strict: false`) in every run. Projects are in `.projects/`.

| Project | Prompts | Result |
|---|---|---|
| `cp-finance` | finance initial + follow-up | accepted, 1 call each |
| `cp-inventory` | inventory initial + follow-up | accepted; initial needed 2 calls (1 rejected output, recovered) |
| `cp-invoices` | invoices initial + follow-up | accepted, 1 call each |
| `cp-farm` | farm initial + follow-up | accepted, 1 call each |
| `travel-2`, `travel-3` | travel prompt | accepted, 1 call each |
| `cp-cofounder` | cofounder initial + follow-up | initial **failed** (3 calls, 3 rejected); follow-up then ran on an empty schema, so the project is not a valid test |
| `cp-cofounder-b` | same | initial **failed** (3 rejected); follow-up ran on an empty schema |
| `cp-cofounder-c` | same | accepted, 1 call each |
| `cp-finance-nw` | finance, `neuralwatt/glm-5.3-flash` | both prompts **failed** (3 rejected each) |
| `cp-fin-glm-5.3-flex` | same, `glm-5.3-flex` | both prompts **failed** |
| `cp-fin-deepseek-v4-flash` | same, `deepseek-v4-flash` | accepted, 1 call each |
| `cp-fin-kimi-k3` | same, `kimi-k3` | accepted, 1 call each |

`glm-5.3-flash` and `glm-5.3-flex` both exist in the Neuralwatt `/v1/models` list, so the unverified model name is fine.

## Findings

- **Stringified arguments are not fixed by retrying.** All 8 failed runs fail the same way: the model sends `change_set` (or `assumptions`) as a string holding JSON. The retry loop works as designed: every rejection has an `invalid_output` line, the next call continues the conversation, and the follow-up quotes the serde error (`invalid type: string "...", expected struct ChangeSet`). The model reads it. In the GLM run its reasoning says "the change_set was passed as a string instead of a JSON object", and it resends a string anyway. This is a limit of how those models emit tool calls. Retrying with feedback only helped once (inventory initial). Strict mode would prevent it on Anthropic, but it degrades Haiku (see below), so Haiku has no fix today.
  - **Proposal (needs the tester's decision, not in epic 005):** a deterministic repair in `run_extractor` before deserializing. If a field the schema expects as an object or list arrives as a JSON string, parse it. This keeps "LLMs emit typed values, deterministic code does the rest".
- **A failed initial prompt leaves the next prompt running on an empty schema.** `cp-cofounder` and `cp-cofounder-b` show this: the follow-up created a whole new schema. `cp-cofounder` and `cp-cofounder-b` should not be read as evidence about the cofounder criteria.
- **Facts, not results: pass.** Travel stores `expense_shares` in both runs, so who owes whom can be worked out. Nothing stores balances or debts in finance, invoices or travel. Assumptions say so explicitly.
- **Conventions: pass.** Every reference column follows the naming rule (including `paid_by_friend_id`, `added_by_person_id`, `family_member_id`). No proposal lists `id`, `created_at` or `updated_at`, and none adds a redundant "date recorded" column.
- **Real references: pass** on `cp-cofounder-c`: no `<x>_type` + `<x>_id` pairs; one `*_similarity_scores` table per list with `score:decimal` and two real references each. `quantity:decimal` and `tax_rate:decimal` also appear in invoices.
- **Data vs schema: pass.** The farm follow-up adds no schema change for the business account and says so in its summary. The migration name `add_business_account_and_item_quantities` still suggests a change was made.
- **Checks: pass.** Every `change_set_accepted` event lists 10 steps in the specified order, all passed, with plain-sentence details (for example "Ran 0 earlier migrations on an empty database (no data.sqlite yet)."). The `Checked on SQLite: up, down, up again; ...` line prints after each result.
- **Models and strict: pass.** `actor.model` is `anthropic:claude-haiku-5-5` on every proposal; `run_started` shows `strict: false`; `run_finished` shows `calls` and `invalid_outputs` (it now also records `repaired_outputs`, added after this review with the string-argument repair).
- **Churn: low.** `travel-2` and `travel-3` have the same six tables apart from naming (`trip_members` vs `trip_participants`, `expenses` vs `trip_expenses`, `description` vs `notes`, `trip_expense_shares`). Both have budget on `trips` with an optional currency, and neither has trip dates. Much closer than the two epic 004 runs.
- **Assumptions: mostly good, a few storage details leak.** Invoices: "entered as a percentage, such as 20 for 20%", "a short code such as USD ... rather than in a separate list" and "recorded as free text". Farm initial says "Each account belongs to one person" while `accounts.person_id` is optional.
- **Redundant columns.** Farm purchases and sales keep `item_description` next to the new `item_kind_id`. Cofounder-c has both `people.location` (text) and a `locations` list.
- **Other providers.** With the same finance prompts, `deepseek-v4-flash` and `kimi-k3` passed all checks on both prompts in 1 call. `kimi-k3` leaves `loans` without a borrower reference and `deepseek-v4-flash` uses `paid_by`/`borrower` as plain text, so quality varies. GLM fails for the stringification reason above.
- **Strict mode.** Opt-in and off by default, because it makes Haiku return a meaningless `drop_table` change set (Sonnet was fine). Not exercised in the checkpoint. OpenAI strict mode is implemented but has never run live (no key).

## Checkpoint items

| Item | Status |
|---|---|
| `cargo build/test/clippy/fmt` | Pass |
| Facts, not results | Pass |
| Conventions | Pass |
| Real references | Pass (one valid cofounder run) |
| Data vs schema | Pass |
| Assumptions | Mostly pass; storage details leak in invoices and one contradiction in farm |
| Churn | Pass |
| Checks | Pass |
| Retries | Partial: logging and follow-up work; the follow-up does not fix stringified arguments |
| Models | Pass |
| Other provider | Partial: two Neuralwatt models work, two GLM models fail |

## Open items

- Resolved: the string-repair step was added to this epic (see follow-up below). Before it, expect roughly one in three Haiku runs on large prompts to fail.
- Resolved: the checkpoint text in `epics/005` is kept as it was run (`claude-haiku-5-5` as the base). `project-base.json` now defaults to `neuralwatt/glm-5.3-flash`, which works since the string-argument repair.
- Tighten Task 1's assumptions rule or add a checker advice for storage-detail wording (percentages, "short code", "free text"), and for a text column next to a list of the same thing.
- Run OpenAI strict mode live once a key exists.
- `.projects/strict-live` and `.projects/strict-live2` are scratch runs and can be deleted. The `cp-*` projects are kept for inspection.

## Follow-up after the review (2026-10-09)

- **String-argument repair built** in `extract_with_model` (`unstringify`, `repair_output`, `repaired_output` log line), tested from the failed `cp-finance-nw` logs (fixtures in `crates/nocodo-agent/src/extract/fixtures/`). When repair unwraps strings but the value still fails, the retry message and `invalid_output` carry the repaired-value error (for example missing field `assumptions`), with the original in `original_error`. Unit test only; not seen live.
- **Preamble tightened:** no storage wording in assumptions; no free-text column next to a list of the same thing. Haiku rerun (`.projects/v7-inv`, `.projects/v7-farm`): mostly fixed; leaks remain ("short code", "kept as a list"). Farm follow-up keeps the old `description` column beside the new `item_kind_id` (evolution rules forbid dropping). A checker advice rule is the suggested follow-up, not built.
- **GLM reruns:** `.projects/v8-*`: finance follow-up failed 3 times (`change_set` stringified, `assumptions` omitted). `.projects/v9-finance-glm` and `.projects/v9-cofounder-glm` (with repair): all 4 prompts accepted, 6 completions all repaired, 0 `invalid_output`; initial prompts took 2 agent runs because of checker findings. The cofounder schema is generic (`entity_items`) and `people.location` is text next to lists.
- **Fixtures:** farm prompts added as `fixtures/prompts/farm/01-initial.md` and `02-follow-up.md`.
