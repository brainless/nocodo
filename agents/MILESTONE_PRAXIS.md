# Milestone: Praxis

A provenance-aware vocabulary crate (`nocodo_praxis`) that makes the gap between project spec and generated code explicit, typed, and verifiable.

## Context

- nocodo's PO and PM agents produce text artifacts (epics, tasks, project notes). The connection between requirements and generated code is implicit.
- `nocodo_praxis` encodes business logic structure as Rust types. The spec crate *is* the runtime (Path C from RUNTIME.md).
- RustEngineer will eventually consume praxis types to generate project-specific logic (permissions, state machines, controllers).
- Design document: `agents/RUNTIME.md`

## Crate Name

`nocodo_praxis` — "the process by which an idea is enacted." Published to crates.io eventually.

---

## Phase 1: Scaffold `nocodo_praxis`

Minimal crate with the primitives that every spec needs. No state machines or entities yet — add when the first spec demands them.

### Tasks

- [ ] Create `nocodo_praxis/` as a new crate in the workspace (add to root `Cargo.toml`)
- [ ] Implement `nocodo_praxis::primitives`:
  - `AtLeastOne<T>` — non-empty collection, structurally enforces "at least one responsible party"
  - `Unresolved<T>` — first-class incompleteness (`Resolved`, `Pending`, `Blocked`)
- [ ] Implement `nocodo_praxis::provenance`:
  - `Provenance` enum — `Conversation`, `JiraTicket`, `GitCommit`, `ConfluencePage`, `File`, `Inferred`
  - `PrdValue<T>` — value + provenance carrier
- [ ] Implement `nocodo_praxis::auth`:
  - `RoleId`, `PermissionId`, `PersonaId` newtypes
  - `RoleSemantics` — `Flat`, `Inherits`, `Union`
  - `Role`, `Permission`, `UserPersona` structs
  - `ImplicitRole` — `AnyAuthenticated`, `AnyUser`
- [ ] Add basic unit tests for `AtLeastOne` and `Unresolved` methods
- [ ] Crate compiles clean with `cargo check -p nocodo_praxis`

### Not in scope

- State machines (`State`, `Transition`, `Transitions`)
- Entities (`Entity`, `Field`, `Assignee`)
- Tree-sitter queries
- Any agent integration

---

## Phase 2: Validate with a Hand-Written Spec

Write a spec crate by hand (using a coding agent) that depends on `nocodo_praxis`. The todo app from RUNTIME.md §4 is the candidate. Goal: discover vocabulary gaps and ergonomic issues before building agents.

### Tasks

- [ ] Create a `todo_app_spec/` crate (or similar) that depends on `nocodo_praxis`
- [ ] Hand-write the todo spec: personas, permissions, roles, task state machine
- [ ] Identify missing types or awkward patterns in `nocodo_praxis`
- [ ] Iterate on `nocodo_praxis` API based on findings
- [ ] Spec crate compiles and types compose correctly

### Success criteria

- The todo spec compiles against `nocodo_praxis` with zero `Unresolved` workarounds (i.e., the vocabulary is sufficient)
- Any gaps found are documented as `nocodo_praxis` issues or added to Phase 3+

---

## Phase 3: Add State Machines and Entities

Expand `nocodo_praxis` with the remaining core types from RUNTIME.md §3.4–3.5.

### Tasks

- [ ] Implement `nocodo_praxis::statemachine`:
  - `StateId`, `TransitionCondition`, `Transition`, `Transitions`, `State`
- [ ] Implement `nocodo_praxis::entity`:
  - `EntityId`, `Assignee<Id>`, `Field`, `Entity`
- [ ] Update the todo spec to use state machine and entity types
- [ ] Add unit tests for state machine soundness helpers (e.g., terminal state detection)

---

## Phase 4: Spec Agent Integration (Future)

Wire PO/PM output into spec crate generation. Close the gap between chat-based requirements and typed specs.

### Tasks (not yet detailed)

- [ ] Spec agent that reads PO project notes + PM artifacts and emits a spec crate
- [ ] Validation agent (tree-sitter queries + rustc)
- [ ] Clarification loop: `Unresolved` nodes → structured questions back to user via PO
- [ ] RustEngineer consumes spec crate for codegen

---

## Principles

1. **Grow from evidence** — every new type in `nocodo_praxis` must be motivated by a real spec that needed it (§7 of RUNTIME.md)
2. **Spec is runtime** — no separate IR, no sync problem
3. **Incompleteness is first-class** — `Unresolved` blocks codegen, not approximates around it
4. **Inference is visible** — `Provenance::Inferred` marks every LLM assumption
