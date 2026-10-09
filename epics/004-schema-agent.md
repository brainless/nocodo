# Epic 004: Schema Agent

**Status:** Done
**Goal:** The first real test of the hypothesis. A prompt goes in, the Schema agent (the only LLM agent) emits a typed `ChangeSet`, and the epic 003 pipeline checks it and writes the migration. Checker findings go back to the agent for a bounded number of repair attempts. Every LLM call is logged so the reviewer can see why each table exists.

**Prerequisite:** Epic 003.

---

## Context

- The agent emits typed values only, through Rig's `Extractor` (`~/Projects/rig/crates/rig-agent/src/extractor.rs`). It never writes SQL or files.
- **No questions yet.** In this epic the tester may write detailed, even technical, prompts (`PRD.md` §8). Instead of asking questions, the agent lists its **assumptions**: what it would have asked. These become the seed for the question epic.
- A follow-up prompt is just another run against the current `schema.json`. Additive evolution is tested here.
- Replay with `rig-cassette` is out of scope, but the logs must capture enough to reconstruct each run by reading them.

## Tasks

### Task 1: Types additions (`nocodo-types`)

- Add `schemars` and derive `JsonSchema` on `ChangeSet`, `SchemaChange`, `Column`, `ColumnType` and `Index`. If the generated JSON schema gives the provider trouble (e.g. tagged-enum `oneOf`), fix it with serde/schemars attributes on these types. Don't add a parallel "LLM-only" copy.
- Model config:
  - `ProjectConfig` gains `models: ModelsConfig { schema: ModelConfig }`, with `#[serde(default)]` so 003 projects still load.
  - `ModelConfig { provider: Provider, model: String }`.
  - `Provider::Anthropic` is the only variant. The default model is `claude-sonnet-5-5`. `init` writes the default explicitly, so the tester can change it in `project.json`.
- `Actor` gains `User` and `SchemaAgent { run_id: String }`.
- `EventKind` gains `UserPrompt { text }`. `ChangeSetProposed` gains `prompt_seq: Option<u64>`, `attempt: u32` and `assumptions: Vec<String>`, all with `#[serde(default)]`.

### Task 2: `nocodo-agent` base

- Dependencies:
  - `rig-core` and `rig-agent` 0.43 with the Anthropic provider;
  - `tokio`, `schemars`, `serde_json`, `thiserror`;
  - `uuid` (v7) for run IDs.

  Check the Cranelift build. If it fails, add a per-package override and record it in `AGENTS.md`.
- `build_model(&ModelConfig)`:
  - reads `ANTHROPIC_API_KEY` from the environment;
  - if the key is missing, returns an error saying exactly which variable to set.
- `AgentLog`: appends JSONL to `<project>/.agent-logs/<run_id>.jsonl`. Each line carries `run_id`, `role`, `model`, `at` and a `type`:
  - `run_started`: the input (preamble and user message, in full);
  - `completion`: the raw request and response, or at least the `submit` tool arguments; token usage; duration; attempt number. Use Rig's `AgentHook` (`on_completion_call`, `on_model_turn_finished`, `on_invalid_tool_call` in `~/Projects/rig/crates/rig-agent/src/agent/hook.rs`) to capture the real exchange.
  - `invalid_output`: extraction failures and retries;
  - `run_finished`: the typed output or the error.
- Generic helper: `run_extractor::<T>(ctx: &AgentContext, preamble, input) -> Result<T>`. It wires the model, the hook and the log. No role-specific logic.

### Task 3: `nocodo-agent-schema`

- Output type: `SchemaProposal { change_set: ChangeSet, assumptions: Vec<String> }`.
- The preamble lives in `src/preamble.md` (loaded with `include_str!`) so it's easy to review. It covers:
  - the role: turn the user's description into a change set against the current schema;
  - the conventions: `id`, `created_at` and `updated_at` are implicit; money is `money`, dates are `date`/`date_time`; names are snake_case plural; descriptions are plain language and required;
  - modeling rules: relations through `references`; lookup tables for repeated values such as categories or people; no JSON, key-value or "misc" columns;
  - evolution rules: prefer additive changes; never rename or drop unless the prompt clearly asks; columns added to existing tables must be nullable;
  - assumptions: list every guess as a short plain sentence.
- Input message, built deterministically:
  - the current `schema.json` (pretty JSON);
  - the user prompt;
  - on repair attempts, the previous `ChangeSet` and the checker findings, verbatim.
- `SchemaAgent::propose(ctx, current: &SchemaSpec, prompt: &str, feedback: Option<&Feedback>) -> Result<SchemaProposal>`.

### Task 4: Orchestrator: prompt run with repair loop

