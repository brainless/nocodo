# Epic 005: Schema Agent Hardening

**Status:** Not started
**Goal:** Make every Schema agent run trustworthy and inspectable before building on it. The agent models the facts the user enters, not results computed from them, uses real references only, and is told exactly which columns every table gets. Invalid model output is prevented where the provider allows it; otherwise it is retried with feedback and logged. Each accepted migration records the SQLite checks it passed. The model and provider can be anything Rig supports, not only Anthropic.

**Prerequisite:** Epic 004 (closed; checkpoint review in `epics-notes/004-schema-agent.md`).

---

## Context

From the epic 004 checkpoint (`claude-haiku-5-5`, six runs in `.projects/`: travel twice, small farm finance and cofounder matching, each of the last two with a follow-up). Full review in `epics-notes/004-schema-agent.md`. The travel prompt:

> I plan trips with friends. For each trip I want to track the budget, who is coming, what each person paid for (flights, hotels, food, activities), which currency it was paid in, and who owes whom at the end.

- **Neither accepted travel schema can answer "who owes whom".** Both store `expenses` (who paid) plus a `settlements` table holding the final debts as entered values. Nothing records who shares each expense, so the debts can't be worked out; the user would have to compute and type them in. The first, discarded answer in run 1 had `expense_shares` and said "who owes whom is worked out from the payments and how they are split, not stored separately". Run 2 had no parse error and still lost it, so this is a modeling problem, not a side effect of the retry. The preamble has no rule about storing facts rather than computed results.
- **Hidden retries in 3 of 6 runs.** The model sent `submit` arguments as strings: in travel, `assumptions` was a JSON string; in farm follow-up and cofounder initial, `change_set` was one string holding the change set followed by `"assumptions": [...]`. Rig's `TypedRun` (`~/Projects/rig/crates/rig-agent/src/agent/typed.rs`) failed to deserialize and retried **from scratch, without telling the model why**. Our `LogHook` only sees `on_invalid_tool_call` (undispatchable calls), so the logs have two `completion` lines and no `invalid_output` line, and `events.jsonl` shows one clean attempt. Those runs cost about twice the tokens and time.
- **Polymorphic references.** Cofounder's `item_similarities` uses `item_type` (text) + `item_id` (integer, no `references`), so a score can point at any of five lists and nothing checks that the item exists. Both the discarded and accepted answers did this; no checker rule caught it.
- **Schema change for a data-only request.** Farm follow-up: "We opened a separate business account" only needs a new row in `accounts`, but the agent added `accounts.is_business_account`.
- **No type for fractional numbers.** `ColumnType` has `integer` and `money` only. Cofounder's similarity score became `integer`, and an assumption leaked "the score column may need a decimal type".
- **No strict tool use.** The extractor forces a `submit` tool (`ToolChoice::Required`), but we don't turn on the provider's strict tool schemas, so the arguments aren't constrained. Rig supports it on the Anthropic wire (`with_strict_tools()` in `rig-core/src/providers/anthropic/wire.rs`; see `rig-cassette/tests/providers/anthropic/cassette/messages_strict_tools.rs`) and on OpenAI-format wires. It is a wire option, so it must be reachable from however `build_model` constructs the model.
- **Reference naming not enforced.** The discarded first answer named references `payer_id` (→ `people`) and `category_id` (→ `expense_categories`), against the preamble's `<singular_table>_id` convention. No checker rule catches this.
- **Churn and unrequested extras.** Travel, same prompt, same model: `trip_participants` vs `trip_members`, `budget_amount` vs `budget`, `people.email` in one run only, trip start/end dates in one run only. Extras the user never mentioned account for most of the difference.
- **Assumptions don't always match the schema.** Run 1 says "Each person … is not tracked across different trips as a separate record", but its `people` table is shared across trips. "Amounts are stored in the smallest unit of their currency" describes storage, not something a user would be asked.
- **Implicit columns are hard-coded in several places.** The preamble names `id`, `created_at` and `updated_at` without their types or meaning; sqlgen and the checker (`RESERVED_COLUMNS` in `nocodo-checker/src/rules.rs`) keep their own copies.
- **Checks are invisible.** The checker dry-runs every migration (up → down → up on a copy, compared with the spec), but `change_set_accepted` records only the migration name. A reviewer can't see that the migration was ever run on SQLite.
- **Anthropic only.** `ModelConfig.provider` only allows `anthropic`. Later roles will need models of different strength and cost, and the Schema agent should be tried on another provider family.

