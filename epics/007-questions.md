# Epic 007: Questions

**Status:** Not started
**Goal:** The Schema agent asks the user what it needs to know, in domain language, with typed choices, instead of guessing silently. Plain code checks every question for technical words, orders them, records them and the answers in `events.jsonl`, and resumes the run once every question is answered. Removing data needs the user's explicit confirmation, asked as a question in domain terms. This epic tests PRD claim 3 and success criteria 1 and 2: with plain-language prompts, at most 8 questions per initial scenario, and none of them technical.

**Prerequisite:** Epic 006 (closed; checkpoint review in `epics-notes/006-runtime.md`).

---

## Context

- The Schema agent can't ask anything today. Its preamble says "The user cannot be asked questions right now", and every guess goes into `assumptions` on `change_set_proposed`, accepted without anyone seeing it.
- Epic 006's review found assumptions that promise things the system doesn't do. Each of these is really a question the agent should have asked:
  - computed values: "the unpaid amount of an invoice is worked out by subtracting all its payments";
  - starting entries: "the three kinds the user mentioned are added as the first kinds", "the family members … are entered into the existing list of people";
  - a follow-up first judged "already covered", then answered by replacing a detail instead of adding one.
- Destructive change sets are rejected with `destructive_not_confirmed` unless the developer passes `--confirm-destructive` (`nocodo-checker/src/rules.rs`). Nothing can ask the user, so `fixtures/change-sets/finance/04-drop-notes.json` has never been used on top of data.
- The prompt fixtures (`fixtures/prompts/*/`) are detailed on purpose: they name every detail so the epics 004–006 checks had something to work with (PRD §8). With them the agent has little to ask, so claim 3 needs new, plain-language prompts.
- Two event-log gaps from epic 006 move here (Task 0), because the question flow adds more model calls per session:
  - a failed model call leaves a lone `user_prompt` in `events.jsonl`, and the retry adds a second identical one;
  - a provider error marked `retryable` (such as a 429) ends the command.
- The word list the summary test uses (`crates/nocodo-runtime/tests/fixtures.rs`) lives in a test. Questions need the same list at run time.
- The `nocodo` CLI comes in epic 008. Until then the `nocodo-api` subcommands present the questions and record the answers, as they stand in for the CLI everywhere else.

## Decisions

Decided by the tester on 2026-10-11.

