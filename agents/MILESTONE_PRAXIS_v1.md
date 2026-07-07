# Milestone: Praxis v1 — Code as the Only Artifact

> **Epic**: Replace the PO → PM → Epics/Tasks → Specialists pipeline with a direct
> PO ↔ RustEngineer loop. Code is the only artifact. Provenance links back to chat.
> Static checking drives the loop to completion.
>
> **Context**: nocodo is an LLM-native coding agent. The user is not expected to be
> a PO, PM, or engineer. The system asks good questions and generates code.
> Epics, Tasks, and PM were experiments from a traditional software mental model —
> they add UX surface area with no verifiable value for this product.
>
> **Implementation strategy**: Prove the loop end-to-end with one type (UserPersona)
> before building all serializers. The implementation order is loop-first, not
> component-first. See §10.

---

## 1. Thesis

### Motivation

nocodo exists to turn a non-technical user's idea into running code. The user
describes what they want — in plain language, without structure — and the system
generates it.

Traditional software teams use POs, PMs, epics, tasks, and sprint planning to
bridge the gap between "what the user wants" and "what the engineer builds." These
roles exist because humans need them: a PM translates vague requirements into
actionable work items, an engineer reads those work items and writes code.

LLMs don't need that translation. An LLM can go from "admins assign tasks to
team members" directly to a generated `assign_task` function — if it has the
right vocabulary (`nocodo_praxis` types) and a deterministic way to detect what's
missing (the static checker). The intermediate artifacts — epics, tasks, project
notes, PM role — are overhead that slows the loop and leaks information.

**The goal is the shortest possible path from user intent to verifiable code.**

