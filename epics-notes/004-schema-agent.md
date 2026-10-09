# Epic 004 notes

Review of the manual checkpoint session. All runs used `claude-haiku-5-5`. The tester used their own prompts instead of the `fixtures/prompts/` scenarios:

| Project | Prompts | Migrations |
|---|---|---|
| `.projects/travel-finance-planner` | travel (initial) | 0001 |
| `.projects/travel-finance-planner-v2` | same travel prompt (churn) | 0001 |
| `.projects/small-farm-finance-planner` | farm money (initial), business account + items (follow-up) | 0001, 0002 |
| `.projects/cofounder-matching` | matching platform (initial), importance + relocation (follow-up) | 0001, 0002 |

The `finance`, `inventory` and `invoices` fixtures were not run. `epics/004-schema-agent.md` still has no `Status: Done` and no Task Notes; the go/no-go below is a proposal for the tester.

## Runs

| Run | Model calls | Hidden retries | Input tokens | Output tokens | Duration |
|---|---|---|---|---|---|
| travel | 2 | 1 | 5,968 | 3,373 | 13.8 s |
| travel-v2 | 1 | 0 | 2,984 | 1,603 | 6.8 s |
| farm, initial | 1 | 0 | 3,057 | 1,263 | 6.3 s |
| farm, follow-up | 2 | 1 | 8,620 | 1,533 | 8.2 s |
| cofounder, initial | 2 | 1 | 6,156 | 5,840 | 20.9 s |
| cofounder, follow-up | 1 | 0 | 6,754 | 958 | 4.6 s |

Every run was accepted at orchestrator attempt 1. No checker finding fired, so the repair loop was never exercised against a real model.

## Findings

- **Hidden retries in 3 of 6 runs.** Same failure each time: the model sends the `submit` arguments as strings instead of objects. In travel, `assumptions` was a JSON string. In farm follow-up and cofounder initial, `change_set` was one string holding the change set *and* `"assumptions": [...]` after it, with no top-level `assumptions` argument. Rig's `TypedRun` retried from scratch without telling the model. The logs have no `invalid_output` line, and `events.jsonl` shows a clean single attempt. This costs about 2× tokens and latency, and the discarded design differs from the accepted one. Epic 005 Tasks 4 and 6 (own retry loop, strict tool use) are the fix; at 50% this is the top priority, not an edge case.
- **Discarded designs are sometimes better, sometimes worse.** Travel: the discarded answer had `expense_shares` (better). Farm follow-up: the discarded answer put one `item_category_id` and `quantity` on `money_movements` (worse; the accepted `money_movement_items` allows several items per payment). Cofounder: the discarded answer had per-person `created_by_person_id` on every list and a `custom_entities` table (different, not clearly better). This is run-to-run variance; nothing reviews it.
- **Computed results stored as facts (travel).** Both accepted travel schemas store `settlements` (who owes whom) as entered values and record nothing about who shares each expense, so the debts can't be worked out. Covered by epic 005 Task 1.
- **Polymorphic references (cofounder).** `item_similarities` uses `item_type` + `item_id` (text + integer, no `references`). A score can point at any of five lists, the database can't check that it exists, and the checker allows it. It breaks the "relations through `references`" rule without any checker finding. Both the discarded and the accepted answers did this, so it is not a one-off.
- **Wide table with a type field (farm).** `money_movements` has `movement_type` (purchase / sale / transfer as text) and five optional references (`destination_account_id`, `seller_id`, `buyer_id`, `person_id`), most of which are null for any given row. A developer would accept it for a first version, but which references a row must have depends on `movement_type`, and nothing enforces that. The pigs and chickens in the prompt weren't modeled in the initial run; the follow-up added them as `item_categories`.
- **Schema change for a data-only request (farm).** "We opened a separate business account" needed no schema change: it's a new row in `accounts` with no `person_id`. The agent added `accounts.is_business_account` (boolean) anyway, and its own assumption admits "only its existence is recorded". The preamble's "If nothing needs to change, return an empty change set" didn't help when the same prompt also asked for a real change.
- **No decimal type.** `ColumnType` has `integer` and `money` but nothing for fractions. Cofounder's similarity score became `integer`, and an assumption leaked it: "the score column may need a decimal type", which mentions a column and a type. Farm quantities are whole numbers only (assumption: "no separate unit type such as kilograms"). Weights, percentages and ratings with fractions will hit this.
- **Reference naming slips.** Only in discarded answers so far (travel: `payer_id`, `category_id`). Covered by epic 005 Task 3.

## Checkpoint items

- **Schema quality:** real tables and foreign keys throughout, apart from the polymorphic `item_similarities`. Conventions (snake_case, plural tables, descriptions, `money` for amounts) were followed in every accepted answer. No JSON or key-value columns. Weak points: stored computed results (travel), polymorphic references (cofounder), wide type-switched table (farm).
- **Additive evolution:** both follow-ups were purely additive: new tables, new nullable columns, new indexes; no drops or renames. Both reused existing tables well: cofounder linked relocation to the existing `locations`, as the prompt asked ("a list of cities we store"), and added `importance` to the five person links the prompt named but not to `person_roles`, which it didn't. sqlgen's `down.sql` rebuilds the tables to undo `add_column`, as designed.
- **Repairs:** none needed; no checker codes fired. See hidden retries above.
- **Assumptions:** mostly the right things to ask a user (one budget currency per trip, several items per purchase, whole-number quantities, cities from the existing list). Problems:
  - one contradicts the schema (travel: "Each person … is not tracked across different trips as a separate record", but `people` is shared across trips);
  - some describe storage ("Amounts are stored in the smallest unit", "Money amounts are recorded in cents", "the score column may need a decimal type");
  - extras the user didn't ask for (email, trip dates, bio) are only sometimes listed.
- **Churn:** the two travel runs differ in table names (`trip_participants` / `trip_members`), column names (`budget_amount` / `budget`) and unrequested extras (`people.email`, trip dates). Same overall structure, same weakness (stored `settlements`).
- **Debuggability:** good. Each table traces to its prompt through `prompt_seq` in `events.jsonl` and the `run_id` on the actor, and `.agent-logs` hold the full preamble, input, raw responses and usage. The gap is the hidden retries: only the `completion` line count in `.agent-logs` reveals them.
- **Token usage and latency:** 3–9k input and 1–6k output tokens per run, 5–21 s. Hidden retries roughly double both.

## Proposed go/no-go

**Go to epic 005**, with additions. The pipeline works end to end: prompts produce reasonable relational schemas, follow-ups evolve them additively, and runs are traceable. The hidden retry rate (3 of 6) confirms that hardening comes first.

Findings epic 005 doesn't cover yet:

1. **Polymorphic references:** a checker rule (e.g. flag a `<x>_type` text column next to a matching `<x>_id` integer without `references`) plus a preamble rule: a link to one of several lists is modeled as separate link tables, or one optional reference per list.
2. **Data-only requests:** a preamble rule: a new entry the user describes (an account, a person, a category value) is data, not a schema change. Only change the schema for new kinds of things or new details.
3. **Decimal type:** add a `decimal` `ColumnType`, or decide that fractions aren't supported yet and say so in the preamble so the agent doesn't leak it into assumptions.
4. **Fixture scenarios:** `finance`, `inventory` and `invoices` have never been run. Epic 005's checkpoint already includes them; its prerequisite (epic 004 closed with those scenarios run) should be relaxed to accept this session instead.