- **Questions live only in `events.jsonl`.** No separate `questions.jsonl`. Questions and answers are typed events, and a question's status is derived from the log, like `schema.json`. Why: one source of truth, and a reviewer reads one file.
- **One agent question round per prompt.** A Schema agent call returns either questions or a change set. Once a round is answered, the next call must return a change set. Why: it keeps the count per scenario bounded and easy to measure, and needs no `depends_on`. The rule covers the agent's questions only: the destructive confirmation is a separate `system` round and may follow an answered agent round in the same prompt.
- **Choices only; no typed answers.** Every answer is one of the agent's choices. The user can't type their own answer in this epic. Why: typed answers are free text the agent must interpret, and the choices should cover what matters for storage.
- **Answers come in through subcommands, possibly over several invocations.** `nocodo-api answer` takes `<question id>=<choice number>` arguments. Each answer is recorded as it arrives; the round stays open until every question in it is answered.
- **Answering the last open question resumes the run automatically.**
- **`nocodo-api continue` retries whatever failed last, without recording anything twice.** If the latest prompt run ended in `prompt_failed` (the first model call, or the call after the answers), `continue` runs it again under the same `user_prompt`. Running `prompt` with the same text instead records a new prompt. Why: one prompt in the log, one `user_prompt` event.
- **A confirmed removal continues the existing proposal.** After "yes", the checks run again on the same proposal, and `change_set_accepted` (or `change_set_rejected`) points at the original `change_set_proposed`. No second copy of the proposal is recorded.
- **Choices are numbered from 1 everywhere.** The command (`q1=1`), the stored `default` and `choice`, and every event detail use the same number for the same choice. Why: the log and the command never disagree, and a reviewer reads the number as written.
- **The agent writes question text and choices; plain code does everything else:** IDs, checks, ordering, removing duplicates, recording, and deciding when to resume. Why: PRD §3 replaces a PM agent with a deterministic presenter.
- **The presenter logic lives in the orchestrator for now**, as pure functions the api calls. The `cli` will call the same code through the api in epic 008. Why: the orchestrator owns `events.jsonl`, and the api must stay thin.
- **`assumptions` stay**, for minor guesses not worth one of the 8 questions, under the new rules in Task 3. Why: removing them would hide guesses again; the reviewer checks that no assumption should have been a question.
- **A zero-question initial run is acceptable.** Forcing questions would make the agent invent them. The reviewer notes zero-question runs; with the plain prompts, they would be a finding in themselves.
- **The destructive-change confirmation is generated by plain code**, from the change set and the descriptions in `schema.json`, not written by the agent. Why: it must be exact about what will be forgotten, and the developer `changes` path needs it too.
- **The confirmation is asked the same way for `changes` and `prompt`, and only for a change set that passes everything else.** Every other rule and the dry run run first; a change set with any other finding is rejected as today, without a question. In a prompt run, removing data is no longer a finding sent back to the agent to fix: it becomes the user's decision. Why: there is no point asking the user to confirm a change set that would be rejected anyway, and the agent must not quietly rework a removal the user asked for.
- **`TECH_WORDS` is the list in Task 1.** Words like "record", "type", "key" and "list" stay allowed because users say them about their own things ("type of expense", "keep a list"). "Table" will sometimes be furniture; the rare false finding is accepted.
- **Provider retries: 3 tries, waiting 2 s, 5 s and 10 s.** Enough to ride out the `concurrent_budget_exceeded` seen in epic 006 without making a broken setup slow to fail.
- **The plain prompt texts in Task 6 are a starting point.** Any that still sounds like a specification gets rewritten, so the agent has something real to ask.

## Tasks

### Task 0: Failed and retried model calls (`nocodo-types`, `nocodo-agent`, `nocodo-orchestrator`)

- New event `PromptFailed { prompt_seq, error, run_id, detail }` (actor `System`), appended when the proposer fails for any reason (provider error, invalid output after every retry). `detail` example: "The model call for prompt 8 failed: 429 concurrent_budget_exceeded (retried 3 times). Nothing was proposed; see .agent-logs/<run_id>.jsonl. Run `nocodo-api continue` to try again."
- `nocodo-agent` retries an error the provider marks retryable (HTTP 429, 5xx, connection reset) up to 3 times, waiting 2 s, 5 s and 10 s, honouring a `retry-after` header when present. Each wait is a `provider_retry` line in `.agent-logs`, and `run_finished` gains `provider_retries`.
- A non-retryable error fails at once, as today.

### Task 1: Types (`nocodo-types`)

- `Question`:
  - `id: String`: `q1`, `q2`, … one sequence per project, assigned by the orchestrator.
  - `asked_by: String`: `schema` (the agent) or `system` (plain code, such as the destructive confirmation).
  - `text: String`: the question, in domain language.
  - `choices: Vec<String>`: 2 to 5 options.
  - `default: Option<usize>`: the number (from 1) of the choice used when the user accepts the defaults. `None` only for blocking questions.
  - `blocking: bool`: blocking questions must be answered explicitly; `--defaults` skips them.
  - `why: String`: for the reviewer, not shown to the user: what in the change set depends on the answer. Technical words are allowed here.