Every hop we remove is a place where:
- Information drifts (PM interprets PO's notes differently than the user intended)
- The user loses context (what does "task #47" mean vs seeing the actual code)
- Verification is deferred (tasks say "done" but code may not compile)

Code is verifiable. A task status is not. The static checker can prove a spec is
complete; a PM can only guess.

### Pipeline comparison

nocodo's user describes what they want to build. The system generates code. The
current pipeline intermediates that with traditional software artifacts:

```
User ↔ PO → Project Notes → PM → Epics/Tasks → Specialist agents → code
```

Each hop is a serialization point where information is lost. PM, Epics, Tasks, and
Project Notes are proxies — they exist because traditional software teams need them.
An LLM-native product does not. The user doesn't know or care what an "epic" is.

**v1 replaces all of that with a single loop:**

```
User ↔ PO → RustEngineer → generated code → static checker → gaps → PO → User
                                                        │
                                              (no gaps? loop terminates)
```

Code is the only persistent artifact. Chat sessions carry provenance. The static
checker is the engine that drives the loop to exhaustion.

### What code replaces

| Old artifact          | Code equivalent                                      |
|-----------------------|------------------------------------------------------|
| Project Note (persona)| `static PERSONA_ADMIN: UserPersona = { ... }`        |
| Project Note (perm)   | `static PERM_ASSIGN_TASK: Permission = { ... }`      |
| Task "create X"       | the generated `fn assign_task(...)` itself           |
| Epic grouping         | chat session + `Provenance::Conversation { id }`     |
| PM structuring        | static checker detecting gaps, loop asking PO        |

---

## 2. The Loop in Detail

### 2.1 Core loop

The loop runs one iteration per discovery. It does not batch — one function, one
gap, one question at a time.

```
1. PO asks user:　"What actions do people take in this system?"
2. User:　"Admins assign tasks to team members."
3. RustEngineer generates:

   fn assign_task(user: &User, task: Task, assignee: &User) -> Result<Task, TransitionError> {
       if !user.has_role(RoleId("admin")) {
           return Err(TransitionError::Forbidden {
               required: RoleId("admin"),
               found: user.role.clone(),
           });
       }
       if !task.is_in_state(StateId("todo")) {
           return Err(TransitionError::InvalidState {
               entity: EntityId("task"),
               expected: StateId("todo"),
               found: task.state,
           });
       }
       Ok(Task { assignee: Some(assignee.id), ..task })
   }

4. Static checker scans the generated function:
   - RoleId("admin")      → is ROLE_ADMIN declared?  → NO  → gap
   - StateId("todo")      → is STATE_TODO declared?   → NO  → gap
   - EntityId("task")     → is ENTITY_TASK declared?  → NO  → gap
   - User, Task types     → declared in the project?  → NO  → gap

5. PO asks user:　"A few things I need to clarify. Let's start: what is a 'task'
   in your system? What information does it carry?"

6. User:　"It has a title, a due date, who it's assigned to, and a status."

7. RustEngineer generates (or PO triggers diesel_model to generate) a Task struct.
   Static checker now sees ENTITY_TASK is defined but STATE_TODO still missing.

8. PO asks:　"What states can a task be in?"

9. User:　"Todo, In Progress, Done, Cancelled."

10. RustEngineer generates STATE_TODO, STATE_IN_PROGRESS, etc.
    Static checker: StateId("todo") now resolves. STATE_TODO is declared.
    But `fn assign_task` only handles "todo" → what about other transitions?

11. PO asks:　"What can happen from the 'In Progress' state?"

12. Loop continues until zero Unresolved gaps in generated code.
```

### 2.2 Termination guarantee

The loop terminates when the static checker finds **zero** of:

1. `Unresolved::Pending` nodes in any codegen artifact
2. Referential gaps — a `RoleId("x")`, `StateId("y")`, or `EntityId("z")` referenced
   in a generated function but not declared as a static in the spec
3. Missing guard functions — a `State` declares a `Transition` but no generated
   function implements that transition's guard logic
4. Missing transition coverage — a guard function handles only a subset of
   declared transitions for a state

The checker is exhaustive by design: every category above is checked on every
iteration. The loop cannot silently skip a gap.

### 2.3 Provenance

Every generated function and static carries provenance:

```rust
const CHAT_SESSION_42: Provenance = Provenance::Conversation {
    id: "session-42",
    excerpt: "Admins assign tasks to team members",
};

fn assign_task(...) -> Result<Task, TransitionError> {
    // provenance: CHAT_SESSION_42
    ...
}
```

The running system can answer "why does this rule exist?" — click through to the
chat message that produced it.

---

## 3. What Gets Commented Out

All intermediate artifact infrastructure is disabled. We comment out imports and
call sites rather than deleting code — this is an experiment and the old code
remains available for reference. Once v1 loop is proven, we delete for real.

### 3.1 Agent imports (comment out in mod.rs / agent registry)

| Agent                  | Reason                                            |
|------------------------|---------------------------------------------------|
| `project_manager` (PM) | No epics/tasks to create. Loop replaces PM        |
| `engineering_manager`  | No tasks to shape. Loop absorbs gating            |
| `db_engineer`          | Task execution agent — no tasks to execute        |
| `backend_engineer`     | Task execution agent — no tasks to execute        |
| `frontend_engineer`    | Task execution agent — no tasks to execute        |
| `ui_designer`          | Task execution agent — no tasks to execute        |

**Note on specialist agents:** The diesel_model, diesel_schema, and diesel_model_struct
RustEngineer modes are **not** these agents. They are prompt modes within RustEngineer,
triggered by the user/admin, not dispatched by a task state machine. They stay.

### 3.2 API handlers and routes (comment out)

- All CRUD endpoints for `epics`, `tasks`, `project_notes`
- All comment endpoints (`/api/epics/{id}/comments`, `/api/tasks/{id}/comments`)
- Agent chat endpoints (`/api/agents/{agent}/chat`)
- Dispatcher (task dispatch, status transitions)
- Task validation handlers (PO `validate_tasks`, EM `mark_task_ready`)

### 3.3 admin-gui components (comment out imports)

- Epic list/detail views
- Task list/detail views (including status, assignment)
- Task comment panes
- Agent chat drawer (already removed per Phase 6 of previous milestone)
- Dispatcher-related UI

### 3.4 Database tables

Tables stay for now (no drop migration). They are unused once the imports above
are commented out. Drop migration comes later when v1 is proven.

### 3.5 Configuration and registry

- Comment out agent registry entries for disabled agents
- Comment out `initial_state_for` policy function
- Comment out task state machine transitions from backend

---

## 4. Prepare `nocodo_praxis`

Praxis needs types to support the behavioral layer — not just declarative structures
but the error types and helper traits that generated transition functions compose with.

### 4.1 Add `TransitionError`

A first-class error type for transition guard functions. This type becomes the
vocabulary the RustEngineer uses when emitting error returns.

```rust
// nocodo_praxis::statemachine

pub enum TransitionError {
    /// The acting user lacks the required role.
    Forbidden {
        required: RoleId,
        found: Option<RoleId>,
    },
    /// The acting user lacks the required permission.
    ForbiddenPermission {
        required: PermissionId,
    },
    /// The entity is not in the expected state for this transition.
    InvalidState {
        entity: EntityId,
        expected: StateId,
        found: Option<StateId>,
    },
    /// A business invariant was violated.
    InvariantViolation {
        invariant: &'static str,
    },
    /// The transition is not defined because the PRD was silent.
    Unresolved {
        reason: &'static str,
    },
}
```

### 4.2 Add `trait TransitionGuard`

A trait that every generated guard function implements. This gives the static
checker a hook to find all guard functions via tree-sitter or Rust analysis.

```rust
// nocodo_praxis::statemachine

pub trait TransitionGuard<Input, Output> {
    /// The entity this guard operates on.
    const ENTITY: EntityId;
    /// The source state this guard transitions FROM.
    const FROM: StateId;
    /// The target state this guard transitions TO.
    const TO: StateId;
    /// Execute the transition. Returns the mutated entity or an error.
    fn guard(input: Input) -> Result<Output, TransitionError>;
}
```

Implementation example (generated):

```rust
struct AssignTask;

impl TransitionGuard<(User, Task, User), Task> for AssignTask {
    const ENTITY: EntityId = EntityId("task");
    const FROM: StateId = StateId("todo");
    const TO: StateId = StateId("in_progress");

    fn guard((user, task, assignee): (User, Task, User)) -> Result<Task, TransitionError> {
        if !user.has_role(RoleId("admin")) {
            return Err(TransitionError::Forbidden {
                required: RoleId("admin"),
                found: Some(user.role),
            });
        }
        if !task.is_in_state(StateId("todo")) {
            return Err(TransitionError::InvalidState {
                entity: EntityId("task"),
                expected: StateId("todo"),
                found: Some(task.state),
            });
        }
        Ok(Task { assignee: Some(assignee.id), state: StateId("in_progress"), ..task })
    }
}
```

### 4.3 Add `trait HasRole` and `trait HasState`

Traits that the project's model types implement, bridging the gap between the
Diesel model structs and the praxis vocabulary.

```rust
// nocodo_praxis::auth

pub trait HasRole {
    fn role(&self) -> RoleId;
    fn has_role(&self, role: RoleId) -> bool {
        self.role() == role
    }
}

// nocodo_praxis::statemachine

pub trait HasState {
    fn state(&self) -> StateId;
    fn is_in_state(&self, state: StateId) -> bool {
        self.state() == state
    }
}
```

### 4.4 Expand documentation

Every new type and trait needs doc comments and construction examples (doctests),
following the pattern from MILESTONE_PRAXIS Phase 4a.0. This feeds the RustEngineer
prompt via `praxis_doc.rs`.

### Tasks (Phase: Prepare Praxis)

- [ ] Add `TransitionError` enum to `nocodo_praxis::statemachine`
- [ ] Add `TransitionGuard` trait to `nocodo_praxis::statemachine`
- [ ] Add `HasRole` trait to `nocodo_praxis::auth`
- [ ] Add `HasState` trait to `nocodo_praxis::statemachine`
- [ ] Add `EntityId`, `StateId`, `RoleId`, `PermissionId` to the prelude for easy import
- [ ] Doc comments + doctests for all new items
- [ ] `cargo test -p nocodo_praxis` — all doctests pass
- [ ] `cargo doc -p nocodo_praxis --no-deps` — docs complete
- [ ] Update `agents/src/praxis_doc.rs` to extract new types for RustEngineer prompts

---

## 5. `record_spec_fact` — Type-by-Type Serialization

`record_spec_fact` is the bridge between "user said something in chat" and
"valid Rust code exists in the project." PO emits structured JSON; the backend
serializes it to valid `nocodo_praxis` Rust code and writes it to the project.

Each praxis type gets its own serialization path. We build and test them one at
a time, starting with the type that already works (PersonaNote → UserPersona).

**Phased approach:** Type 1 (UserPersona) ships in Phase 3 and is immediately
wired into the minimal loop (Phase 3.5) to prove the end-to-end flow. Types 2–5
ship in Phase 4 after the loop is proven. This avoids building five serializers
before discovering the loop doesn't work.

### 5.1 Type order

The order follows the dependency graph — each type references the ones before it:

| # | Type | PO JSON input | Backend Rust output | Depends on |
|---|------|---------------|---------------------|------------|
| 1 | `UserPersona` | `PersonaNote` (exists) | `static PERSONA_X: UserPersona = ...` | nothing |
| 2 | `Permission` | `{ id, description }` | `static PERM_X: Permission = ...` | nothing |
| 3 | `Role` | `{ id, description, permission_ids, persona_ids }` | `static ROLE_X: Role = ...` | Permission, Persona |
| 4 | `State` + `Transition` | `{ id, description, transitions: [{ to, role_ids, condition }] }` | `static STATE_X: State = ...` | Role |
| 5 | `Entity` | `{ id, description, fields, state_ids }` | `static ENTITY_X: Entity = ...` | State |
| 6 | Transition guard | natural language action description | `struct + impl TransitionGuard` | Entity, State, model types |

### 5.2 Serialization pattern

Each type follows the same pattern:

```
1. PO calls record_spec_fact(fact_type: "persona", data: { ... })
2. Backend deserializes data into the Rust struct (e.g. PersonaNote)
3. Backend calls the serializer: persona_to_rust(note, session_id) -> String
4. Serializer produces valid Rust source with Provenance::Conversation
5. Backend writes to backend/src/praxis/specs/{module}.rs via schema_codegen
6. Backend registers the module in mod.rs if new
7. Static checker runs on the updated codebase
```

The serializer is deterministic — no LLM call. It's a Rust function that
formats structured data into `nocodo_praxis` constructor syntax.

### 5.3 Type 1: UserPersona (already works)

`PersonaNote` → `UserPersona` conversion exists from MILESTONE_PRAXIS Phase 4b.
We wrap it in the `record_spec_fact` flow and write to `specs/personas.rs`.

```rust
// Input: PersonaNote JSON
{
    "id": "admin",
    "name": "Team Admin",
    "description": "Manages team and task assignment",
    "goals": ["Manage team membership", "Create tasks"],
    "pain_points": ["Manual tracking", "No visibility"],
    "provenance": "session-42"
}

// Output: Rust source written to backend/src/praxis/specs/personas.rs
use nocodo_praxis::auth::{PersonaId, UserPersona};
use nocodo_praxis::provenance::Provenance;

const CONV_SESSION_42: Provenance = Provenance::Conversation {
    id: "session-42",
    excerpt: "Manages team and task assignment",
};

pub const PERSONA_ADMIN: PersonaId = PersonaId("admin");

pub static ADMIN_persona: UserPersona = UserPersona {
    id: PERSONA_ADMIN,
    name: "Team Admin",
    description: "Manages team and task assignment",
    goals: &["Manage team membership", "Create tasks"],
    pain_points: &["Manual tracking", "No visibility"],
    provenance: &[CONV_SESSION_42],
};
```

**Tasks:**
- [ ] Wrap existing `PersonaNote → UserPersona` conversion in `record_spec_fact` handler
- [ ] Add `persona_to_rust(note: &PersonaNote, session_id: &str) -> String` serializer
- [ ] Write to `backend/src/praxis/specs/personas.rs` via schema_codegen
- [ ] Unit test: round-trip JSON → serialize → compile against `nocodo_praxis`
- [ ] Test: two personas in the same file, different provenance IDs

### 5.4 Type 2: Permission

```rust
// Input
{ "id": "create_task", "description": "Create a new task" }

// Output: written to backend/src/praxis/specs/permissions.rs
use nocodo_praxis::auth::{PermissionId, Permission};
use nocodo_praxis::provenance::Provenance;

const CONV_SESSION_42: Provenance = Provenance::Conversation {
    id: "session-42",
    excerpt: "Create a new task",
};

pub const PERM_CREATE_TASK: PermissionId = PermissionId("create_task");

pub static PERM_CREATE_TASK_DEF: Permission = Permission {
    id: PERM_CREATE_TASK,
    description: "Create a new task",
    provenance: &[CONV_SESSION_42],
};
```

**Tasks:**
- [ ] Define permission JSON schema (simple: `id`, `description`, optional `inferred: bool`)
- [ ] Add `permission_to_rust()` serializer
- [ ] Add to `record_spec_fact` handler with `fact_type: "permission"`
- [ ] Unit test: round-trip

### 5.5 Type 3: Role

Roles reference existing permissions and personas by ID. The serializer looks
up the `const` names from already-written spec files.

```rust
// Input
{
    "id": "admin",
    "description": "Team admin",
    "permission_ids": ["create_task", "assign_task"],
    "persona_ids": ["admin"],
    "semantics": "inherits",
    "parent_role_id": "member"
}

// Output: written to backend/src/praxis/specs/roles.rs
use nocodo_praxis::auth::{RoleId, Role, RoleSemantics, PermissionId, PersonaId};
use nocodo_praxis::provenance::Provenance;

const CONV_SESSION_42: Provenance = Provenance::Conversation {
    id: "session-42",
    excerpt: "Team admin",
};

pub const ROLE_ADMIN: RoleId = RoleId("admin");

pub static ROLE_ADMIN_DEF: Role = Role {
    id: ROLE_ADMIN,
    description: "Team admin",
    semantics: RoleSemantics::Inherits { parent: RoleId("member") },
    permissions: &[PermissionId("create_task"), PermissionId("assign_task")],
    personas: &[PersonaId("admin")],
    provenance: &[CONV_SESSION_42],
};
```

**Note:** The serializer does NOT validate that referenced `PermissionId` /
`PersonaId` values exist in the spec. The static checker (C2: referential
integrity) catches missing references after the code is written. This keeps
the serializer simple and the checker load-bearing.

**Tasks:**
- [ ] Define role JSON schema (`id`, `description`, `permission_ids`, `persona_ids`, `semantics`, `parent_role_id`)
- [ ] Add `role_to_rust()` serializer — handles Flat, Inherits, Union semantics
- [ ] Add to `record_spec_fact` handler with `fact_type: "role"`
- [ ] Unit test: round-trip for each semantics variant

### 5.6 Type 4: State + Transition

States reference roles in their `permitted_roles`. The `TransitionCondition`
may be `Unresolved` if the user hasn't specified the condition.

```rust
// Input
{
    "id": "todo",
    "description": "Task created, not yet started",
    "transitions": [
        {
            "to": "in_progress",
            "role_ids": ["member", "admin"],
            "condition": "only_if_assigned_to_self"
        },
        {
            "to": "cancelled",
            "role_ids": ["admin"],
            "condition": "always"
        }
    ]
}

// Output: written to backend/src/praxis/specs/states.rs
use nocodo_praxis::auth::RoleId;
use nocodo_praxis::primitives::AtLeastOne;
use nocodo_praxis::provenance::Provenance;
use nocodo_praxis::statemachine::{State, StateId, Transition, Transitions, TransitionCondition};

const CONV_SESSION_42: Provenance = Provenance::Conversation {
    id: "session-42",
    excerpt: "Task created, not yet started",
};

pub const STATE_TODO: StateId = StateId("todo");

pub static STATE_TODO_DEF: State = State {
    id: STATE_TODO,
    description: "Task created, not yet started",
    transitions: Transitions::To(AtLeastOne {
        head: Transition {
            to: StateId("in_progress"),
            permitted_roles: AtLeastOne {
                head: RoleId("member"),
                tail: &[RoleId("admin")],
            },
            condition: TransitionCondition::OnlyIfAssignedToSelf,
            provenance: &[CONV_SESSION_42],
        },
        tail: &[Transition {
            to: StateId("cancelled"),
            permitted_roles: AtLeastOne {
                head: RoleId("admin"),
                tail: &[],
            },
            condition: TransitionCondition::Always,
            provenance: &[CONV_SESSION_42],
        }],
    }),
    provenance: &[CONV_SESSION_42],
};
```

**Condition mapping:** The serializer maps string condition names to
`TransitionCondition` variants. Unknown conditions produce
`TransitionCondition::Unresolved("condition not specified")`.

**Tasks:**
- [ ] Define state JSON schema (`id`, `description`, `transitions[]` with `to`, `role_ids`, `condition`)
- [ ] Add `state_to_rust()` serializer — builds `AtLeastOne<Transition>`, maps conditions
- [ ] Add to `record_spec_fact` handler with `fact_type: "state"`
- [ ] Unit test: state with multiple transitions, state with Unresolved condition

### 5.7 Type 5: Entity

```rust
// Input
{
    "id": "task",
    "description": "A unit of work that can be assigned and tracked",
    "fields": [
        { "name": "title", "description": "Short name" },
        { "name": "status", "description": "Current lifecycle state" },
        { "name": "assignee", "description": "Who is responsible" }
    ],
    "state_ids": ["todo", "in_progress", "done", "cancelled"]
}

// Output: written to backend/src/praxis/specs/entities.rs
use nocodo_praxis::entity::{Entity, EntityId, Field};
use nocodo_praxis::primitives::Unresolved;
use nocodo_praxis::provenance::Provenance;
use nocodo_praxis::statemachine::StateId;

const CONV_SESSION_42: Provenance = Provenance::Conversation {
    id: "session-42",
    excerpt: "A unit of work that can be assigned and tracked",
};

pub const ENTITY_TASK: EntityId = EntityId("task");

pub static ENTITY_TASK_DEF: Entity = Entity {
    id: ENTITY_TASK,
    description: "A unit of work that can be assigned and tracked",
    fields: &[
        Field {
            name: "title",
            description: "Short name",
            invariants: &[Unresolved::Pending {
                reason: "Maximum title length not specified",
                provenance: &[Provenance::Inferred {
                    reason: "All text fields require a length bound",
                    from: &["session-42"],
                }],
            }],
            provenance: &[CONV_SESSION_42],
        },
        Field {
            name: "status",
            description: "Current lifecycle state",
            invariants: &[],
            provenance: &[CONV_SESSION_42],
        },
        Field {
            name: "assignee",
            description: "Who is responsible",
            invariants: &[],
            provenance: &[CONV_SESSION_42],
        },
    ],
    states: &[
        StateId("todo"),
        StateId("in_progress"),
        StateId("done"),
        StateId("cancelled"),
    ],
    invariants: &[],
    provenance: &[CONV_SESSION_42],
};
```

**Field invariants:** Fields without explicit constraints get
`Unresolved::Pending` — the static checker flags these as gaps and PO asks
the user for constraints.

**Tasks:**
- [ ] Define entity JSON schema (`id`, `description`, `fields[]`, `state_ids`)
- [ ] Add `entity_to_rust()` serializer — produces Field with Unresolved invariants by default
- [ ] Add to `record_spec_fact` handler with `fact_type: "entity"`
- [ ] Unit test: entity with fields, Unresolved invariants, state references

### 5.8 Type 6: Transition Guard (via RustEngineer)

Transition guards are NOT serialized by `record_spec_fact`. They are generated
by RustEngineer's `state_transition` mode (Section 6) because they contain
application logic that requires an LLM. PO triggers codegen via
`trigger_codegen(action_description)`.

**Prerequisite:** Model types (User, Task, etc.) must exist before transition
guards can reference them. See Section 6.4.

### 5.9 `record_spec_fact` tool definition

```rust
// agents/src/tools/record_spec_fact.rs

struct RecordSpecFactParams {
    /// Type of spec fact: "persona", "permission", "role", "state", "entity"
    fact_type: String,
    /// The structured data — schema depends on fact_type
    data: serde_json::Value,
}
```

PO calls this tool after gathering enough information from the user. The backend
routes to the correct serializer based on `fact_type`.

**PO does not know Rust syntax.** PO emits structured JSON with field names
and values. The serializer handles all Rust formatting, imports, const/static
declarations, and Provenance construction.

### 5.10 Summary

| Step | What happens | LLM involved? |
|------|-------------|---------------|
| User describes system | PO asks clarifying questions | Yes (PO) |
| PO calls `record_spec_fact` | PO emits structured JSON | Yes (PO) |
| Backend serializes JSON → Rust | Deterministic formatting | No |
| Backend writes to project | schema_codegen atomic write | No |
| Static checker runs | Scans for gaps | No |
| Gaps found? | PO asks follow-up questions | Yes (PO) |
| No gaps? | Loop continues or terminates | — |

---

## 6. RustEngineer — State Transition Mode

### 6.1 Mode: `state_transition`

A new RustEngineer mode that generates a single transition guard function per call.
Follows the existing single-shot pattern: build prompt → LLM call → parse →
deterministic post-processing → write to project.

**Input:**
- Natural language description of the action (from PO's conversation with user)
- Already-generated spec statics in the project (personas, roles, states, entities)
- Project model types (User, Task, etc.) — their struct definitions and trait impls

**Output:**
- One `struct` + `impl TransitionGuard` block
- Written to `backend/src/praxis/transitions/{action_name}.rs`

### 6.2 Prompt Composition

The system prompt includes:

1. **Praxis type reference** — `TransitionGuard`, `TransitionError`, `HasRole`,
   `HasState`, `EntityId`, `StateId`, `RoleId` with doc comments and usage examples
2. **Existing spec context** — already-declared `static`s (personas, roles, states)
   that the function may reference. If a needed role/state is not yet declared,
   the LLM must use it anyway — the static checker will catch the gap
3. **Model type context** — relevant project structs (User, Task, etc.) so the LLM
   knows field names and types. This may require extending diesel_model modes —
   **analysis postponed to implementation time**
4. **Strict output contract:**
   - One `struct {name}` + one `impl TransitionGuard<...> for {name}` block
   - Every referenced `RoleId("x")`, `StateId("y")`, `EntityId("z")` must use a
     `const` pattern — declare `const ROLE_X: RoleId = RoleId("x")` at the top
   - Missing information → `TransitionError::Unresolved { reason: "..." }`
   - No imports (prepended deterministically)
   - No markdown fences
5. **Concrete examples** — 2-3 complete transition guard functions from the todo_app
   reference project

### 6.3 Post-Processing

1. `extract_code()` — strip `<think>` blocks, unwrap code fences
2. Parse to confirm `impl TransitionGuard` is present
3. Extract all `RoleId("...")`, `StateId("...")`, `EntityId("...")` references
4. Prepend deterministic imports from `nocodo_praxis` and project model crate
5. Write to `backend/src/praxis/transitions/{name}.rs` via `schema_codegen`
   (new helper: `write_transition(project_root, name, code)`)
6. Register in `backend/src/praxis/transitions/mod.rs`
7. Emit a structured summary for the static checker: `{ function_name, referenced_roles, referenced_states, referenced_entities, unresolved }`

### 6.4 Model Type Dependency — Resolved

Transition guard functions reference project model structs (`User`, `Task`, etc.).
These structs must exist before transition guards can reference them. This is
not circular — it's a dependency ordering that the loop handles naturally:

**Ordering rule:** Model types (Diesel structs) must be generated before transition
functions that reference them. The existing `diesel_model_struct` and `diesel_schema`
RustEngineer modes already handle this. The PO loop triggers model generation first:

```
PO: "What information does a task carry?"
User: "Title, due date, assignee, status"
PO → trigger_codegen("generate Task struct") → diesel_model_struct
Static checker: ENTITY_TASK now has a model type ✓

PO: "Who can assign a task?"
User: "Admins, when the task is in 'todo' state"
PO → trigger_codegen("assign_task transition") → state_transition
Static checker: assign_task references Task, Task exists ✓
```

The static checker validates this: if a transition guard references `Task` and
no `Task` struct exists in the project, it's a gap (C2: referential integrity).
PO asks the user about the missing model, triggers `diesel_model_struct`, then
continues.

**No placeholder types.** The loop generates real types in the correct order.
The checker enforces that referenced types exist.

### Tasks (Phase: RustEngineer Mode)

- [ ] Create `agents/src/rust_engineer/modes/state_transition.rs`
- [ ] Implement `build_system_prompt() → String` — praxis types + existing spec + rules
- [ ] Implement `build_user_prompt(action_description, model_types, existing_spec) → String`
- [ ] Implement `extract_transition_metadata(code) → TransitionMetadata` — parse
      referenced IDs from generated code
- [ ] Add `run_state_transition(...)` method on `RustEngineerAgent`
- [ ] Add praxis write helpers to `schema-codegen/src/lib.rs`:
  - `write_transition(project_root, name, code)` — atomic write to `transitions/{name}.rs`
  - `register_transition_module(project_root, name)` — add to `mod.rs`
- [ ] Create `backend/src/praxis/transitions/mod.rs` template
- [ ] Backend handler: `POST /api/rust-engineer/state-transition`
  - Accepts: `project_id`, `action_description`, `apply: bool`
  - Returns: `{ system_prompt, prompt, raw_response, code, metadata }`
- [ ] Admin UI: mode selector entry for `state_transition` on `RustEngineerPage`
  (free-text area for action description + preview panels)
- [ ] Decide and document the model type ordering rule: before PO calls
      `trigger_codegen` for a transition guard on entity X, a model struct for X
      must exist. Enforce this either in PO's prompt ("always generate the model
      struct before the transition guard") or in the backend handler (check for
      existence of the model file and return an error if missing). Document the
      chosen approach in the `run_state_transition` handler.
- [ ] Analyze model type dependency and document the decision in code comments

---

## 7. Static Checker

The static checker is a **deterministic, non-LLM** component that operates on the
generated code tree. It uses tree-sitter (Rust grammar) to extract references and
cross-reference them against declared statics.

### 7.1 Check Categories

#### C1: Unresolved nodes

Scan all generated code for `Unresolved::Pending { reason: "..." }` or
`TransitionError::Unresolved { reason: "..." }`. Each is a named gap.

```rust
// Tree-sitter query (pseudocode)
(call_expression
  function: (scoped_identifier
    path: (identifier) @enum_type
    name: (identifier) @variant)
  (#match? @variant "Pending|Unresolved")
  arguments: (arguments
    (field_initializer
      name: (field_identifier) @field (#eq? @field "reason")
      value: (string_literal) @reason)))
```

#### C2: Referential integrity

Extract every `RoleId("...")`, `StateId("...")`, `EntityId("...")`,
`PermissionId("...")`, `PersonaId("...")` string literal from generated code.
Cross-reference against declared statics in `backend/src/praxis/specs/`.

```rust
// Extract all ID references from generated code
fn extract_id_refs(code: &str) -> IdRefs {
    // RoleId("admin") → ("role", "admin")
    // StateId("todo") → ("state", "todo")
    // EntityId("task") → ("entity", "task")
    // ...
}

// Cross-reference against declared statics
fn check_referential_integrity(refs: &IdRefs, spec: &SpecIndex) -> Vec<Gap> {
    refs.all().filter(|r| !spec.contains(r.kind, r.id)).collect()
}
```

#### C3: Guard function coverage

For every declared `State` with `Transitions::To(...)`, verify a `TransitionGuard`
impl exists with matching `FROM` and `TO` constants.

```rust
// Find all declared transitions
fn declared_transitions(spec: &SpecIndex) -> Vec<TransitionSpec> {
    // entity_id, from_state, to_state
}

// Find all generated guard functions
fn implemented_transitions(code_tree: &CodeTree) -> Vec<TransitionSpec> {
    // Extract FROM/TO from impl TransitionGuard blocks
}

// Gap: declared but not implemented
fn missing_guards() -> Vec<Gap> {
    declared.difference(implemented)
}
```

#### C4: Missing spec declarations

A generated guard function references `RoleId("admin")` — this means a
`static ROLE_ADMIN: Role` must exist. Check for missing spec artifacts
(personas for roles, permissions for roles, states for entities, etc.).

**Order of checking matters.** If a `Role` is declared but its associated
`UserPersona` is not, that's also a gap. A `Role` with permissions but no
personas is suspect. The checker walks the dependency graph.

### 7.2 Gap → PO Question Mapping

The checker produces a `Vec<Gap>`. Each gap maps to a structured question PO
can ask the user:

| Gap type | Example gap | PO question |
|----------|------------|-------------|
| Unresolved | `reason: "Maximum title length not specified"` | "How long should task titles be? Pick a character limit." |
| Ref integrity | `RoleId("admin")` not declared | "You mentioned 'admin'. Can you describe this role?" |
| Missing guard | `todo → cancelled` declared but no guard | "Who can cancel a task, and under what conditions?" |
| Missing persona | `ROLE_MEMBER` has `personas: [PersonaId("member")]` but `PERSONA_MEMBER` not declared | "Can you describe the typical team member?" |
| Missing permission | `ROLE_ADMIN` has `permissions: [PermissionId("create_task")]` but `PERM_CREATE_TASK` not declared | "What exactly does 'create a task' mean?" |

### 7.3 Implementation Strategy

Start with `cargo check` as the first checker. Run `cargo check` on the
generated spec crate, parse the compiler error output, and map errors to gaps.
This catches type mismatches, missing fields, invalid enum variants, and
unresolved imports — 80% of the checker's value for 20% of the effort.

Then add praxis-specific checks incrementally:

1. **`cargo check` wrapper** (Phase 3.5) — run `cargo check` on the generated
   spec crate and pass the raw stderr to PO for interpretation. PO translates
   compiler errors into user-facing questions. **Do not parse rustc error output
   into `Gap` structs** — rustc's error format is not stable and a regex parser
   over it is brittle. PO's language understanding is the right tool to bridge
   "cannot find value `ROLE_ADMIN` in this scope" → "you mentioned 'admin' — can
   you describe that role?". This is replaced in Phase 5 with the dedicated
   static checker, at which point PO no longer needs to interpret raw compiler
   output.
2. **Regex-based scanning** (Phase 5) — `Unresolved::Pending` patterns,
   `RoleId("...")` extraction. Covers what `cargo check` can't (semantic gaps
   like missing guard functions).
