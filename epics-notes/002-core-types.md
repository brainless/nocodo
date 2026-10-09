# Epic 002 notes

Points raised after the subagents implemented Tasks 1–6.

- **Strictness gap:** `Actor` and `EventKind` (`crates/nocodo-types/src/event.rs`) don't reject unknown fields, because only `SchemaChange` has `deny_unknown_fields` among the tagged enums. The Task 4 agent said serde doesn't support it on tagged enums, but the Task 6 agent showed it works on `SchemaChange`. Adding `deny_unknown_fields` to the `#[serde(tag = "type", ...)]` attributes of those two enums should be a small change. Not made yet.
- **Extra `deny_unknown_fields`:** the Task 2 and 3 agents added it to `SchemaSpec`, `SchemaChange` and `ChangeSet`, which the epic doesn't list.
- **Epic file not updated:** `epics/002-core-types.md` still has no `Status: Done` and no Task Notes. The checkpoint is left for the tester to evaluate.
