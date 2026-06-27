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

- [x] Create `nocodo_praxis/` as a new crate in the workspace (add to root `Cargo.toml`)
- [x] Implement `nocodo_praxis::primitives`:
  - `AtLeastOne<T>` — non-empty collection, structurally enforces "at least one responsible party"
  - `Unresolved<T>` — first-class incompleteness (`Resolved`, `Pending`, `Blocked`)
- [x] Implement `nocodo_praxis::provenance`:
  - `Provenance` enum — `Conversation`, `JiraTicket`, `GitCommit`, `ConfluencePage`, `File`, `Inferred`
  - `PrdValue<T>` — value + provenance carrier
- [x] Implement `nocodo_praxis::auth`:
  - `RoleId`, `PermissionId`, `PersonaId` newtypes
  - `RoleSemantics` — `Flat`, `Inherits`, `Union`
  - `Role`, `Permission`, `UserPersona` structs
  - `ImplicitRole` — `AnyAuthenticated`, `AnyUser`
- [x] Add basic unit tests for `AtLeastOne` and `Unresolved` methods
- [x] Crate compiles clean with `cargo check -p nocodo_praxis`

### Not in scope

- State machines (`State`, `Transition`, `Transitions`)
- Entities (`Entity`, `Field`, `Assignee`)
- Tree-sitter queries
- Any agent integration

---

## Phase 2: Backend Helpers — Validate Praxis Types Through Use

Write helper functions in `backend/src/praxis/` of the target project (rustysolid) that consume `nocodo_praxis` types. These are pure, stateless functions callable from Controllers or Background Tasks. Goal: validate that the auth types compose well in real backend contexts — not just statically declare.

**Why backend helpers instead of a static spec crate:**
- nocodo will eventually generate code for projects built on the rustysolid template
- Writing functions that *use* the types catches ergonomic issues that static declaration misses
- Follows rustysolid's "functions over structs for stateless ops" pattern
- Functions are the exact shape of code controllers/background tasks will call

### Tasks

- [x] Add `nocodo_praxis` as a path dependency to `backend/Cargo.toml`
- [x] Create `backend/src/praxis/mod.rs`, `permcheck.rs`, `resolver.rs`
- [x] Implement `has_permission(user_roles, all_roles, required)` — checks if a set of roles grants a permission, resolving `RoleSemantics` (Flat, Inherits, Union) with cycle detection
- [x] Implement `resolve_permissions(role, all_roles)` — transitively resolves all effective permissions for a role, useful standalone for audit/logging
- [x] Add unit tests exercising all three `RoleSemantics` variants, cycle detection, and edge cases
- [x] Crate compiles clean with `cargo check`
- [x] Add `Copy` derive to `RoleId`, `PermissionId`, `PersonaId` (ergonomic gap discovered during use)

### Success criteria

- All helper functions pass their unit tests
- Cycle detection in `Inherits` chains works (infinite loop prevention)
- Any gaps in `nocodo_praxis` types are documented as issues or addressed in-place

### Not in scope

- Wiring helpers into controllers or routes (future Phase)
- DB integration — helpers are pure functions, no Diesel
- State machines / entities (Phase 3)

---

## Phase 2b: Sample Apps — Encode PRDs as Executable Specs

Create minimal crates in `~/NocodoProjects/` that encode a PRD as praxis-typed helpers + tests. Each sample app has an immutable `PRD.md` (reference, never edited) and a `src/lib.rs` that models the app's core logic using `nocodo_praxis` types. Goal: discover vocabulary gaps (state machines, entities, invariants) through real use.

Multiple sample apps may be created; each exposes gaps in the praxis vocabulary that feed back into `nocodo_praxis`.

### Sample: Todo App

- [x] Scaffold `~/NocodoProjects/todo_app/` with `PRD.md`, `Cargo.toml`, `src/lib.rs`, `tests/`
- [x] Depend on `nocodo_praxis` + rustysolid `app-backend` (lib) for `praxis` helpers
- [x] Model todo roles (`member`, `admin`), permissions, and state machine (todo → in_progress → done | cancelled) in code
- [x] Write tests that verify "does the code encode the PRD?" — e.g., admin can create tasks, member cannot
- [x] Document any missing `nocodo_praxis` types (state machines, entities) for Phase 3