3. **Tree-sitter queries** (future) — precise source locations for errors,
   state machine soundness checks, completeness queries. Replaces regex when
   the checker stabilizes.

The checker is a Rust library — no LLM call, no network (except `cargo check`
subprocess in the early phases). It compiles the project's spec and transition
code into an in-memory index, then runs queries against that index.

### Tasks (Phase: Static Checker)

- [ ] Implement `cargo check` wrapper — run `cargo check` on spec crate, capture
      raw stderr, pass to PO as gap context (Phase 3.5 — PO interprets compiler
      errors into questions; no parsing of rustc output into Gap structs)
- [ ] Create `agents/src/static_checker/mod.rs`
- [ ] Implement `SpecIndex` — in-memory index of all declared statics
  (personas, roles, permissions, states, entities) loaded from project source
- [ ] Implement `extract_id_refs(code: &str) -> IdRefs` — extract all praxis
  ID references from generated code (regex first, tree-sitter later)
- [ ] Implement `check_unresolved(code: &str) -> Vec<Gap>` — scan for
  `Unresolved::Pending` and `TransitionError::Unresolved` patterns
- [ ] Implement `check_referential_integrity(refs, spec) -> Vec<Gap>` —
  cross-reference IDs against declared statics
- [ ] Implement `check_guard_coverage(spec, transitions) -> Vec<Gap>` —
  ensure every declared transition has a guard impl