## Tasks

### Task 1: Modeling rules in the preamble (`nocodo-agent-schema`)

Update `src/preamble.md`:

- **Store facts, not computed results.** Store what the user enters or observes. Anything that can be worked out from stored entries (totals, balances, who owes whom, remaining budget) is not stored. Instead, make sure everything needed to work it out is stored. Example: to know who owes whom, record who paid each expense and who shares it.
- **Model only what was asked.** Add the tables and columns the request needs, plus what's needed to connect them and tell entries apart. If you add anything the user didn't mention (such as an email address or trip dates), list it in `assumptions`.
- **New entries are data, not schema.** When the user mentions a new account, person, category or other entry of a kind the schema already holds, that is data they will enter later; don't change the schema for it. Change the schema only for new kinds of things or new details about them. If a request has both, change only the part that needs it.
- **Every link is a real reference.** Never pair a "which list" text column with an ID column. If something can point at one of several lists, give it one optional reference per list, or use a separate linking table per list.
- **Assumptions describe the user's world and match the change set.** Each assumption must agree with the tables and references you return. Don't describe how things are stored (units, formats, types, lookups).

### Task 2: One definition of the implicit columns (`nocodo-types`, `nocodo-sqlgen`, `nocodo-checker`, `nocodo-agent-schema`)

- Define the implicit columns once in `nocodo-types`: name, `ColumnType` and a plain-language description for `id`, `created_at` and `updated_at`. sqlgen (column generation), the checker's `reserved_column` rule and the Schema agent all read this list. Remove the other hard-coded copies.
- The preamble gets a section rendered from that list (a placeholder in `preamble.md`, filled deterministically when the message is built). It says:
  - every table has these columns, with their types and meaning;
  - `<x>_id` columns reference the `id` of table `<x>`;
  - `created_at` already records when an entry was added, so add a date column only for a real-world date (such as `paid_on`);
  - never list these columns yourself.
- A snapshot test covers the full rendered preamble, including Task 1's rules.

### Task 3: Checker rules for references (`nocodo-types`, `nocodo-checker`)