### Success criteria

- All spec tests pass
- `PRD.md` is never edited after initial write — all iteration happens in code
- Any vocabulary gaps are documented as Phase 3+ tasks or `nocodo_praxis` issues

### Not in scope

- Controllers, DB, HTTP — pure functions only
- Agent integration
- UI

---

## Phase 3: Add State Machines and Entities

Expand `nocodo_praxis` with the remaining core types from RUNTIME.md §3.4–3.5, motivated by gaps found in the Todo app sample (Phase 2b) and any other sample apps.

### Tasks

- [x] Implement `nocodo_praxis::statemachine`:
  - `StateId`, `TransitionCondition`, `Transition`, `Transitions`, `State`
- [x] Implement `nocodo_praxis::entity`:
  - `EntityId`, `Assignee<Id>`, `Field`, `Entity`
- [x] Update the Todo app spec to use state machine and entity types
- [x] Add unit tests for state machine soundness helpers (e.g., terminal state detection)

---

## Phase 4: Spec Agent Integration

Wire PO/PM output into praxis-typed spec generation inside the managed project. The spec lives in the project itself (`backend/src/praxis/specs/`), not in a separate crate. The real test: can PO ask detailed enough structured questions, and can a small-model RustEngineer mode (Praxis Writer) consume structured answers to emit valid `nocodo_praxis` Rust code?

### Context

- PO and PM were created *before* the praxis-driven approach. They must be adjusted so their output serves code-generating agents, not just user-facing conversation.
- Project notes are the transport layer from PO intake → agent consumption. Currently they are free-text. They need structured JSON content (different schema per agent mode) so downstream agents (PM, Praxis Writer) can parse them deterministically.
- PM's role shifts: instead of producing user-facing epics/tasks, PM produces **structured agent tasks** whose descriptions are machine-readable for small-model code generation.
- The Praxis Writer is a new RustEngineer mode (not a separate agent crate). It follows the same pattern as `diesel_model`: extract context → build example-rich prompt → single-shot LLM call → deterministic post-processing → write to project via `schema_codegen`.
- The `schema_codegen` crate (`schema-codegen/src/lib.rs`) is the file-writing bridge. Its `write_file_atomic`, `register_model_in_mod`, etc. are the infrastructure RustEngineer uses to write generated code into the managed project (`project_path`). We extend this with praxis-aware write helpers.
- Praxis Writer needs documentation of `nocodo_praxis` types in its prompt. We extract type signatures, field docs, and usage examples from `nocodo_praxis/src/*.rs` — either via tree-sitter (like existing RustEngineer modes) or a build-time module.

---

### Phase 4a: Praxis Documentation

The Praxis Writer LLM must know what each `nocodo_praxis` type represents, what its fields mean, and how to construct it. Currently the crate has almost no doc comments — only `Provenance::Inferred` has one. We must document the crate first, then build the extraction tooling.

#### Phase 4a.0 — Add Doc Comments to `nocodo_praxis`

Every public type, field, variant, and method needs a doc comment. These are the LLM's only source of semantic knowledge — without them the prompt is just bare type signatures.

**Required documentation per module:**

| Module | Types to document |
|---|---|
| `primitives` | `AtLeastOne<T>` — struct, fields (`head`, `tail`), methods (`all`, `contains`, `len`). `Unresolved<T>` — enum, all variants (`Resolved`, `Pending`, `Blocked`), methods (`is_resolved`, `blocks_codegen`, `resolved_value`, `reason`) |
| `provenance` | `Provenance` — enum, all variants (`Conversation`, `Inferred`, `File`), method (`excerpt`). `PrdValue<T>` — struct, fields, method (`new`) |
| `auth` | `RoleId`, `PermissionId`, `PersonaId` — newtype semantics. `RoleSemantics` — enum, all variants with inheritance/union semantics. `Role` — struct, all fields (especially `permissions` vs `semantics` interaction). `Permission` — struct. `UserPersona` — struct, all fields. `ImplicitRole` — enum |
| `statemachine` | `StateId`, `TransitionCondition` — enum, every variant with when to use each. `Transition` — struct, `permitted_roles` vs `condition` distinction. `Transitions` — enum, `Terminal` vs `To` semantics. `State` — struct. Helper fns (`find_state`, `has_terminal_state`, `find_unresolved_transitions`) |
| `entity` | `EntityId`, `Assignee<Id>`, `Field`, `Entity` — structs, fields, methods |