- [ ] Implement `check_spec_completeness(spec) -> Vec<Gap>` —
  roles have associated personas and permissions
- [ ] Implement `map_gaps_to_questions(gaps) -> Vec<PoQuestion>` —
  convert each gap to a structured question PO can ask
- [ ] Unit tests: each check category with both passing and failing input
- [ ] Integration test: load todo_app spec, run all checks, expect zero gaps
- [ ] Wire checker into the loop (replaces `cargo check` parsing from Phase 3.5)

---

## 8. Wire the Loop

With all pieces built, wire them into a single session loop.

### 8.1 Session flow

A `user_chat_session` now has a single, indefinite loop. No handoff, no planning
session, no completion. The session is "open" until the static checker finds
zero gaps.

```
POST /api/user-chats                    → create session, fire PO
POST /api/user-chats/{id}/messages      → append user message, fire PO
```

PO's responses are driven by the static checker:

1. User sends message → PO responds (text or structured question).
   PO records information via `record_spec_fact` helper (writes directly to
   spec static files, not to a project_notes table).
2. When PO identifies an action to generate, it signals the backend.
3. Backend fires RustEngineer `state_transition` mode.
4. Backend runs static checker on the updated codebase.
5. If gaps found → backend fires PO with gap context → PO asks user.
6. User answers → goto step 1.
7. If no gaps → backend marks session complete.