- `Answer { question_id, choice: usize, by_default: bool }`. `choice` is numbered from 1, as typed in `answer`.
- New `EventKind` variants:
  - `QuestionsAsked { prompt_seq: Option<u64>, proposal_seq: Option<u64>, questions: Vec<Question>, detail }`. It points at the prompt (agent questions) or at the proposal (destructive confirmation) the round belongs to. Actor: `SchemaAgent` or `System`.
  - `QuestionAnswered { answer: Answer, detail }`, actor `User`. Example detail: "q2 answered: \"Yes, a spend can be for the whole family\" (choice 1 of 3)."
  - `QuestionsRejected { prompt_seq, findings }`: a round that failed the question checks (Task 2), so a reviewer sees what was caught.
  - `QuestionsDropped { question_ids, detail }`, for duplicates removed by the presenter (Task 4).
- `TECH_WORDS`: the shared list of words that must never appear in text shown to the user. It starts with the summary test's list (table, column, row, sql, database, schema, id) plus api, json, migration, query, index, foreign key, nullable, integer, boolean, string, field. Matching ignores case and works on whole words. The summary test uses this list too.
- Old event logs load unchanged.

### Task 2: Checker rules for questions (`nocodo-checker`)

`check_questions(questions, answered_so_far) -> Vec<Finding>`, run on every round before anything is recorded:

- `technical_word`: the text or a choice contains a word from `TECH_WORDS`. The finding names the word.
- `choices_count`: fewer than 2 or more than 5 choices; `duplicate_choice`: two choices are the same once trimmed and lowercased.
- `bad_default`: `default` is outside 1 to the number of choices, or missing on a non-blocking question.
- `too_many_questions`: more than 8 questions in the round.
- `already_answered`: the text matches an answered question (same text once normalized). The finding quotes the earlier answer.
- `questions_after_answers`: the call that follows an answered round returned questions again (Decisions: one round).

Findings go back to the agent through the existing repair loop (`max_attempts`), exactly like change-set findings. The orchestrator's `Proposer` and `Feedback` carry the agent's whole reply (questions or change set), so a rejected question round is sent back with its questions, not only a previous change set. A rejected round is recorded as `QuestionsRejected`, so a reviewer sees, for example, the technical word that was caught.

### Task 3: The Schema agent asks (`nocodo-agent-schema`)

- The agent's output becomes `SchemaReply { questions: Vec<QuestionDraft>, change_set: Option<ChangeSet>, assumptions: Vec<String> }`. Exactly one of `questions` (non-empty) and `change_set` must be set; the orchestrator rejects other shapes with a finding. `QuestionDraft` is `Question` without `id` and `asked_by`. One struct rather than a tagged enum, so strict tool schemas keep working (epic 005 Task 7).
- `build_input` gains an "Answers so far" section: every answered question for this project (not only this prompt), with the chosen text and whether it was a default. The agent must not ask these again.
- Preamble, new section "Asking the user", replacing the "Assumptions" paragraph that says the user can't be asked:
  - Ask only when the answer changes what is stored: which things are separate, whether something can have one or many, whether a detail is needed at all. Don't ask about wording, names or anything you can safely choose yourself.
  - Ask everything you need in one round, at most 8 questions, the most important first. Mark a question blocking when no sensible default exists.
  - Write as the user speaks: about their money, items, clients, family. Never mention how anything is stored. Give 2 to 5 short choices that together cover the sensible answers, since the user can only pick one of them, and a default unless the question is blocking.
  - Fill in `why` for the reviewer.
  - Once answers arrive, return a change set and never ask again in that run.
  - Returning a change set with no questions is fine when nothing needs asking.
- Assumptions keep the minor guesses that weren't worth a question. New rules:
  - Assumptions describe what will be remembered, never what will be worked out or added for the user. Never say something will be calculated, totalled, pre-filled or entered for them.
  - A follow-up that asks to track more adds details or lists; it never replaces or removes existing ones. If you think the request is already covered, ask instead of returning an empty change set.

### Task 4: The question round in the pipeline (`nocodo-orchestrator`)