**Required usage examples (doctests):**

Every struct that the LLM must construct needs at least one concrete construction example as a doctest. Minimum set:

- `UserPersona` — construct a persona with id, name, description, goals, pain_points, provenance
- `Permission` — construct a permission with id, description, provenance (conversation + inferred)
- `Role` — construct three roles demonstrating Flat, Inherits, and Union semantics
- `State` — construct states with Terminal and To transitions, including Unresolved conditions
- `Entity` — construct an entity with fields, states, and invariants
- `Provenance` — construct Conversation and Inferred provenances
- `Unresolved` — construct Resolved, Pending (with provenance), and Blocked variants

The todo_app spec (`~/NocodoProjects/todo_app/src/lib.rs`) is the reference for what good construction looks like.

**Tasks**

- [x] Add `//!` module-level doc to `nocodo_praxis/src/lib.rs` — what the crate is, how modules compose
- [x] Add `///` doc comments to every public item in `primitives.rs`
- [x] Add `///` doc comments to every public item in `provenance.rs`
- [x] Add `///` doc comments to every public item in `auth.rs`
- [x] Add `///` doc comments to every public item in `statemachine.rs`
- [x] Add `///` doc comments to every public item in `entity.rs`
- [x] Add doctests with construction examples for at minimum: `UserPersona`, `Permission`, `Role` (x3 semantics), `State` (x2: Terminal, To), `Entity`, `Provenance` (Conversation + Inferred), `Unresolved` (Resolved + Pending)
- [x] `cargo test -p nocodo_praxis` — all doctests pass
- [x] `cargo doc -p nocodo_praxis --no-deps --open` — verify rendered docs are complete

#### Phase 4a.1 — Documentation Extraction Tool

Build `agents/src/praxis_doc.rs` that extracts doc comments + type signatures from `nocodo_praxis` source files and formats them as prompt-safe reference text.

**Tasks**

- [x] Create `agents/src/praxis_doc.rs` — reads `nocodo_praxis/src/*.rs`, extracts:
  - `///` and `//!` doc comments
  - Struct/enum definitions (full source)
  - `impl` block methods (with doc comments)
- [x] Format output as prompt-safe text: doc comment first, then type definition, then impl methods (no markdown fences, no HTML)
- [x] Per-module reference functions: `auth_types_reference()`, `statemachine_types_reference()`, `entity_types_reference()`, `primitives_reference()`, `provenance_reference()`
- [x] Full reference: `all_praxis_types_reference()` — concatenation of all modules
- [x] Wire into RustEngineer so any mode can call these functions
- [x] Test: reference string contains doc comments + type signatures for all items listed in 4a.0

#### Not in scope

- Live reload of docs (regenerate on `nocodo_praxis` version bump)
- Tree-sitter queries for validation (Phase 4f)

---

### Phase 4b: Structured Spec Schemas (Rust)

Define Rust types for the JSON content that PO stores in `project_note.note` field. Different PO interview modes produce different schemas. These types live in `agents/src/storage/` and are used by PO to serialize, and by PM/Praxis Writer to deserialize.

#### Tasks

- [x] Create `agents/src/storage/spec_schemas.rs` — structured note content types:
  ```rust
  /// Wrapper for structured note content. Serialized to JSON in project_note.note.
  enum SpecNoteContent {
      Persona(PersonaNote),
      // Future: Permission(PermissionNote), Role(RoleNote), ...
  }

  struct PersonaNote {
      id: String,             // short identifier, e.g. "admin"
      name: String,           // human-readable, e.g. "Team Admin"
      description: String,    // 1-2 sentences
      goals: Vec<String>,     // what they need to accomplish
      pain_points: Vec<String>, // what frustrates them currently
      provenance: String,     // session_id or "inferred" tag
  }
  ```
- [x] `PersonaNote` → `UserPersona` conversion helper (used by Praxis Writer to map structured notes → praxis types)
- [x] JSON serialization/deserialization (`serde`)
- [x] Extend `record_project_note` in PO tools to accept a `content_type` parameter alongside `topic` — PO chooses `"persona"` and the handler serializes the structured data