- Define the trait in the orchestrator: `trait Proposer { async fn propose(&self, current, prompt, feedback) -> Result<Proposal>; }`. It returns the change set, assumptions and actor. `api` adapts `SchemaAgent` to it. The orchestrator must not depend on agent crates.
- `Project::run_prompt(prompt, &proposer, confirm_destructive, max_attempts = 3) -> Result<Outcome>`:
  1. Append `UserPrompt` (actor `User`).
  2. Loop: propose, then `Project::propose` (the 003 pipeline). On rejection, call the proposer again with the findings, up to `max_attempts`.
  3. Each attempt is its own `ChangeSetProposed`/`Rejected`/`Accepted` pair, linked by `prompt_seq` and `attempt`.

### Task 5: `nocodo-api prompt`

- `nocodo-api prompt --project <dir> [--confirm-destructive] (<text> | --file <path>)`. It creates a tokio runtime in `main`.
- Prints:
  - the accepted change set's `summary`;
  - the list of assumptions;
  - the migration path;
  - or the final findings after the last failed attempt, with exit code 1.

### Task 6: Prompt fixtures

`fixtures/prompts/<scenario>/01-initial.md` and `02-follow-up.md` for the three `PRD.md` §7 scenarios. Write them the way a fairly precise but non-developer user would: concrete nouns and relations, no SQL.

- **finance:**
  - initial: salary, household spending by category, money lent to a brother and repayments, two bank accounts;
  - follow-up: "I also want to track which family member a spend was for".
- **inventory:**
  - initial: household items, which room or storage place they're in, purchase date and price, warranty end dates;
  - follow-up: lending items to neighbours and getting them back.
- **invoices:**
  - initial: clients, invoices with line items, payments received against invoices;
  - follow-up: tax rate per line item and a client's preferred currency.

### Task 7: Tests

No live LLM calls in `cargo test`.

- Orchestrator `run_prompt` with a scripted fake `Proposer`:
  - the first attempt is invalid and the second valid: two proposals are logged, one migration is written;
  - every attempt invalid: stops at `max_attempts` with no files written;
  - a destructive proposal without confirmation is rejected.
- `AgentLog` writes well-formed JSONL. The input message builder is deterministic (a snapshot test).
- Loading an epic 003 `project.json` without `models` still works.

## Out of scope

- Questions to the user, and a PM agent.
- Reading `inputs/` sample files.
- Applying migrations, `data.sqlite` (epic 006).
- `rig-cassette` replay (epic 008).
- Providers other than Anthropic.

## Checkpoint

Manual session. The tester exports `ANTHROPIC_API_KEY` and runs, for each `<s>` in `finance inventory invoices`:

```bash
cargo run -q --bin nocodo-api -- init --project projects/<s>
cargo run -q --bin nocodo-api -- prompt --project projects/<s> --file fixtures/prompts/<s>/01-initial.md
cargo run -q --bin nocodo-api -- prompt --project projects/<s> --file fixtures/prompts/<s>/02-follow-up.md
```

Plus one churn run: `init --project projects/finance-2`, then the same finance initial prompt.

The tester may also try their own prompts in extra projects.

- [ ] `cargo build/test/clippy/fmt` pass.
- [ ] The reviewer reads every project folder and records in Task Notes:
  - **Schema quality:** real tables and foreign keys, conventions followed, no blobs or key-value modeling. Would a developer call it reasonable?
  - **Additive evolution:** each follow-up produced an additive migration with no drops or renames.
  - **Repairs:** how many attempts each run needed, and which checker codes fired.
  - **Assumptions:** are they the right things to ask a user about? These feed the question epic.
  - **Churn:** how much `finance` and `finance-2` differ.
  - **Debuggability:** can each table be traced to a prompt through `events.jsonl` and `.agent-logs`?
  - Token usage and latency per run.
- [ ] Go/no-go recorded: proceed to epic 005 (hardening), or change the types, preamble or model first.
- [ ] Task Notes added below, and `Status: Done`.

## Task Notes

- **Built:** Tasks 1–7 as specified: `schemars` on the change types, per-role model config, `nocodo-agent` (Rig extractor, `AgentLog` JSONL, `run_extractor`), `nocodo-agent-schema` with `preamble.md`, `Project::run_prompt` with the repair loop behind the `Proposer` trait, `nocodo-api prompt`, and the prompt fixtures.
- **Checkpoint:** run with `claude-haiku-5-5` on the tester's own prompts instead of the fixtures: travel (twice, for churn), small farm finance and cofounder matching, the last two with a follow-up each. The `finance`, `inventory` and `invoices` fixtures were not run; epic 005's checkpoint covers them. Full review: `epics-notes/004-schema-agent.md`.
- **Results:** every run was accepted at attempt 1, and both follow-ups were purely additive. Runs trace cleanly from `events.jsonl` to `.agent-logs`.
- **Surprising:** 3 of 6 runs had a hidden retry inside Rig (`submit` arguments sent as strings), invisible in `events.jsonl`. Schema weaknesses: stored computed results (travel `settlements`), polymorphic references (cofounder `item_similarities`), a schema change for a data-only request (farm `is_business_account`), and no type for fractional numbers.
- **Go/no-go:** go to epic 005, extended with the findings above.