- `run_prompt`:
  - When the agent returns questions, run `check_questions`. If they pass, assign IDs, order them (blocking first, then in the agent's order), drop exact duplicates of each other (`QuestionsDropped`), append `QuestionsAsked` and return `PromptOutcome::Waiting { questions }`.
  - When it returns a change set, continue as today.
- `Project::open_questions()`: the questions still waiting, derived from the log.
- `Project::answer(answers) -> Result<AnswerOutcome>`: validates each answer (a known open question, a valid choice) and appends `QuestionAnswered`. A question already answered is refused. Answers may arrive over several calls; while any question in the round is open it returns `StillWaiting { open }`, and once none is left it returns `ReadyToContinue`. It never calls the model itself.
- `Project::continue_pending(&proposer)`: finds what the latest prompt run is waiting on in the log and resumes it:
  - a fully answered agent round: re-runs the prompt with all answers in the input (Task 3), under the original `user_prompt` (`prompt_seq`); attempt numbers carry on from the earlier attempts;
  - a fully answered confirmation round: on yes, re-runs the checks and the dry run on the same proposal with `confirm_destructive` on (the data may have changed since, for example through `sample-rows`), then appends `ChangeSetAccepted` or `ChangeSetRejected` with the original `proposal_seq`. On no, appends `ChangeSetRejected` for that proposal with finding `user_declined`. No new `ChangeSetProposed` is recorded;
  - a prompt run that ended in `PromptFailed`: repeats the failed model call (the first one, or the one after the answers) under the same `user_prompt`;
  - otherwise it returns `NothingToContinue`.
- **Destructive confirmation:**
  - When a change set is destructive and not confirmed, the orchestrator no longer rejects it with `destructive_not_confirmed`. It runs every other rule and the dry run as if confirmed:
    - any other finding rejects the change set as today, with no question. In a prompt run those findings go back to the agent through the repair loop; `destructive_not_confirmed` never does;
    - when everything else passes, it appends a `system` question with choices "Yes, forget it" / "No, keep it" (blocking, no default), pointing at the `ChangeSetProposed` event, and returns `Waiting`. Nothing is written to `migrations/` or `specs/schema.json` until the answer is yes.
  - The same applies to `changes` and to `prompt`, including a prompt whose agent questions were already answered: there the confirmation is a second round, allowed because it is asked by `system`.
  - The text is built from `schema.json` descriptions, one line per removed thing. For example: "This will forget the notes you have entered on each expense (A short free-text note about the spend). Continue?" A dropped list reads "This will forget every <list name in words> you have entered, and everything recorded about them."
  - It uses the same words helper as the `inspect` summary, and the text must pass `technical_word`.
  - `--confirm-destructive` on `changes` still skips the question, for scripted runs.
- `prompt` and `changes` refuse to start while a round is waiting: "2 questions are waiting (q3, q4). Answer them with `nocodo-api answer`, or see them with `nocodo-api questions`."

### Task 5: Commands (`api`)

- `prompt` and `changes`: when the outcome is `Waiting`, print the questions as `questions` does and exit 0.
- `nocodo-api questions --project <dir>`: prints each open question with its ID and numbered choices, the default marked. Blocking questions come first and are marked "needs an answer". With nothing open: "No questions waiting."
- `nocodo-api answer --project <dir> q1=2 q3=1`:
  - each argument is `<question id>=<choice number>`; anything else is refused with a message naming the argument;
  - it may answer some of the open questions; the rest stay open for a later `answer`, and it prints which are still waiting;
  - `--defaults` accepts the default for every open non-blocking question not answered in the same command;
  - when the round is complete, it calls `continue_pending` and prints the result as `prompt` does.
- `nocodo-api continue --project <dir>`: calls `continue_pending` and prints the result as `prompt` does, or says "Nothing to continue."
- The api stays thin: no validation, ordering or wording in it.

### Task 6: Plain-language prompt fixtures (`fixtures/prompts`)

New `fixtures/prompts/<s>/plain-01.md` (initial) and `plain-02.md` (follow-up) for `finance`, `inventory`, `invoices` and `farm`. They are short, written as a non-technical user speaks, and leave the structure open. The existing detailed fixtures stay for regression runs. Starting text, to be rewritten where it still sounds like a specification:

| Scenario | `plain-01.md` | `plain-02.md` |
|---|---|---|
| finance | I want to keep an eye on my family's money. My salary comes in every month, we spend on the household, and sometimes I lend money to my brother. We have two bank accounts. | I'd also like to know who in the family each spend was for. |
| inventory | I want to know what things we have at home and where they are. Sometimes I lend things to the neighbours. | I want to know when warranties run out. |
| invoices | I do some consulting and want to keep track of who I bill and whether they have paid me. | Some clients pay me in other currencies. |
| farm | We run a small pig and chicken farm as a family. I want to see what we spend, what we earn and money moving between us. | We opened a business account for the farm. |

The farm follow-up is a data-only request (epic 005): the right answer is no schema change, and the agent should not ask about it either.

### Task 7: Tests

No live LLM calls in `cargo test`.

- Types: `Question`, `Answer` and the new events round-trip. Old logs load.
- `TECH_WORDS`: whole-word matching ("identity" and "rowing" are not flagged; "ID" and "Table" are). The summary test now uses the shared list.
- Checker: one test per question rule, passing and failing.
- Agent: `build_input` snapshot with an "Answers so far" section, and a preamble snapshot.
- Orchestrator, with a scripted `Proposer`:
  - questions, then answers, then a change set: events in order (`user_prompt`, `questions_asked`, `question_answered` ×n, `change_set_proposed`, `change_set_accepted`). The second call's input holds the answers.
  - answers over two `answer` calls: the first returns `StillWaiting` and the proposer is not called; the second resumes the run.
  - answering an already-answered question, an unknown ID, or an out-of-range choice is refused.
  - a round with a technical word is rejected, repaired and recorded.
  - questions after answers are rejected (`questions_after_answers`).
  - `--defaults` skips blocking questions; `prompt` refuses while a round waits.
  - a failing proposer gives `prompt_failed`, and `continue_pending` succeeds on the next try with no second `user_prompt`; the same for a failure on the call after the answers.
  - a confirmed removal appends `change_set_accepted` with the original `proposal_seq` and no second `change_set_proposed`.
  - choices are numbered from 1: `q1=1` picks the first choice, `q1=0` is refused, and the event detail says "choice 1 of 2".
  - destructive: `04-drop-notes.json` on the finance spec gives a confirmation question whose text passes `technical_word`. "Yes" accepts and applies it, "no" gives `user_declined`, and `--confirm-destructive` skips the question.
  - destructive in a prompt run: a scripted proposer that answers its questions with a change set dropping a detail gets a confirmation round after the answered agent round, and the proposer is not called again. A destructive change set that also fails another rule is rejected without a question, and the agent's feedback does not contain `destructive_not_confirmed`.
- Agent retries: a scripted model returning 429 twice, then a valid reply, records 2 `provider_retry` lines (if Rig's API allows a scripted model without a network call; otherwise say so in Task Notes).
- End to end through the binary: `init`, `changes 01`, `sample-rows`, `changes 04-drop-notes.json`, `questions`, `answer q1=1`, `inspect`. The notes detail is gone, the other rows are intact, and the backup still has the notes.

### Task 8: Docs

- PRD §5: remove `questions.jsonl`; questions live in `events.jsonl`.
- PRD §3 table: the question presenter is plain code in the orchestrator, presented by `nocodo-api` until epic 008.
- AGENTS.md, manual test loop: add the `questions` / `answer` / `continue` commands.

## Not in this epic

- The `nocodo` CLI, Axum, and `rig-cassette` replay (epic 008).
- Answers the user types in their own words (an `allow_other` choice).
- Adding starting entries the user names (such as "groceries, rent, school fees"). Adding them needs data entry.
- Questions that depend on other questions (`depends_on`), more than one round per prompt, and questions asked later, outside a prompt run.
- Changing an answer once it is recorded.
- Questions from anything but the Schema agent and the destructive confirmation.
- An LLM reviewer of question wording. The deterministic word check is the gate; the reviewer reads the rest at the checkpoint.
- Carried over from epic 005 and 006:
  - checker advice for a redundant text column next to a new reference;
  - a live run of OpenAI strict mode;
  - stringified tool arguments from `glm-5.3-flash` (the repair handles them; a model choice, not code).

## Checkpoint

Manual session, with the model in `project-base.json`.

First, the deterministic destructive confirmation (no LLM):

```bash
P=.projects/q-finance-dev
cargo run -q --bin nocodo-api -- init --project $P
cargo run -q --bin nocodo-api -- changes --project $P fixtures/change-sets/finance/01-initial.json
cargo run -q --bin nocodo-api -- sample-rows --project $P
cargo run -q --bin nocodo-api -- changes --project $P fixtures/change-sets/finance/04-drop-notes.json
cargo run -q --bin nocodo-api -- questions --project $P
cargo run -q --bin nocodo-api -- answer --project $P q1=1
cargo run -q --bin nocodo-api -- inspect --project $P
```

Then the plain prompts, for each `<s>` in `finance inventory invoices farm`. The tester answers as the user would, picking non-default choices where they fit the scenario better, and in at least one scenario answers over two `answer` commands:

```bash
cargo run -q --bin nocodo-api -- init --project .projects/q-<s>
cargo run -q --bin nocodo-api -- prompt --project .projects/q-<s> --file fixtures/prompts/<s>/plain-01.md
cargo run -q --bin nocodo-api -- questions --project .projects/q-<s>
cargo run -q --bin nocodo-api -- answer --project .projects/q-<s> q1=… q2=…
cargo run -q --bin nocodo-api -- sample-rows --project .projects/q-<s>
cargo run -q --bin nocodo-api -- prompt --project .projects/q-<s> --file fixtures/prompts/<s>/plain-02.md
# answer any follow-up questions the same way
cargo run -q --bin nocodo-api -- inspect --project .projects/q-<s>
```

- [ ] `cargo build/test/clippy/fmt` pass.
- [ ] The reviewer confirms, from the project folders alone:
  - **Few questions:** at most 8 per initial scenario. Record the count per scenario and per follow-up, how many were blocking, and any zero-question initial run.
  - **No technical questions:** every question and choice reads as domain language. Note any `questions_rejected` for a technical word, and any wording the word list missed.
  - **Useful questions:** each answer visibly shaped the accepted change set (follow `why` to the change). Note questions whose answer changed nothing, and questions where none of the choices fit what the user would say.
  - **No re-asking:** the follow-up asks nothing already answered.
  - **Assumptions:** none promise computed values or pre-filled entries, and none should clearly have been a question.
  - **Data-only follow-up:** the farm follow-up changes no schema and asks nothing.
  - **Destructive confirmation:** in `q-finance-dev`, the question names exactly what is forgotten in domain words; after "yes", the notes are gone from `data.sqlite`, the latest backup still has them, and every other sample row is intact.
  - **Events:** `events.jsonl` tells the story without the terminal: prompt, questions, each answer with its choice text, proposal, acceptance, apply. Answers split over two commands resume only after the last one. Any failed model call appears as `prompt_failed` with its run ID, and was retried with `continue`: no duplicate `user_prompt` or `change_set_proposed`. Choice numbers in the events match the numbers typed in `answer`.
  - Everything epic 006 checks still holds: data survives, additive follow-ups, backups.
- [ ] Task Notes added below, and `Status: Done`.

---

## Task Notes