### 8.2 PO modes

PO has a single, unified prompt. The old separate modes (`requirements_gathering`,
`project_naming`, `persona_interview`) are commented out. The loop context tells
PO what to ask about — no mode switching needed.

PO's tools:
- `record_spec_fact(fact_type, data)` — writes a spec static to the project
  (persona, role, permission, state, entity). PO doesn't need to know the
  Rust syntax — the backend serializes PO's structured output to valid Rust.
  See Section 5 for the type-by-type plan.
- `request_user_input(question, input_type, options)` — structured question
- `trigger_codegen(action_description)` — signals backend to fire RustEngineer
  (for model types via `diesel_model_struct`, or transition guards via `state_transition`)
- `finalize_session(closing_message)` — when static checker returns zero gaps

**PO ordering mode:** When the static checker reports gaps, PO inspects what's
missing and decides what to ask about next (FIFO). PO doesn't need a separate
"ordering" mode — the gap list is the ordering. The checker says "RoleId('admin')
not declared" → PO asks about admin. First gap, first question.

**Context window management:** By iteration 10-15, PO's prompt will contain the
full conversation, all generated spec files, gap descriptions, and the praxis
type reference. For a ≤20B model, this is the dominant risk. Mitigations:

- The static checker produces a **summarized gap context** (just the unresolved
  IDs and their reasons), not a full spec dump. PO only needs to know what's
  missing, not what's complete.