#### Not in scope

- `PermissionNote`, `RoleNote` — added when PO gets Permission Interview mode (Phase 4c+)
- Validation of note schemas (structural correctness enforced by Rust types/deserialization)

---

### Phase 4c: PO Persona Interview Mode

A new PO mode (`modes/persona_interview.rs`) that drills deep into each user persona the project needs. This replaces the current shallow "who uses the software?" questions with structured interviews that produce `PersonaNote` records.

#### Mode Specification

- **When**: Activated after PO identifies initial entities and user types, or when the user explicitly describes personas
- **System prompt**: PO persona interview core — warm, empathetic, non-technical. Goal: extract complete `UserPersona` data for each persona. Do NOT discuss permissions/roles/access control. Record each persona as a structured note immediately.
- **Tools available**: `request_user_input`, `record_project_note` (with `content_type = "persona"`)
- **Exit**: PO calls a new tool `complete_persona_interview` when all personas are documented, returning to `requirements_gathering` mode

#### Interview Flow (per persona)

1. **Identify**: User mentions a persona → PO asks for short identifier ("What should we call this role? e.g. 'admin', 'member', 'viewer'")
2. **Describe**: "Describe [Persona] in one sentence — their role and context."
3. **Goals**: Structured question — "What does [Persona] need to accomplish?" Options generated by PO from conversation context (e.g. "Track my own tasks", "See team progress", "Manage members", ...). Multiple choice.
4. **Pain points**: Structured question — "What frustrates [Persona] without this software?" Options: "Can't see what others are working on", "Manual task assignment", "No progress tracking", ... Multiple choice.
5. **Record**: PO calls `record_project_note(topic: "spec", content_type: "persona", note: <PersonaNote JSON>)` immediately after each persona is complete
6. **Next**: PO asks "Any other types of users?" — if yes, loop to step 1 for next persona; if no, call `complete_persona_interview`

#### Tasks

- [x] Create `agents/src/product_owner/modes/persona_interview.rs`
- [x] Write `po_persona_core()` — kept inline in `persona_interview.rs` (separate tone from requirements_gathering justified this)
- [x] Write `system_prompt()` — detailed instructions for the interview flow above
- [x] Add tool `complete_persona_interview` to PO tools (`tools.rs`)
- [x] Wire into PO's `respond_in_session()` — new entry path: the backend can direct PO into persona interview mode
- [x] Backend trigger: `session_type = "persona_interview"` routing in `run_po_intake`; `handle_persona_interview` creates persona session after requirements complete
- [x] Update `record_project_note` handler to serialize `SpecNoteContent::Persona(..)` into the `note` column when content_type is `"persona"`
- [x] Test with mock LLM: PO produces valid `PersonaNote` JSON for a sample conversation

#### Success criteria

- PO asks all 5 persona dimensions (id, name, description, goals, pain_points) per persona
- Each persona produces a `PersonaNote` stored as a structured project note
- `PersonaNote` deserializes cleanly from the `note` column
- PO gracefully handles "I don't know" answers — marks field as `Pending` / records as incomplete

#### Not in scope

- Permission interview (Phase 4c+)
- Linking personas to roles (Phase 4c+)
- Re-interviewing to fill gaps (Phase 4f clarification loop)

---

### Phase 4d: PM Rework — Agent-Facing Artifact Creation

PM's `finalize_session` currently creates user-facing epics + generic tasks. Shift PM to produce **agent-facing structured tasks** whose content is machine-readable. PM becomes a translator: PO's structured notes → formatted agent tasks that small-model code generators (Praxis Writer, DB Engineer, etc.) can consume.

#### Key Changes

| Before | After |
|---|---|
| PM creates epics for user visibility | Epics stay for user visibility, but task descriptions are structured JSON for agents |
| Task description is prose ("Create the users table...") | Task description is structured spec data (`{ "personas": [...], "permissions": [...] }`) + human-readable summary |
| `assigned_to_agent` maps to human-facing role | `assigned_to_agent` maps to agent MODE (`praxis_engineer::praxis_auth`, `db_engineer::diesel_schema`, etc.) |
| PM runs once, finalizes | PM may create multiple task batches — one per agent mode — each with its own structured payload |