- New rule `reference_name`: a column with `references: <table>` must be named `<singular>_id` or `<qualifier>_<singular>_id`, where `<singular>` is the singular form of `<table>` (`people` → `person`, `expense_categories` → `expense_category`). Examples: `person_id` and `paid_by_person_id` pass; `payer_id` and `category_id` (→ `expense_categories`) fail. The finding names the expected column name, so a repair attempt can fix it.
- New rule `unlinked_id`: an `integer` column whose name ends in `_id` must set `references`. This catches polymorphic pairs such as `item_type` + `item_id`. The finding says to link each list with its own reference column or linking table (Task 1's rule), or, if the value is an outside code rather than a link, to rename it (e.g. `_number`) and use `text`.
- Singular forms come from one small deterministic function in `nocodo-types` (regular rules plus a short list of irregular words such as `people`). No inflection crate unless the rules get out of hand.
- Apply both rules to `create_table` and `add_column`.

### Task 4: A type for fractional numbers (`nocodo-types`, `nocodo-sqlgen`, `nocodo-checker`, `nocodo-agent-schema`)

- Add `ColumnType::Decimal`: a fractional number that is not money (scores, weights, percentages, quantities in kilograms). sqlgen maps it to SQLite `REAL`; the checker's spec-vs-database comparison accepts it.
- The preamble's conventions say: `money` for amounts of money, `decimal` for other numbers that can have a fraction, `integer` for counts and whole numbers.
- Existing specs and change sets load unchanged.

### Task 5: Own the extraction retry loop (`nocodo-agent`)

- Set Rig's extractor `retries(0)`. `run_extractor` loops up to `ctx.retries` times itself.
- On `StructuredOutputError::Deserialization` or `EmptyResponse`:
  - append an `invalid_output` line with the call number, the raw output and the serde error;
  - call again, continuing the conversation instead of starting over. Pass the original user message and the model's rejected `submit` call as history (`AgentRunner::history`), followed by an error tool result that quotes the serde error verbatim and asks for one valid `submit` call. If Rig's typed run can't send an error tool result, send a new user message instead: the original input, the rejected arguments and the error, verbatim. Record which approach was used in Task Notes.
- `run_finished` records `calls` (model calls made) and `invalid_outputs` (how many were rejected).
- These retries stay inside one agent run. They don't count against the orchestrator's `max_attempts`, which is for checker findings.

### Task 6: Any provider through Rig's registry (`nocodo-types`, `nocodo-agent`)

- **First, confirm that strict tools (Task 7) can be enabled on a model built through the registry**, e.g. `Provider::Configured(ProviderConfig)` with typed wire options (`~/Projects/rig/crates/rig-core/src/providers/registry.rs`). If it can't, `build_model` builds Anthropic and OpenAI-format models through their own clients and every other provider through the registry. Record the outcome in Task Notes.
- `ModelConfig { provider: String, model: String }`. `provider` is a Rig provider selection (`anthropic`, `openai`, `gemini`, `openrouter`, `ollama`, `deepseek`, …). `nocodo-types` must not depend on Rig, so it stays a string. Existing `project.json` and `project-base.json` files (`"provider": "anthropic"`) load unchanged. Remove the `Provider` enum.
- `build_model` resolves the provider with `ProviderId::resolve(provider)`, then builds the model with `ProviderRef::registered(id, model)` and `completion_model()`. `ExtractorBuilder::new` already takes a `DynModel<Completion>`.
- Errors are specific:
  - an unknown provider lists the valid selections (`ProviderId::all()`);
  - a missing key names the exact variable for that provider (`ProviderId::api_key_env()`). This check is skipped when `requires_credential()` is false (local servers).
- Check the Cranelift build with the providers this pulls in, and record any override in `AGENTS.md`.
- `.env.example` lists the key variables for the common providers.

### Task 7: Strict tool use where the provider supports it (`nocodo-agent`)

- When the selected provider supports strict tool schemas (Anthropic, OpenAI-format wires), turn them on so the `submit` arguments are constrained to the schema. Other providers rely on Task 5.
- Do one live run per supported provider family to confirm the `SchemaProposal` schema is accepted in strict mode. If the provider rejects it (e.g. the tagged `SchemaChange` enum), fix it with serde/schemars attributes on the types, as in epic 004. Don't add a parallel LLM-only type.
- The `run_started` log line records `strict: true/false`.

### Task 8: Record the checks each migration passed (`nocodo-types`, `nocodo-checker`, `nocodo-orchestrator`, `api`)

- New type `CheckStep { check: String, passed: bool, detail: String }`, with `check` a stable snake_case code:
  `static_rules`, `earlier_migrations_applied`, `db_matches_spec_before`, `up`, `db_matches_spec_after`, `foreign_key_check`, `down`, `db_matches_spec_after_down`, `up_again`, `db_matches_spec_after_up_again`.
- `detail` is a plain sentence a reviewer can read alone, e.g. "Ran 3 earlier migrations on an empty database (no data.sqlite yet)."
- The checker returns the steps it ran alongside its findings. `ChangeSetAccepted` and `ChangeSetRejected` gain `checks: Vec<CheckStep>` (`#[serde(default)]`, so old events load). A rejection lists the steps up to and including the one that failed.
- `nocodo-api changes` and `prompt` print one line after the result, e.g. `Checked on SQLite: up, down, up again; database matched the spec each time.`

### Task 9: Model in events (`nocodo-types`, `api`)

- `Actor::SchemaAgent` gains `model: String` (`provider:model`, `#[serde(default)]`), so `events.jsonl` shows which model proposed each change set without opening `.agent-logs`.

### Task 10: Project folder path in docs

- Testers keep projects in `.projects/` (already gitignored). Update `AGENTS.md` (manual test loop) to use `.projects/<name>` instead of `projects/<name>`.

### Task 11: Tests

No live LLM calls in `cargo test`.

- Preamble: the snapshot from Task 2 covers the implicit-column section and the Task 1 rules.
- Implicit columns: the shared list drives sqlgen output (existing snapshots unchanged) and the `reserved_column` rule.
- `reference_name`: `person_id`, `paid_by_person_id` and `expense_category_id` pass; `payer_id` and `category_id` fail with the expected name in the finding. The singular-form function covers regular and irregular plurals.
- `unlinked_id`: `item_type` + `item_id` (no `references`) fails; `person_id` with `references` passes; a `text` column ending in `_id` is not flagged.
- `decimal`: sqlgen snapshot for a `decimal` column (`REAL`), and a dry run of a table with one passes the checker.
- Retry loop: the follow-up message builder is deterministic (snapshot). If Rig's API allows it without a network call, also test the loop with a scripted model; otherwise say so in Task Notes.
- `build_model`: an unknown provider and a missing key give the specific errors; `ollama` (no credential needed) builds without a key.
- Old `project.json` (`"provider": "anthropic"`) and old events (no `checks`, no `model`) still load.
- Orchestrator: an accepted proposal records all check steps, in order, all passed; a broken migration's rejection records the failing step.

## Out of scope

- Keeping `updated_at` current when a row is edited (no trigger exists yet). That belongs with the epic that writes data.
- Computing derived values such as balances or who owes whom (needs the runtime and query layer).
- Applying migrations to `data.sqlite` (epic 006).
- Questions to the user, and few-shot examples in the preamble.
- Prompt caching, cost tracking across runs.
- Choosing models automatically per task.
- An LLM reviewer on a stronger model that critiques proposals (e.g. stored results, wide type-switched tables). Possible later, as advice or questions only; pass/fail stays with the deterministic checker.

## Checkpoint

Manual session. The tester sets keys in `.env` and, with `claude-haiku-5-5` in `project-base.json`, runs the fixture scenarios (`finance`, `inventory`, `invoices`, each initial plus follow-up) in fresh projects under `.projects/`. They rerun the small farm and cofounder prompts from epic 004 (initial plus follow-up, text in their `events.jsonl`) in fresh projects, repeat `finance` with one non-Anthropic provider of their choice (`init --base <file>`), and run the travel prompt twice in fresh projects to compare churn:

```bash
for p in travel-2 travel-3; do
  cargo run -q --bin nocodo-api -- init --project .projects/$p
  cargo run -q --bin nocodo-api -- prompt --project .projects/$p "I plan trips with friends. For each trip I want to track the budget, who is coming, what each person paid for (flights, hotels, food, activities), which currency it was paid in, and who owes whom at the end."
done
```

- [ ] `cargo build/test/clippy/fmt` pass.
- [ ] The reviewer confirms, from the project folders alone:
  - **Facts, not results:** in both travel projects, who owes whom can be worked out from what is stored (each expense records who shares it), and no table stores computed totals or debts. The same holds for the finance and invoices scenarios (e.g. no stored balances).
  - **Conventions:** every reference column follows the naming rule; no proposal lists `id`, `created_at` or `updated_at`, or adds a redundant "date recorded" column.
  - **Real references:** no `<x>_type` + `<x>_id` pairs; the cofounder similarity score links its items through real references and uses `decimal`.
  - **Data vs schema:** the farm follow-up adds no schema change for the new business account.
  - **Assumptions:** they agree with the schema, avoid storage details, and list every extra the user didn't ask for.
  - **Churn:** how much `travel-2` and `travel-3` differ, compared with the two epic 004 travel runs.
  - **Checks:** each `change_set_accepted` event lists the SQLite checks it passed, with readable details.
  - **Retries:** every rejected model output has an `invalid_output` line with the reason, and the follow-up call quotes it. If no output was rejected in the session, say so; the Task 11 tests cover the loop.
  - **Models:** events show the model for every agent proposal, and `run_started` shows whether strict mode was on.
  - **Other provider:** the non-Anthropic run worked, or its failure is clearly explained in the logs.
- [ ] Task Notes added below, and `Status: Done`.

---

## Task Notes

Status is intentionally not set to Done: the manual checkpoint has not been run.

### What was built

Tasks 1-11 as specified, plus two extras: a Neuralwatt provider in `build_model`, and strict tools as an opt-in `ModelConfig.strict` (default false).

### Decisions and surprises

- **Task 3:** the finance fixture column `transfer_to_id` was renamed `transfer_to_account` (it references `accounts`, so it needed a name following the new `reference_name` rule).
- **Task 5 retry follow-up approach: error tool result.** After a rejected `submit`, the history is the original user message, the model's rejected `submit` call, then a tool result quoting the serde error verbatim and asking for one valid `submit` call. Rig 0.43.0's `ToolResult` has no `is_error`, so the rejection is expressed in the result text. When the model made no tool call at all (`EmptyResponse`), the fallback is a new user message (original input, rejected output, error). Both follow-up builders are snapshot-tested. The scripted-model loop test works offline through `MockCompletionModel`, so the loop itself is tested without a network call.
- **Task 6 outcome: strict tools cannot be enabled on registry-built models in Rig 0.43.0** (the version locked in this workspace; `~/Projects/rig` is newer and differs). So `build_model` builds Anthropic and OpenAI-format models directly through their own clients, and every other provider through the registry. Registry findings for 0.43.0: there is no `ollama` (the keyless test uses `llamacpp`), and `gemini` resolves only as `gcp.gemini/gemini`.
- **Neuralwatt (extra):** added as a direct OpenAI-chat preset (base URL `https://api.neuralwatt.com/v1`, key `NEURALWATT_API_KEY`), strict off. The model name `glm-5.3-flash` in `project-base.json` is unverified against the portal; the synth project uses `glm-5.3-flex`.
- **Task 7 live runs:** Anthropic: the `SchemaProposal` schema was accepted in strict mode, but `claude-haiku-5-5` degraded under strict to a meaningless `drop_table` "placeholder" change set (`claude-sonnet-5-5` was fine). Strict is therefore opt-in per `ModelConfig` (default off). **The OpenAI strict live run was NOT done** (no key; Neuralwatt is in use for now), so OpenAI strict is implemented but unverified. `run_started` records `strict`.
- **Task 8:** the `foreign_key_check` step is only recorded after the first `up`.

### Tested (no live LLM calls)

Preamble snapshot (implicit-column section and modeling rules); `reserved_column` driven by the shared `IMPLICIT_COLUMNS` list (sqlgen snapshots unchanged); `reference_name` and `unlinked_id` rules; singular-form function (regular and irregular); `decimal` sqlgen snapshot and checker dry run; retry loop (follow-up snapshots, scripted-model loop, bounded retries with logged failure); `build_model` errors (unknown provider lists valid selections, missing key names the variable, keyless `llamacpp` builds, strict flags); old `project.json` / events without `checks` or `model` load; orchestrator accepted run records all steps in order, broken migration rejection ends with the failing step. The task 11 audit found everything covered; the only change was making the `reserved_column` test iterate the shared list.

### For the reviewer

The checkpoint text says to use `claude-haiku-5-5` in `project-base.json`, but `project-base.json` now uses `neuralwatt/glm-5.3-flash`. The checkpoint text was left unedited.