- The conversation history is trimmed to the last N exchanges plus the original
  user description. Earlier Q&A is summarized by PO itself into a "what we know"
  block that replaces the raw history.
- The praxis type reference is included once (system prompt), not repeated per
  iteration. Only the gap summary changes between iterations.
- If context pressure becomes unmanageable, the session can be split: one PO
  session per entity type (personas, then roles, then states). The checker
  orchestrates which session to activate next.

### 8.3 Backend orchestration

The backend manages the loop as a state machine:

```
WaitingForUser
    │
    ▼
PoResponding        ← PO generates a response
    │
    ├── text/structured → WaitingForUser
    ├── trigger_codegen → CodeGenRunning
    └── finalize        → Completed

CodeGenRunning
    │
    ▼
StaticChecking      ← checker runs (Phase 3.5: cargo check; Phase 5+: dedicated checker)
    │
    ├── gaps found → PoWithGaps → PO generates gap questions → WaitingForUser
    └── no gaps    → PoResponding (PO asks "anything else?") → repeat or finalize
```

The initial checker (Phase 3.5) is a `cargo check` wrapper — it runs the
compiler on the generated spec crate and passes raw stderr to PO, which
interprets the errors into user-facing questions (no Gap struct parsing).
This is replaced in Phase 5 with a dedicated static checker that produces
structured gaps and eliminates the need for PO to interpret compiler output.

### Tasks (Phase: Wire the Loop)

- [ ] Simplify `user_chat_session` — remove `handoff_session_id`, `session_type`
      (no intake/planning/persona_interview distinction). Single session type.
- [ ] Rewrite PO agent to single unified mode (`agents/src/product_owner/modes/loop.rs`)
- [ ] Implement `record_spec_fact` tool — PO describes a persona in structured JSON;
      backend serializes it to valid praxis Rust code and writes to project
      (persona only in Phase 3.5, expanded to all types in Phase 4)
- [ ] Implement `trigger_codegen` tool — backend runs `cargo check` on the
      generated spec crate, parses error output, maps to gaps (Phase 3.5);
      upgraded to dedicated static checker in Phase 5