#### Structured Task Content

PM tasks for `praxis_engineer` carry a `task_spec` field (JSON in `task.description`):

```rust
/// PM's structured output for Praxis Writer consumption.
struct PraxisWriterTaskSpec {
    /// The mode the Praxis Writer should use.
    mode: String,  // e.g. "praxis_auth"
    /// Persona data extracted from PO's structured project notes.
    personas: Vec<PersonaNote>,
    /// Permission data (future: from PO Permission Interview).
    permissions: Vec<PermissionNote>,
    /// PM's instructions to the Praxis Writer (natural language).
    instructions: String,
    /// Reference: which project notes informed this task.
    source_note_ids: Vec<i64>,
}
```

**Why PM, not PO directly?** PO's job is requirements discovery. PM's job is structuring those requirements for consumption by specialist agents. PM reads structured PO notes, resolves ambiguities where possible, and produces a clean task spec. If PM can't resolve an ambiguity, it flags it as `Unresolved` — which feeds the clarification loop.

#### Tasks

- [ ] Define `PraxisWriterTaskSpec` in `agents/src/storage/spec_schemas.rs`
- [ ] Update PM's `po_handoff` mode prompt to:
  - Read all structured project notes (personas, permissions)
  - Group related personas/permissions
  - Produce a `PraxisWriterTaskSpec` per mode needed
  - Use `finalize_session` with tasks whose descriptions contain serialized `PraxisWriterTaskSpec`
- [ ] Add `assigned_to_agent: "praxis_engineer"` to the agent registry
- [ ] Update `finalize_session` backend handler to parse task descriptions — if they contain a `PraxisWriterTaskSpec`, route to the appropriate agent mode dispatch
- [ ] Backend: when PM finalizes with a praxis_engineer task, auto-dispatch the Praxis Writer (RustEngineer in praxis_auth mode)

#### Success criteria

- PM reads PO's structured PersonaNotes and produces a valid `PraxisWriterTaskSpec`
- `PraxisWriterTaskSpec` deserializes from task description
- Backend dispatches praxis_engineer task to RustEngineer correctly

---

### Phase 4e: Praxis Writer — RustEngineer Mode

A new RustEngineer mode `praxis_auth` that consumes `PraxisWriterTaskSpec` data and generates `backend/src/praxis/specs/{personas,permissions,roles}.rs` in the managed project. Follows the existing RustEngineer pattern: build prompt → single-shot LLM call → deterministic post-processing → write to disk via `schema_codegen`.

#### Module Structure

```
agents/src/rust_engineer/modes/
├── mod.rs                 ← add pub mod praxis_auth;
├── praxis_auth.rs         ← NEW: prompt builder + post-processing
```

#### Input

