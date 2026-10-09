# Epic 005: Schema Agent Hardening

**Status:** Not started
**Goal:** Make every Schema agent run trustworthy and inspectable before building on it. Invalid model output is prevented where the provider allows it, and is otherwise retried with feedback and logged. The agent is told exactly which columns every table gets. Each accepted migration records the checks it passed. The model and provider can be anything Rig supports, not only Anthropic.

**Prerequisite:** Epic 004.

---

## Context

Findings from the first epic 004 manual run (`.projects/travel-finance-planner`, `claude-haiku-5-5`):

- In the first model call, the `submit` tool's `assumptions` argument came back as a malformed JSON *string* instead of an array. Rig's `TypedRun` (`~/Projects/rig/crates/rig-agent/src/agent/typed.rs`) failed to deserialize it and retried **from scratch, without telling the model why**. Our `LogHook` only sees `on_invalid_tool_call` (undispatchable calls), so the log has two `completion` lines and no `invalid_output` line, and `events.jsonl` shows one clean attempt. The discarded first answer was also the better design: it had `expense_shares`, and the retry dropped it.
- We already use tool calls: the extractor forces a `submit` tool (`ToolChoice::Required`). We don't turn on the provider's **strict tool use**, so the arguments aren't constrained to the schema. Rig supports it (`with_strict_tools()` on the Anthropic and OpenAI wires; see `rig-cassette/tests/providers/anthropic/cassette/messages_strict_tools.rs`).
- The preamble says `id`, `created_at` and `updated_at` exist automatically, but not their types or meaning. The agent can't tell that `created_at` already answers "when was this entry recorded".
- The checker dry-runs every migration (up → down → up on a copy, compared with the spec), but `change_set_accepted` only records the migration name. A reviewer can't see that the migration was ever run on SQLite.
- `ModelConfig.provider` only allows `anthropic`. Agents must work with any provider and model; different roles will need models of different strength and cost.

## Tasks

### Task 1: Any provider through Rig's registry (`nocodo-types`, `nocodo-agent`)

- `ModelConfig { provider: String, model: String }`. `provider` is a Rig provider selection (`anthropic`, `openai`, `gemini`, `openrouter`, `ollama`, `deepseek`, …). `nocodo-types` must not depend on Rig, so it stays a string. Existing `project.json` and `project-base.json` files (`"provider": "anthropic"`) load unchanged. Remove the `Provider` enum.
- `build_model` uses `rig_core::providers::registry` (`~/Projects/rig/crates/rig-core/src/providers/registry.rs`): `ProviderId::resolve(provider)`, then `ProviderRef::registered(id, model)` and `completion_model()`. `ExtractorBuilder::new` already takes a `DynModel<Completion>`.
- Errors are specific:
  - an unknown provider lists the valid selections (`ProviderId::all()`);
  - a missing key names the exact variable for that provider (`ProviderId::api_key_env()`), skipped when `requires_credential()` is false (local servers).
- Check the Cranelift build with the providers this pulls in; record any override in `AGENTS.md`.
- `.env.example` lists the key variables for the common providers.

### Task 2: Strict tool use where the provider supports it (`nocodo-agent`)

- When the selected provider supports strict tool schemas (Anthropic, OpenAI-format wires), enable it so the `submit` arguments are constrained to the schema. Other providers fall back to Task 3.
- Do one live run per supported provider family to confirm the `SchemaProposal` schema is accepted in strict mode. If the provider rejects it (e.g. the tagged `SchemaChange` enum), fix it with serde/schemars attributes on the types, as in epic 004. Don't add a parallel LLM-only type.
- The `run_started` log line records `strict: true/false`.

### Task 3: Own the extraction retry loop (`nocodo-agent`)

- Set Rig's extractor `retries(0)`. `run_extractor` loops up to `ctx.retries` times itself.
- On `StructuredOutputError::Deserialization` or `EmptyResponse`:
  - append an `invalid_output` line with the call number, the raw output and the serde error;
  - call again with a deterministic follow-up message that quotes the error verbatim and asks for one valid `submit` call. Don't start from scratch.
- `run_finished` records `calls` (model calls made) and `invalid_outputs` (how many were rejected).
- These retries stay inside one agent run. They don't count against the orchestrator's `max_attempts`, which is for checker findings.