- [ ] Implement `finalize_session` tool — PO signals the system is complete
- [ ] Implement backend orchestration state machine (per-session state tracking)
- [ ] Static checker runs on every codegen trigger and surfaces gaps to PO
- [ ] PO produces gap-driven questions when gaps exist
- [ ] End-to-end test: create session, describe todo app, loop to completion,
      verify generated code has zero static checker gaps

---

## 9. UI Updates

### 9.1 admin-gui

The user chat UI stays — it's the primary interface. Changes:

- **Remove:** epic list, task list, task detail, comment panes, dispatcher views
- **Keep:** user chat surface (PO conversation), RustEngineer page (expanded with
  new modes), spec viewer (read-only view of generated `praxis/specs/` and
  `praxis/transitions/`)

### 9.2 Spec viewer (new)

A read-only file tree showing generated praxis artifacts:

```
praxis/
├── specs/
│   ├── personas.rs
│   ├── permissions.rs
│   ├── roles.rs
│   ├── states.rs
│   └── entities.rs
├── transitions/
│   ├── assign_task.rs
│   ├── complete_task.rs
│   └── ...
└── mod.rs
```

Each file shows the code with syntax highlighting. Each `Provenance::Conversation`
link is clickable — it navigates to the chat message that produced it.

### Tasks (Phase: UI)

- [ ] Remove epic/task views and routes from admin-gui
- [ ] Remove comment components
- [ ] Add spec viewer to admin-gui sidebar
- [ ] Spec viewer: file tree + code viewer with clickable provenance links
- [ ] RustEngineer page: add `state_transition` to mode selector
- [ ] User chat: remove handoff navigation logic (no planning session)
- [ ] User chat: show static checker gap context when PO asks gap-driven questions
- [ ] User chat: when loop reaches zero gaps, display a "review your spec"
      prompt — NOT a "done" message. Spec viewer must be immediately reachable
      from this state (see risk §11.7: zero gaps ≠ complete system)

---

## 10. Implementation Order

Tasks are ordered so each phase produces a working increment. No phase deletes
working code — we comment out imports and disable call sites. Deletion comes
after v1 loop is proven end-to-end.

**Key restructuring principle: prove the loop end-to-end as early as possible.**
Build one type (UserPersona), wire the minimal loop, then expand. A working loop
with one type is more valuable than five serializers without a loop.

### Phase 1: Disable old infrastructure (comment out, don't delete)

Dependencies: none. Safe to start immediately.

1. Comment out agent registry entries for PM, EM, specialist agents
2. Comment out epic/task/project_notes API handler imports in backend router
3. Comment out agent chat endpoint imports
4. Comment out dispatcher and task state machine imports
5. Comment out admin-gui imports for epics, tasks, comments, dispatcher views
6. `cargo check` — verify nothing is broken (just unused imports)

**Exit criteria:** `cargo check` passes. Old code is still in the repo but not compiled.

### Phase 2: Prepare praxis

Dependencies: Phase 1 may overlap — praxis is a separate crate.

1. Add `TransitionError` enum
2. Add `TransitionGuard` trait
3. Add `HasRole`, `HasState` traits
4. Doc comments + doctests
5. Update `praxis_doc.rs` to extract new types

**Exit criteria:** `cargo test -p nocodo_praxis` passes all new doctests.

### Phase 3: `record_spec_fact` — UserPersona only

Dependencies: Phase 2 (praxis types must exist for serialization).

Build and prove the full pipeline with just one type. Wrap existing
`PersonaNote → UserPersona` conversion in the `record_spec_fact` handler.

1. Type 1: `UserPersona` — wrap existing `PersonaNote → UserPersona` conversion
2. Add `persona_to_rust(note, session_id) -> String` serializer
3. Wire `record_spec_fact` into PO's tool set (persona fact_type only)
4. Write to `backend/src/praxis/specs/personas.rs` via schema_codegen
5. Unit test: round-trip JSON → serialize → compile against `nocodo_praxis`

**Exit criteria:** PO can call `record_spec_fact` for personas, valid Rust code
appears in the project, round-trip test passes.

### Phase 3.5: Wire the minimal loop (personas only)

Dependencies: Phase 3 (persona serializer works).

Prove the loop end-to-end before building more types. The minimal loop uses
`cargo check` as the static checker and only handles persona facts.

1. Create unified PO loop mode (`agents/src/product_owner/modes/loop.rs`) —
   single prompt, no mode switching
2. Implement `record_spec_fact` tool — PO describes a persona in structured JSON;
   backend serializes it to valid praxis Rust code and writes to project
3. Implement `trigger_codegen` tool — backend runs `cargo check` on the
   generated spec crate, parses error output, maps to gaps
4. Implement `finalize_session` tool — PO signals the system is complete
5. Backend orchestration: state machine that routes PO → codegen → checker → PO
6. Simplify `user_chat_session` — single session type, no handoff
7. End-to-end test: create session, describe a system's users, loop until
   `cargo check` passes on the generated spec

**Exit criteria:** PO conducts a conversation, persona facts are written to the
project, `cargo check` runs, gaps (compile errors) drive follow-up questions.
The loop terminates with valid, compiling persona statics.

### Phase 4: `record_spec_fact` — remaining types

Dependencies: Phase 3.5 (loop is proven with personas).

Now expand the serializer to handle all praxis types. Each type adds a variant
to the `record_spec_fact` handler and a unit test.

1. Type 2: `Permission` — new serializer
2. Type 3: `Role` — references Permission + Persona IDs
3. Type 4: `State` + `Transition` — references Role IDs, `AtLeastOne` construction
4. Type 5: `Entity` — references State IDs, `Field` with `Unresolved` invariants
5. Write each to `backend/src/praxis/specs/{module}.rs` via schema_codegen helpers
6. Unit tests: round-trip for each type

**Exit criteria:** PO can call `record_spec_fact` for all five types. Each type
has a passing round-trip unit test. The loop from Phase 3.5 works with all types.

### Phase 5: Static checker (incremental, replaces cargo check)

Dependencies: Phase 4 (needs generated spec files to check against).

Upgrade from `cargo check` parsing to a dedicated static checker with
praxis-specific gap detection. Build incrementally — each check category is
a separate unit-testable function.

1. Implement `SpecIndex` — load declared statics from project source
2. Implement `extract_id_refs` — regex-based extraction of praxis ID references
3. C1: unresolved node detection
4. C2: referential integrity checking
5. C3: guard function coverage checking
6. C4: spec completeness checking
7. `map_gaps_to_questions`
8. Unit tests for each check category
9. Integration test against todo_app spec
10. Wire checker into the loop (replaces `cargo check` parsing from Phase 3.5)

**Exit criteria:** Checker finds zero gaps on a complete spec, finds expected
gaps on an incomplete spec. The loop from Phase 3.5 uses the new checker.

### Phase 6: RustEngineer state_transition mode

Dependencies: Phase 2 (praxis types for prompt), Phase 4 (model types must be
generable via `diesel_model_struct` before transitions can reference them).

