# Epic 007 notes

Review of the implementation and the checkpoint session, run by Claude on 2026-10-11. The destructive flow passes; the plain-prompt runs found three problems and a few weaker points.

Gate: `cargo build`, `test`, `clippy -D warnings` and `fmt --check` all pass.

Epic status: not set to `Done`, and no Task Notes in the epic yet. Both wait for the tester's decision on findings 1–3.

## Code review

The work matches the epic. Deviations listed by the implementer (all reasonable, needed to carry answers and flags through the pipeline):

- `continue_pending` takes two extra parameters, `confirm_destructive` and `max_attempts`.
- `check_questions` takes an `AnsweredQuestion` with an `in_this_prompt` flag.
- `words()` and `column_words()` moved into `nocodo-types`.
- The accepted output of `prompt` now reads "Accepted → migrations/…", the same as `changes`.

Points found while reading the code, none blocking:

1. **`field` is in `TECH_WORDS`.** A farm question about "each field" is rejected as technical, although farm users say it about crops. It did not happen in the checkpoint runs. If `questions_rejected` shows it later, consider dropping the word.
2. **Plural matching only strips a trailing "s".** "indexes" and "queries" slip through the word check.
3. **Dropped duplicates leave gaps in IDs.** The user may see `q1, q3`. Deliberate (`next_number` counts dropped IDs), but odd to look at.
4. **A failure on a repair attempt loses the findings.** If `prompt_failed` happens on attempt 2 or later, `continue` re-runs with no feedback, so the agent may repeat the rejected output.
5. **A questions round that uses up `max_attempts` cannot be continued.** The last event is `questions_rejected`, so `continue` says "Nothing to continue." The user must run `prompt` again, which records a second `user_prompt`.
6. **`answer --defaults` with nothing open** prints "Answers recorded. Continuing." and then "Nothing to continue."
7. **Provider retries are capped at 3 per run**, not per model call. A run with several repair calls shares one budget. Matches the epic text.
8. **The provider-retry unit test was not verified.** Check what `crates/nocodo-agent/src/extract/tests.rs` covers; if Rig cannot script a 429 without a network call, say so in the Task Notes.

## Runs

Model: `neuralwatt/glm-5.3-flash` (from `project-base.json`). Projects are in `.projects/`.

### Destructive confirmation (`q-finance-dev`, no LLM) — pass

`01-initial`, `sample-rows`, `04-drop-notes.json`, `questions`, `answer q1=1`, `inspect`.

- One blocking question was asked and nothing was written until the answer was yes.
- After "yes": `transactions` has no `notes`, and the other 12 sample rows are intact.
- The backup `before_0002_remove_payment_notes.sqlite` still has all 3 notes.
- `events.jsonl` tells the story: proposal, question, answer, acceptance, apply, check.
- Wording problem: the question reads "…the notes you have entered on each transaction (Anything worth remembering about this payment). Continue?". The column description is pasted in as it is and reads oddly.

### Plain prompts

| Scenario | Project | Initial questions | Follow-up questions | Result |
|---|---|---|---|---|
| finance | `q-finance` | 2, none blocking; answered over two `answer` commands, resumed only after the second | 0 | Additive; 18 rows survived |
| inventory | `q-inventory` | **0** | 0 | See finding 1 |
| invoices | `q-invoices` | 2, none blocking | 0 | Additive; 12 rows survived |
| farm | `q-farm` | 3, none blocking | 0 | See finding 2 |

- At most 8 questions per initial scenario in all four.
- No `questions_rejected`, no `prompt_failed`, no duplicate `user_prompt` or `change_set_proposed`.
- I saw no technical word in any question or choice.
- Every follow-up made a backup, and row counts held in every scenario.

## Findings

1. **Inventory asked nothing.** The epic calls a zero-question initial run with a plain prompt a finding. The agent silently decided: one place per item, one loan per item, and a free-text note for quantities. Those are the "one or many" decisions this epic wants asked.
2. **The farm follow-up should have been data-only and was not.** "We opened a business account for the farm." produced `0002_add_accounts`: a new `accounts` table plus `account_id` on `expenses` and `earnings`. The expected result was no schema change and no questions. Epic 005 already set this rule ("new entries are data, not schema"), but the new account was treated as a new kind of thing.
3. **The finance summary promises a computed value.** The change-set summary says "What he still owes is worked out from each loan and the repayments made against it." This is the kind of promise the epic's new assumption rules forbid, but the rules only cover `assumptions`, not the summary.

Findings 1 and 2 are prompt-quality problems in the Schema agent's preamble. Finding 3 needs the computed-value rule extended to the summary.

## Weaker points

- The questions that were asked are mostly shallow yes/no ("Do you want categories?"). They did shape the schema, but they are not strong questions.
- The follow-ups asked nothing even where choices were open. In the invoices currency follow-up the agent assumed both invoices and payments carry a currency, instead of asking.
- `continue` after a real failed model call was not exercised live, because no call failed. Only the scripted tests cover it.

## To decide

- Whether to fix findings 1–3 (a small preamble change) before closing the epic, or close it and carry them to the next one.
- Then write the Task Notes in the epic and set `Status: Done`.