- `PersonaNote` data (from PM's task spec) — one or more personas
- `PermissionNote` data (future) — one or more permissions
- Project path — where to write generated files (already on `RustEngineerAgent.project_path`)

#### Prompt Composition

The system prompt must include:

1. **Praxis type reference** — extracted from `nocodo_praxis` (Phase 4a): all types the LLM may use, with field names, types, and doc comments
2. **Strict output contract** — the LLM must output ONLY valid Rust code, one `static` per artifact, no `fn`, no `mod`, no imports (we prepend those deterministically)
3. **Concrete examples** — the todo_app `src/lib.rs` persona/permission/role definitions as exemplars
4. **Rules**:
   - Every `PersonaId`/`PermissionId`/`RoleId` must be a `const` with explicit type annotation
   - Every provenance must use `Provenance::Conversation { id, excerpt }` or `Provenance::Inferred { reason, from }`
   - Inferred nodes must be tagged as such — never claim a Conversation source for LLM-invented material
   - If data is missing (e.g., no goals specified for a persona), emit `Unresolved::Pending` — do NOT invent
5. **The persona data** — the actual `PersonaNote` values to encode

#### Output Files

Generated into `backend/src/praxis/specs/`:

```
backend/src/praxis/specs/
├── mod.rs           ← generated: re-exports all submodules
├── personas.rs      ← PersonaId consts + UserPersona statics
├── permissions.rs   ← (future) PermissionId consts + Permission statics
└── roles.rs         ← (future) RoleId consts + Role statics
```

Example generated `personas.rs`:
```rust
use nocodo_praxis::auth::{PersonaId, UserPersona};
use nocodo_praxis::provenance::Provenance;

const CONV_1: Provenance = Provenance::Conversation {
    id: "session-42",
    excerpt: "We need an admin who can manage the team and assign tasks.",
};

pub const PERSONA_ADMIN: PersonaId = PersonaId("admin");

pub static ADMIN_PERSONA: UserPersona = UserPersona {
    id: PERSONA_ADMIN,
    name: "Admin",
    description: "Elevated user who manages team and task assignment",
    goals: &["Manage team membership", "Create and assign tasks to anyone"],
    pain_points: &["Manual task tracking", "No visibility into team progress"],
    provenance: &[CONV_1],
};
```

#### Post-Processing (Deterministic)

1. `extract_code()` — strip `<think>` blocks, unwrap code fences
2. Parse output to confirm each expected `const`/`static` is present
3. Prepend deterministic imports (from `nocodo_praxis::{auth, provenance}`)
4. Write to project via new `code_writer::write_praxis_spec(project_path, module_name, code)` — atomic write, module registration

#### Code Writer Extensions

Add praxis-aware write helpers to `schema-codegen/src/lib.rs` (or `agents/src/code_writer.rs`):

- `write_praxis_spec(project_root, module_name, code)` — writes `backend/src/praxis/specs/{module_name}.rs` atomically
- `register_praxis_module(project_root, module_name)` — adds `pub mod {module_name};` to `backend/src/praxis/specs/mod.rs`
- Create `backend/src/praxis/specs/mod.rs` if it doesn't exist; create `backend/src/praxis/mod.rs` with `pub mod specs;` if needed

#### Tasks

- [ ] Create `agents/src/rust_engineer/modes/praxis_auth.rs` with `build_system_prompt()`, `build_user_prompt(personas)`, `extract_and_validate(raw_response)`
- [ ] Wire praxis type reference (Phase 4a) into the system prompt
- [ ] Add `praxis_auth` entry to `RustEngineerAgent` in `agent.rs` — method `run_praxis_auth(personas: Vec<PersonaNote>) -> Result<PraxisAuthOutput>`
- [ ] Add `PraxisAuthOutput { system_prompt, prompt, raw_response, files_written: Vec<String> }`
- [ ] Add praxis write helpers to `schema-codegen/src/lib.rs`: `write_praxis_spec`, `register_praxis_module`
- [ ] Wrap in `agents/src/code_writer.rs`: `write_praxis_spec(project_path, module_name, code)`
- [ ] Add backend handler: `POST /api/rust-engineer/praxis-auth` — accepts persona data, returns generated code for admin UI preview + writes to project
- [ ] Add admin UI controls for praxis_auth mode on `RustEngineerPage` (mode selector entry + persona input + preview panels)

#### Success criteria

- Praxis Writer generates valid Rust that compiles against `nocodo_praxis`
- Generated `personas.rs` is structurally identical to the todo_app spec's persona definitions
- `PersonaNote` → generated `UserPersona` conversion is correct (all fields mapped, provenance attached)
- Missing data produces `Unresolved::Pending`, not hallucinated values

#### Not in scope

- Permission and role generation (add modes `praxis_auth_permissions`, `praxis_auth_roles` after persona mode succeeds)
- State machine generation (Phase 4h)
- Entity generation (Phase 4i)

---

### Phase 4f: Clarification Loop — Unresolved → PO → Re-generate

The central value proposition. When Praxis Writer encounters missing data and emits `Unresolved::Pending`, those gaps must flow back to PO for re-interviewing, then back to Praxis Writer for re-generation.

#### Loop Mechanics

```
Praxis Writer generates personas.rs
    │
    ▼
Parse output → find Unresolved::Pending nodes
    │
    ├── None → spec complete, task done
    │
    └── Some(gaps) → for each gap:
            │
            ▼
        PM (or PO) receives structured question:
        "The spec is missing: {gap.reason}. Can you answer this?"
            │
            ▼
        PO persona interview mode re-activated with context of the gap
            │
            ▼
        User answers → new PersonaNote (or updated note) stored
            │
            ▼
        Praxis Writer re-runs with updated data
            │
            ▼
        (loop until zero Unresolved::Pending)
```

#### Unresolved Detection

After Praxis Writer generates code, a **post-generation scan** parses the output for `Unresolved::Pending` constructors:

```rust
fn find_pending_gaps(generated_code: &str) -> Vec<SpecGap> {
    // Parse generated Rust source for Unresolved::Pending { reason: "...", ... }
    // Return each gap with the reason string and the artifact it belongs to
}
```

This is **not** tree-sitter (that comes later for full validation). It's simple string/regex scanning on the generated output — sufficient for detecting `Unresolved::Pending { reason: "..." }` patterns.

#### Gap → PO Question Mapping

Each `SpecGap` becomes a structured question for PO:

| Gap reason | PO question | PO mode |
|---|---|---|
| "Maximum title length not specified" | "How long should task titles be? Pick a character limit." | Requirements gathering |
| "No goals specified for persona 'admin'" | "What does Admin need to accomplish? Select all that apply." | Persona interview |
| "Permission scope not defined for 'view_all_tasks'" | "Should 'view_all_tasks' apply to all authenticated users or a specific role?" | Permission interview (future) |

#### Tasks

- [ ] Create `find_pending_gaps(code: &str) -> Vec<SpecGap>` in `agents/src/praxis_doc.rs` or new `agents/src/spec_gap.rs`
- [ ] After Praxis Writer completes, scan output for gaps
- [ ] If gaps found: PM creates a follow-up PO session with structured questions for each gap
- [ ] Backend delivers gap questions to PO (re-activates persona interview or requirements gathering with context)
- [ ] User answers → structured notes updated → Praxis Writer re-runs
- [ ] Termination condition: zero `Unresolved::Pending` in generated output, OR user declines to answer (marked as acknowledged gap)
- [ ] Track loop iterations — if gaps don't shrink after 3 iterations, surface to admin

#### Success criteria

- A missing persona goal triggers a structured question back to the user
- Answering the question re-generates the spec without the gap
- Gaps that user declines to answer are marked (not silently dropped)

---

### Phase 4g: PO Permission Interview Mode (Future)

After personas are solid, PO interviews the user about what each persona should be able to DO. Produces `PermissionNote` structured notes. Same pattern as persona interview, new mode file.

### Phase 4h: Praxis Writer — State Machine Mode (Future)

Consumes structured entity + state data (from PO/PM) and generates `states.rs` with `StateId` consts, `State` statics, and `Transition` definitions.

### Phase 4i: Full Validation Agent (Future)

Replace basic `find_pending_gaps` with tree-sitter queries (RUNTIME.md §5.1), `rustc` compilation check (§5.2), and generated behavioral tests (§5.3).

---

## Dependency Order

```
4a.0 (doc comments in nocodo_praxis)    4b (spec schemas)
     │                                       │
4a.1 (doc extraction tool)                   │
     │                                       │
     ├──────────────────────────────────────┤
     │                                       │
4c (PO persona interview)                    │
     │                                       │
4d (PM rework)                               │
     │                                       │
4e (Praxis Writer mode) ←──── needs 4a.1 + 4b + 4d
     │
4f (clarification loop)
     │
4g+ (permission interview, state machine writer, ...)
```

4a.0 is a hard prerequisite for everything downstream — the LLM has nothing useful in its prompt otherwise. 4a.1 and 4b can run in parallel. 4c (PO persona interview) can start once 4b (PersonaNote schema) is stable. 4d (PM rework) depends on 4c producing structured notes. 4e (Praxis Writer) needs 4a.1 (type reference for prompt), 4b (PersonaNote deserialization), and 4d (PM publishes PraxisWriterTaskSpec). 4f needs 4e complete to have generated output to scan for gaps.

---

## Principles

1. **Grow from evidence** — every new type in `nocodo_praxis` must be motivated by a real spec that needed it (§7 of RUNTIME.md)
2. **Spec is runtime** — no separate IR, no sync problem
3. **Incompleteness is first-class** — `Unresolved` blocks codegen, not approximates around it
4. **Inference is visible** — `Provenance::Inferred` marks every LLM assumption