1. Create prompt builder (`build_system_prompt`, `build_user_prompt`)
2. Implement `extract_transition_metadata` (post-processing)
3. Add `run_state_transition` method on `RustEngineerAgent`
4. Add praxis write helpers to `schema_codegen`
5. Backend handler `POST /api/rust-engineer/state-transition`
6. Test with todo_app action descriptions
7. Wire `trigger_codegen` to fire `state_transition` mode (in addition to `diesel_model_struct`)

**Exit criteria:** Generated transition function compiles, static checker
validates it. `trigger_codegen` can fire both model and transition generation.

### Phase 7: End-to-end integration test

Dependencies: Phase 5, 6.

Run the full loop from scratch on the todo app. This is the proof that the
architecture works.

1. Create session → describe todo app → PO records personas, roles, permissions, states, entities
2. `trigger_codegen` fires model generation and transition guards
3. Static checker finds gaps → PO asks questions → user answers → re-generate
4. Loop terminates with zero gaps
5. Verify generated code compiles and has zero static checker gaps

**Exit criteria:** Todo app described from scratch, loop completes, generated
code compiles, static checker finds zero gaps.

### Phase 8: UI updates

Dependencies: Phase 1 (cleanup), Phase 3.5 (PO loop is functional).

1. Add spec viewer (file tree + code with provenance links)
2. Add `state_transition` to RustEngineer mode selector
3. Remove handoff navigation from user chat
4. Show gap context in user chat when PO asks gap-driven questions

**Exit criteria:** User can see generated spec and transition code from the UI,
click provenance links to jump to chat messages.

---

## 11. Risks and Open Questions

### Risks

1. **Static checker exhaustiveness** — if the checker misses a gap category,
   the loop terminates with incomplete code. Mitigation: start with the four
   categories above, add more as gaps are discovered in testing. The checker
   expands incrementally — each loop iteration may reveal new gap types.

2. **Small model reliability for transition functions** — transition functions
   are more complex than struct definitions. A ≤1B model may struggle. If
   `diesel_model` quality is the benchmark, this is TBD. Fallback: use a
   larger model for `state_transition` mode.

3. **Loop length** — a real project might need dozens of iterations. User
   fatigue is real. Mitigation: PO batches related questions; the loop is
   explicit about progress ("3 gaps remaining").

4. **Serialization correctness** — `record_spec_fact` serializers must produce
   Rust code that compiles against `nocodo_praxis`. Each type is tested
   independently with round-trip unit tests (JSON → Rust → compile). We build
   type by type, not all at once.

5. **Checker vs rustc gap** — the static checker validates praxis spec artifacts
   but doesn't run `rustc`. Generated code that passes the checker may still have
   type errors. Mitigation: add `cargo check` as a post-codegen step in the loop
   (expands checker scope without building a Rust type checker).

6. **Context window pressure on PO** — by iteration 10-15, PO's prompt contains
   the full conversation, all spec files, gap descriptions, and praxis type
   reference. For a ≤20B model, this is the dominant runtime risk. Mitigation:
   summarized gap context (not full spec dump), conversation trimming, and
   fallback to per-entity-type sessions. See §8.2.

7. **Checker verifies declared coverage, not needed coverage** — the checker
   finds gaps in *declared* transitions (e.g., missing guard functions). It
   cannot find *missing* transitions that the user never mentioned (e.g., the
   user said admins can assign tasks but never said whether they can un-assign).
   The loop may terminate with zero checker gaps but incomplete business logic.
   This is a fundamental trade: precision over coverage. The old PM role was
   doing (unreliable) coverage reasoning. The checker is more verifiable but
   less comprehensive. Explicit user review of the generated spec (Phase 8 UI)
   is the mitigation. **User expectation risk:** "zero gaps" must not be
   presented to the user as "system complete." The UI should communicate
   "no missing pieces found — review the spec to confirm it matches your
   intent" rather than implying completion. The spec viewer (§9.2) must be
   a prominent post-loop step, not a passive detail panel.

8. **Role references before declaration** — `record_spec_fact` for Roles
   (Type 3) references PermissionId and PersonaId by string. The serializer
   does not validate that these exist. The static checker catches this (C2),
   but the UX of "I described the admin role, then the system told me I haven't
   described the permissions yet" is confusing. Mitigation: the checker should
   batch all related gaps into one question, or PO should validate references
   before serializing (adds complexity to PO's prompt).

### Open questions

- **Session lifecycle:** Is a session ever "done"? Or does it stay open for
  ongoing refinement?
- **Regeneration:** When a spec fact changes (e.g., PO adds a permission to a
  role), should existing transition functions be regenerated? Or just rechecked?
- **CI integration:** Should the static checker run as a CI check on every push,
  blocking merge if gaps exist?
- **Provenance for model types:** When `diesel_model_struct` generates a `Task`
  struct, should it carry `Provenance::Conversation` pointing to the chat where
  the user described tasks? Or is provenance only for praxis spec statics?
- **Per-entity-type session splitting:** If context pressure forces splitting
  the PO loop into per-entity-type sessions (one for personas, one for roles,
  etc.), how does the backend orchestrate the handoffs? Does the checker drive
  the ordering, or does PO decide what to interview about next?
- **Transition guard quality gate:** Should there be a compilation check on
  generated transition guards before they're committed? `cargo check` on the
  transition code would catch type errors, but the function might compile and
  still be semantically wrong. Is a human review step needed, or does the
  static checker's coverage analysis suffice?
- **Model struct ordering enforcement:** When PO calls `trigger_codegen` for a
  transition guard on entity X, a Diesel model struct for X must already exist.
  Should this be enforced in PO's prompt (instruction to always generate the
  model first), in the backend handler (check for struct file existence), or by
  the static checker (C2: missing model type = referential gap)? The checker
  approach is more principled but defers the error to after codegen; the backend
  check is earlier but adds coupling. See Phase 6 tasks.

---

## 12. Principles

1. **Code is the only artifact.** Chat is the source of requirements; code is
   the encoded output. Nothing in between.
2. **Incompleteness is explicit.** `Unresolved` / `TransitionError::Unresolved`
   blocks the loop from terminating.
3. **Provenance is mandatory.** Every generated item carries a `Provenance` link
   to the chat session that produced it.
4. **The checker is exhaustive.** It must find every category of gap. If a gap
   type is discovered, add a check category.
5. **One iteration, one artifact.** The loop generates one function or one static
   per iteration. No batching that hides gaps.
6. **Deterministic where possible.** The static checker is non-LLM. PO's
   `record_spec_fact` is deterministic JSON → Rust serialization. Only code
   generation uses an LLM.
7. **Prove the loop early.** Build one type, wire the loop, prove it terminates.
   Then expand. A working loop with one type is more valuable than five
   serializers without a loop. The implementation order reflects this: Phase 3
   (persona serializer) → Phase 3.5 (minimal loop) → Phase 4 (all types).