### Task 4: Tell the agent exactly which columns every table gets

- Define the implicit columns once, in `nocodo-types`: name, `ColumnType` and plain-language description for `id`, `created_at` and `updated_at`. `sqlgen` (column names), the checker's `reserved_column` rule and the Schema agent all read this list. No other hard-coded copies.
- The preamble gets a section rendered from that list (a placeholder in `preamble.md`, filled deterministically at build time of the message). It says:
  - every table has these columns, with their types and meaning;
  - `<x>_id` columns reference the `id` of table `<x>`;
  - `created_at` already records when an entry was added, so only add a date column for a real-world date (such as `paid_on`);
  - never list these columns yourself.
- A snapshot test covers the rendered preamble.

### Task 5: Record the checks each migration passed (`nocodo-types`, `nocodo-checker`, `nocodo-orchestrator`)

- New type `CheckStep { check: String, passed: bool, detail: String }`, with `check` a stable snake_case code:
  `static_rules`, `earlier_migrations_applied`, `db_matches_spec_before`, `up`, `db_matches_spec_after`, `foreign_key_check`, `down`, `db_matches_spec_after_down`, `up_again`, `db_matches_spec_after_up_again`.
- `detail` is a plain sentence a reviewer can read alone, e.g. "Ran 3 earlier migrations on an empty database (no data.sqlite yet)."
- The checker returns the steps it ran alongside its findings. `ChangeSetAccepted` and `ChangeSetRejected` gain `checks: Vec<CheckStep>` (`#[serde(default)]`, so old events load). A rejection lists the steps up to and including the one that failed.
- `nocodo-api changes` and `prompt` print one line after the result, e.g. `Checked on SQLite: up, down, up again; database matched the spec each time.`

### Task 6: Model in events (`nocodo-types`, `api`)

- `Actor::SchemaAgent` gains `model: String` (`provider:model`, `#[serde(default)]`), so `events.jsonl` shows which model proposed each change set without opening `.agent-logs`.

### Task 7: Tests

No live LLM calls in `cargo test`.

- `build_model`: an unknown provider and a missing key give the specific errors; `ollama` (no credential needed) builds without a key.
- Old `project.json` (`"provider": "anthropic"`) and old events (no `checks`, no `model`) still load.
- The retry loop: the follow-up message builder is deterministic (snapshot). If Rig's API allows it without a network call, also test the loop with a scripted model; otherwise say so in Task Notes.
- The implicit-column list drives sqlgen output (existing snapshots unchanged) and the `reserved_column` rule.
- Orchestrator: an accepted proposal records all check steps in order and passed; a broken migration's rejection records the failing step.

## Out of scope

- Keeping `updated_at` current when a row is edited (no trigger exists yet). That belongs with the epic that writes data.
- Applying migrations to `data.sqlite` (epic 006).
- Prompt caching, cost tracking across runs.
- Choosing models automatically per task.

## Checkpoint

Manual session. The tester sets keys in `.env` and runs the epic 004 checkpoint (three scenarios plus the `finance-2` churn run) with `claude-haiku-5-5` in `project-base.json`, then repeats `finance` with one non-Anthropic provider of their choice (`init --base <file>`). Also rerun the travel prompt in a fresh project:

```bash
cargo run -q --bin nocodo-api -- init --project .projects/travel-2
cargo run -q --bin nocodo-api -- prompt --project .projects/travel-2 "I plan trips with friends. For each trip I want to track the budget, who is coming, what each person paid for (flights, hotels, food, activities), which currency it was paid in, and who owes whom at the end."
```

- [ ] `cargo build/test/clippy/fmt` pass.
- [ ] The reviewer confirms, from the project folders alone:
  - each `change_set_accepted` event lists the SQLite checks it passed, with readable details;
  - every rejected model output has an `invalid_output` line with the reason, and the follow-up call quotes it;
  - no proposal lists `id`, `created_at` or `updated_at`, or adds a redundant "date recorded" column;
  - events show the model for every agent proposal;
  - the non-Anthropic run worked, or its failure is clearly explained in the logs.
- [ ] Epic 004's checkpoint notes are filled in from this session.
- [ ] Task Notes added below, and `Status: Done`.
