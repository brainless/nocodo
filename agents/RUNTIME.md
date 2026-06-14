# PRD Core: A Provenance-Aware Runtime for Requirement-Driven Systems

## Status: Design Document v0.1
## Intended audience: Claude Code, implementing `biz_core` and the spec/validation agents

---

## 1. Thesis and Motivation

Current coding agents consume PRDs as prose and emit code. The connection between
a requirement and the code it produced is implicit, lossy, and unverifiable. When
the PRD changes, there is no reliable way to know what code must change. When code
is reviewed, there is no reliable way to know what requirement it satisfies.

This system makes that connection **explicit, typed, and queryable at runtime**.

The central idea:

> A PRD is not a document that generates code. It is a typed artifact that *is*
> the runtime — every node traceable to its source, every gap surfaced as a
> first-class type.

Three consequences follow:

1. **Illegal business states are unrepresentable** — not by convention or linting,
   but by the type system itself.
2. **Incompleteness is explicit** — an `Unresolved` variant blocks code generation
   the way a compile error blocks a build. Shallow PRDs cannot produce silent gaps.
3. **Every running system can answer "why does this rule exist?"** — provenance is
   carried into production, not discarded after codegen.

The approach is not a new programming language. It is a Rust crate (`biz_core`)
that encodes the *structure* of business logic, plus a thin generated crate per
project that instantiates that structure from a PRD.

---

## 2. Architecture Overview

### 2.1 The four-layer dependency chain

```
biz_core  (human-designed, project-agnostic, versioned)
    ↑ depended on by
spec crate  (LLM-generated, project-specific, purely declarative)
    ↑ depended on by
codegen layer  (consumes spec, emits application code or wires runtime)
    ↑ depended on by
application  (runs in production, carries provenance at runtime)
```

Each boundary is a contract:

- `biz_core` → spec crate: the LLM may only use types that exist in `biz_core`.
  It cannot invent structure. If no type fits, it must emit `Unresolved`.
- spec crate → codegen: the spec is purely declarative data — no logic, no impl
  blocks, no functions. Only `static` values built from `biz_core` types.
- codegen → application: the codegen layer is currently a black box. Its input
  contract is well-defined (the spec crate). Its output mechanism is not yet
  specified. See §8.

### 2.2 Path C: The spec is the runtime

Earlier design exploration considered three paths:

- **Path A**: Separate IR, code generation step, two artifacts to keep in sync.
- **Path B**: Annotated application code, provenance in comments/attributes.
- **Path C**: The spec types *are* runtime types. No sync problem. One artifact.

This system follows **Path C**. The `Role`, `Permission`, `State`, and `Entity`
types defined in `biz_core` are used directly by middleware, permission checkers,
and state machine enforcement at runtime. The PRD-derived `static` values are the
values the application actually runs with.

Tree-sitter queries operate on the spec crate source, enabling graph queries
without a separate IR layer. The source tree *is* the queryable artifact.

---

## 3. `biz_core` Design

`biz_core` is the central artifact of the whole system. It is:

- **Not a framework** — it does not provide HTTP handlers, ORMs, or application
  structure.
- **Not a codegen tool** — it does not emit code.
- **A vocabulary crate** — it encodes the structural primitives of business logic
  the way `std` encodes the structural primitives of computation.

Every variant added to `biz_core` has a provenance: a real PRD or real project
that needed it. It grows from evidence, not anticipation. See §7.

### 3.1 Primitives

```rust
// biz_core::primitives

/// A collection that must have at least one element.
/// The fundamental encoding of "this business rule requires a responsible party."
/// An Action with no permitted Role, a Transition with no permitted Role —
/// these are structurally unrepresentable.
pub struct AtLeastOne<T: 'static> {
    pub head: T,
    pub tail: &'static [T],
}

impl<T> AtLeastOne<T> {
    pub fn all(&self) -> impl Iterator<Item = &T> {
        std::iter::once(&self.head).chain(self.tail.iter())
    }

    pub fn contains<F: Fn(&T) -> bool>(&self, f: F) -> bool {
        self.all().any(f)
    }
}

/// First-class incompleteness.
/// Anything that cannot be encoded in current biz_core types must be wrapped
/// in Unresolved rather than approximated. This blocks downstream codegen
/// explicitly and surfaces the gap to the clarification loop.
///
/// Blocked variant: for Unresolvedss that cannot be resolved until another
/// Unresolved is resolved first. Encodes dependency between gaps.
pub enum Unresolved<T> {
    Resolved(T),
    Pending {
        reason: &'static str,
        provenance: &'static [Provenance],
    },
    Blocked {
        by: &'static [&'static str], // IDs of blocking Unresolvedss
        reason: &'static str,
    },
}

impl<T> Unresolved<T> {
    pub fn is_resolved(&self) -> bool {
        matches!(self, Unresolved::Resolved(_))
    }

    pub fn blocks_codegen(&self) -> bool {
        !self.is_resolved()
    }
}
```

**Design note on `AtLeastOne<T>`**: This is the canonical example of encoding
business constraints as types rather than conventions. The constraint "an action
must have at least one permitted role" is not a linter rule, not a code review
comment, not a test — it is structurally unrepresentable to violate. Every similar
business constraint should be evaluated for promotion to a type.

### 3.2 Provenance

```rust
// biz_core::provenance

/// Where did this requirement come from?
/// Every node in the spec graph carries at least one Provenance.
/// Multiple provenances are allowed — a requirement may be stated in
/// a Jira ticket and clarified in a follow-up conversation.
#[derive(Debug, Clone)]
pub enum Provenance {
    Conversation {
        id: &'static str,
        excerpt: &'static str,   // verbatim original text
    },
    JiraTicket {
        id: &'static str,
        url: &'static str,
        excerpt: &'static str,
    },
    GitCommit {
        sha: &'static str,
        path: &'static str,
        lines: (u32, u32),
        excerpt: &'static str,
    },
    ConfluencePage {
        url: &'static str,
        anchor: Option<&'static str>,
        excerpt: &'static str,
    },
    File {
        path: &'static str,
        lines: (u32, u32),
        excerpt: &'static str,
    },
    /// The LLM inferred this from context — it was not explicitly stated.
    /// This variant is critical: it makes LLM inference visible and
    /// independently checkable. Inferred nodes are candidates for
    /// clarification-loop review.
    Inferred {
        reason: &'static str,
        from: &'static [&'static str], // IDs of source nodes that led to this
    },
}

/// A value that carries its PRD origin into the runtime.
/// The running application can answer "why does this config value exist?"
pub struct PrdValue<T> {
    pub value: T,
    pub provenance: &'static [Provenance],
}
```

**Design note on `Provenance::Inferred`**: When the LLM infers something not
explicitly stated in the PRD (e.g., `VIEW_ALL_TASKS` from "team coordination
implies visibility"), it must use `Inferred` rather than any source provenance.
This makes the inference visible to the validation agent and to human reviewers.
Inferred nodes are weaker than stated nodes and should be prioritised for
clarification.

### 3.3 Auth and Permissions

```rust
// biz_core::auth

pub struct RoleId(pub &'static str);
pub struct PermissionId(pub &'static str);
pub struct PersonaId(pub &'static str);

/// How a role's effective permissions are computed.
/// This encodes the intent; the auth library (Casbin, oso, custom) implements it.
/// The spec does not hardcode the mechanism — it expresses the intent.
pub enum RoleSemantics {
    /// Permissions are exactly what is listed. Nothing inherited.
    Flat,
    /// Effective permissions = own permissions + parent's effective permissions.
    /// Resolved recursively at runtime by the auth layer.
    Inherits { parent: RoleId },
    /// Effective permissions = union of all listed roles' effective permissions.
    Union { of: &'static [RoleId] },
}

pub struct Role {
    pub id: RoleId,
    pub description: &'static str,
    pub semantics: RoleSemantics,
    /// Own permissions only. Inherited permissions are not listed here.
    pub permissions: &'static [PermissionId],
    pub personas: &'static [PersonaId],  // which personas typically hold this role
    pub provenance: &'static [Provenance],
}

pub struct Permission {
    pub id: PermissionId,
    pub description: &'static str,
    pub provenance: &'static [Provenance],
}

pub struct UserPersona {
    pub id: PersonaId,
    pub name: &'static str,
    pub description: &'static str,
    pub goals: &'static [&'static str],
    pub pain_points: &'static [&'static str],
    pub provenance: &'static [Provenance],
}

/// Explicit role for any authenticated user, regardless of assigned role.
/// This replaces the concept of "default" visibility — there is no default.
/// Every action has an explicit actor, even if that actor is all authenticated users.
///
/// The spec crate should declare a static of this type when needed:
///   pub static ALL_AUTHENTICATED: ImplicitRole = ImplicitRole::AnyAuthenticated;
pub enum ImplicitRole {
    AnyAuthenticated,
    AnyUser,         // including unauthenticated — use with care
}
```

**Design note on `ImplicitRole`**: The absence of an actor is an `Unresolved`,
not a default. `ImplicitRole::AnyAuthenticated` is an explicit choice that appears
in the spec, is carried into the permission check, and is queryable by tree-sitter.
"Default" visibility is a hidden assumption; `ImplicitRole` makes it visible.

### 3.4 State Machine

```rust
// biz_core::statemachine

pub struct StateId(pub &'static str);

/// Under what circumstances may a transition occur?
/// Conditions are coupled to permissions — the state machine and the
/// permission system are not independent.
pub enum TransitionCondition {
    Always,
    OnlyIfAssignedToSelf,
    OnlyIfAssignedTo(RoleId),
    RequiresPermission(PermissionId),
    /// Compound: all conditions must hold.
    All(&'static [TransitionCondition]),
    /// Compound: any condition must hold.
    Any(&'static [TransitionCondition]),
    /// The PRD did not specify. Blocks codegen. Triggers clarification loop.
    Unresolved(&'static str),
}

pub struct Transition {
    pub to: StateId,
    /// Who may trigger this transition. Structurally requires at least one role.
    pub permitted_roles: AtLeastOne<RoleId>,
    pub condition: TransitionCondition,
    pub provenance: &'static [Provenance],
}

/// Terminal states have no outgoing transitions.
/// Non-terminal states must have at least one — Transitions::To enforces this.
pub enum Transitions {
    /// No exit from this state. Structurally enforced.
    Terminal,
    /// At least one outgoing transition. Structurally enforced.
    To(AtLeastOne<Transition>),
}

pub struct State {
    pub id: StateId,
    pub description: &'static str,
    pub transitions: Transitions,
    pub provenance: &'static [Provenance],
}
```

**Design note on terminal states**: `Transitions::Terminal` makes the concept of
a final state explicit and structurally distinct from "we forgot to add transitions."
A state machine soundness query can verify: does every entity have at least one
`Terminal` state? This is a tree-sitter query, not a runtime check.

**Design note on transition/permission coupling**: `Transition` carries both
`permitted_roles` and `condition`. These are not the same thing. `permitted_roles`
is about identity ("who"). `condition` is about circumstance ("under what
situation"). Both must be satisfied. The coupling is intentional and explicit.

### 3.5 Entity

```rust
// biz_core::entity

pub struct EntityId(pub &'static str);

/// An assignee that cannot be in an ambiguous state.
/// Option<UserId> is banned — None is ambiguous between "unassigned"
/// and "not yet decided." This enum is not.
pub enum Assignee<Id> {
    Unassigned,
    AssignedTo(Id),
    /// Assignment policy not yet decided. Blocks codegen.
    Unresolved(&'static str),
}

pub struct Field {
    pub name: &'static str,
    pub description: &'static str,
    /// Field-level invariants. Each is either resolved (a concrete rule)
    /// or pending (a question that must be answered before codegen).
    pub invariants: &'static [Unresolved<&'static str>],
    pub provenance: &'static [Provenance],
}

pub struct Entity {
    pub id: EntityId,
    pub description: &'static str,
    pub fields: &'static [Field],
    pub states: &'static [State],
    /// Cross-field or cross-entity rules.
    /// e.g. "a task cannot have more than one active assignee"
    pub invariants: &'static [Unresolved<&'static str>],
    pub provenance: &'static [Provenance],
}
```

**Design note on newtypes for field values**: Individual field values (e.g.,
`TaskTitle`) should be newtypes in the spec crate, not raw `String`. The newtype
constructor enforces business constraints (non-empty, max length) at the boundary.
If the PRD does not specify a constraint (e.g., max title length), the constructor
must emit `Unresolved` rather than guess. The tree-sitter validation agent checks
that every `String`-wrapping newtype has an explicit constraint or an explicit
`Unresolved`.

---

## 4. Spec Crate Design — Todo App Worked Example

The spec crate is purely declarative. No logic, no impl blocks. Only `static`
values composed from `biz_core` types.

```rust
// todo_app_spec/src/lib.rs

use biz_core::{
    auth::{Role, RoleId, RoleSemantics, Permission, PermissionId,
           UserPersona, PersonaId, ImplicitRole},
    statemachine::{State, StateId, Transition, Transitions, TransitionCondition},
    entity::{Entity, EntityId, Field, Assignee},
    primitives::AtLeastOne,
    provenance::Provenance,
};

// ── Provenance source ────────────────────────────────────────────────────────

const INIT_CONVERSATION: Provenance = Provenance::Conversation {
    id: "2024-init",
    excerpt: "A simple Todo app. Email/password auth. Anyone who registers can \
              self-assign and self-manage. Admin can create tasks, kick members \
              out, assign to anyone.",
};

// ── Personas ─────────────────────────────────────────────────────────────────

pub static PERSONA_MEMBER: UserPersona = UserPersona {
    id: PersonaId("member"),
    name: "Team Member",
    description: "Anyone who has registered and confirmed their account",
    goals: &["Track my own tasks", "See what my team is working on"],
    pain_points: &[],
    provenance: &[INIT_CONVERSATION],
};

pub static PERSONA_ADMIN: UserPersona = UserPersona {
    id: PersonaId("admin"),
    name: "Admin",
    description: "Elevated user who manages team and task assignment",
    goals: &["Manage team membership", "Create and assign tasks to anyone"],
    pain_points: &[],
    provenance: &[INIT_CONVERSATION],
};

// ── Permissions ───────────────────────────────────────────────────────────────

pub static PERM_VIEW_ALL_TASKS: Permission = Permission {
    id: PermissionId("view_all_tasks"),
    description: "Read access to all tasks regardless of assignee",
    provenance: &[Provenance::Inferred {
        reason: "Team coordination requires shared visibility",
        from: &["2024-init"],
    }],
};

pub static PERM_SELF_ASSIGN_TASK: Permission = Permission {
    id: PermissionId("self_assign_task"),
    description: "Assign an unassigned task to oneself",
    provenance: &[INIT_CONVERSATION],
};

pub static PERM_UPDATE_OWN_TASK_STATUS: Permission = Permission {
    id: PermissionId("update_own_task_status"),
    description: "Transition status of a task assigned to oneself",
    provenance: &[INIT_CONVERSATION],
};

pub static PERM_CREATE_TASK: Permission = Permission {
    id: PermissionId("create_task"),
    description: "Create a new task",
    provenance: &[INIT_CONVERSATION],
};

pub static PERM_ASSIGN_TASK_TO_ANYONE: Permission = Permission {
    id: PermissionId("assign_task_to_anyone"),
    description: "Assign any task to any member",
    provenance: &[INIT_CONVERSATION],
};

pub static PERM_REMOVE_MEMBER: Permission = Permission {
    id: PermissionId("remove_member"),
    description: "Remove a member from the team",
    provenance: &[INIT_CONVERSATION],
};

// ── Roles ─────────────────────────────────────────────────────────────────────

pub static ROLE_ALL_AUTHENTICATED: Role = Role {
    id: RoleId("all_authenticated"),
    description: "Any user with a valid session — explicit, not a default",
    semantics: RoleSemantics::Flat,
    permissions: &[PermissionId("view_all_tasks")],
    personas: &[PersonaId("member"), PersonaId("admin")],
    provenance: &[Provenance::Inferred {
        reason: "Visibility must have an explicit actor; all authenticated users \
                 is that actor",
        from: &["2024-init"],
    }],
};

pub static ROLE_MEMBER: Role = Role {
    id: RoleId("member"),
    description: "Registered team member",
    semantics: RoleSemantics::Inherits { parent: RoleId("all_authenticated") },
    permissions: &[
        PermissionId("self_assign_task"),
        PermissionId("update_own_task_status"),
    ],
    personas: &[PersonaId("member")],
    provenance: &[INIT_CONVERSATION],
};

pub static ROLE_ADMIN: Role = Role {
    id: RoleId("admin"),
    description: "Team admin",
    // Inherits member permissions — admin can also self-manage their own tasks.
    // UNRESOLVED Q6: confirm admin inherits member, not just flat permissions.
    semantics: RoleSemantics::Inherits { parent: RoleId("member") },
    permissions: &[
        PermissionId("create_task"),
        PermissionId("assign_task_to_anyone"),
        PermissionId("remove_member"),
    ],
    personas: &[PersonaId("admin")],
    provenance: &[INIT_CONVERSATION],
};

// ── Task State Machine ────────────────────────────────────────────────────────

pub static STATE_TODO: State = State {
    id: StateId("todo"),
    description: "Task created, not yet started",
    transitions: Transitions::To(AtLeastOne {
        head: Transition {
            to: StateId("in_progress"),
            permitted_roles: AtLeastOne {
                head: RoleId("member"),
                tail: &[RoleId("admin")],
            },
            condition: TransitionCondition::OnlyIfAssignedToSelf,
            provenance: &[INIT_CONVERSATION],
        },
        tail: &[Transition {
            to: StateId("cancelled"),
            permitted_roles: AtLeastOne {
                head: RoleId("admin"),
                tail: &[],
                // UNRESOLVED Q4: can a member cancel their own task?
            },
            condition: TransitionCondition::Always,
            provenance: &[Provenance::Inferred {
                reason: "Cancellation implied by task lifecycle norms",
                from: &["2024-init"],
            }],
        }],
    }),
    provenance: &[INIT_CONVERSATION],
};

pub static STATE_IN_PROGRESS: State = State {
    id: StateId("in_progress"),
    description: "Task actively being worked on",
    transitions: Transitions::To(AtLeastOne {
        head: Transition {
            to: StateId("done"),
            permitted_roles: AtLeastOne {
                head: RoleId("member"),
                tail: &[RoleId("admin")],
            },
            condition: TransitionCondition::OnlyIfAssignedToSelf,
            provenance: &[INIT_CONVERSATION],
        },
        tail: &[
            Transition {
                to: StateId("cancelled"),
                permitted_roles: AtLeastOne {
                    head: RoleId("admin"),
                    tail: &[],
                },
                condition: TransitionCondition::Always,
                provenance: &[Provenance::Inferred {
                    reason: "Admin can cancel at any stage",
                    from: &["2024-init"],
                }],
            },
            Transition {
                to: StateId("todo"),
                permitted_roles: AtLeastOne {
                    head: RoleId("admin"),
                    tail: &[RoleId("member")],
                },
                // UNRESOLVED Q1: can in_progress revert to todo?
                // Currently represented — awaiting client confirmation.
                condition: TransitionCondition::Unresolved(
                    "Client has not specified whether tasks can be un-started"
                ),
                provenance: &[Provenance::Inferred {
                    reason: "Common product pattern; not stated in PRD",
                    from: &["2024-init"],
                }],
            },
        ],
    }),
    provenance: &[INIT_CONVERSATION],
};

pub static STATE_DONE: State = State {
    id: StateId("done"),
    description: "Task completed. Terminal.",
    transitions: Transitions::Terminal,
    provenance: &[INIT_CONVERSATION],
};

pub static STATE_CANCELLED: State = State {
    id: StateId("cancelled"),
    description: "Task cancelled.",
    // UNRESOLVED Q2: can cancelled be reopened?
    // Currently Terminal — awaiting client confirmation.
    transitions: Transitions::Terminal,
    provenance: &[INIT_CONVERSATION],
};

// ── Task Entity ───────────────────────────────────────────────────────────────

pub static ENTITY_TASK: Entity = Entity {
    id: EntityId("task"),
    description: "A unit of work that can be assigned and tracked",
    fields: &[
        Field {
            name: "title",
            description: "Short human-readable name for the task",
            // UNRESOLVED Q5: maximum title length not specified in PRD
            invariants: &[biz_core::primitives::Unresolved::Pending {
                reason: "Maximum title length not specified",
                provenance: &[Provenance::Inferred {
                    reason: "All text fields require a length bound for storage",
                    from: &["2024-init"],
                }],
            }],
            provenance: &[INIT_CONVERSATION],
        },
        Field {
            name: "status",
            description: "Current lifecycle state of the task",
            invariants: &[],
            provenance: &[INIT_CONVERSATION],
        },
        Field {
            name: "assignee",
            description: "Who is responsible for this task",
            invariants: &[],
            provenance: &[INIT_CONVERSATION],
        },
    ],
    states: &[
        STATE_TODO,
        STATE_IN_PROGRESS,
        STATE_DONE,
        STATE_CANCELLED,
    ],
    invariants: &[],
    provenance: &[INIT_CONVERSATION],
};
```

---

## 5. Validation Stack

Three layers. Each catches what the layer above cannot.

### 5.1 Tree-sitter Agent

Operates on spec crate source as a graph. Runs after LLM generation, before
compilation. Uses the Rust tree-sitter grammar (mature, production-ready).

**Referential integrity queries**

```scheme
; Find every RoleId string literal used in a Transition
(struct_expression
  (field_initializer
    (field_identifier) @field (#eq? @field "permitted_roles")
    (call_expression
      (arguments (string_literal) @role_ref))))

; Cross-reference against declared Role statics
; Any @role_ref not matching a declared Role id → integrity error
```

**Completeness queries**

```scheme
; Find all Unresolved::Pending nodes — these block codegen
(call_expression
  (scoped_identifier
    (identifier) @variant (#eq? @variant "Pending"))
  (arguments
    (string_literal) @reason))
; → report each as a named gap with its reason
```

**State machine soundness**

```scheme
; Find entities with no Terminal state
; (all states reference Transitions::To, none reference Transitions::Terminal)
```

**Inferred node audit**

```scheme
; Find all Provenance::Inferred nodes — candidates for human review
(struct_expression
  (type_identifier) @type (#eq? @type "Inferred")
  (field_initializer
    (field_identifier) @field (#eq? @field "reason")
    (string_literal) @reason))
```

Tree-sitter queries are co-versioned with `biz_core`. A new type in `biz_core`
ships with corresponding queries. The query library is a first-class artifact.

### 5.2 `rustc` Layer

Standard Rust compilation. Catches:

- Type mismatches (wrong `RoleId` type in a `PermissionId` position)
- Missing fields in struct expressions
- Invalid enum variants
- `AtLeastOne` structural requirements (if enforced via const generics or
  sealed traits)

This layer is free — it requires no additional tooling.

### 5.3 Generated Tests

Generated by a separate agent that reads the spec crate and emits behavioral
tests. The spec is the source of truth; the tests verify the application honors it.

Examples from the Todo spec:

```rust
// Generated from ROLE_ADMIN.permissions and ROLE_MEMBER.semantics
#[test]
fn admin_can_create_task() { ... }

#[test]
fn member_cannot_create_task() { ... }

// Generated from STATE_TODO transitions
#[test]
fn todo_can_transition_to_in_progress_if_assigned_to_self() { ... }

#[test]
fn todo_cannot_transition_to_done_directly() { ... }

// Generated from Unresolved nodes — these are pending tests, not failing tests
#[test]
#[ignore = "UNRESOLVED Q1: client has not confirmed whether tasks can be un-started"]
fn in_progress_can_revert_to_todo() { ... }
```

`#[ignore]` with a structured reason ties pending tests to their `Unresolved`
nodes. When a gap is resolved and the spec updated, the test is un-ignored.

---

## 6. Agent Architecture

### 6.1 Spec Agent

**Input**: PRD prose (any format — Jira, Confluence, Markdown, conversation)
**Output**: A valid spec crate using only `biz_core` vocabulary
**Constraint**: May not invent types. May not emit bare `String` where a
`biz_core` type exists. Must emit `Unresolved` for anything the vocabulary
cannot express.

The spec agent's system prompt includes the full `biz_core` API as context.
It is told explicitly: if you cannot find a `biz_core` type that fits, emit
`Unresolved` with a clear reason. Do not approximate.

The `Provenance::Inferred` variant must be used whenever the agent encodes
something not explicitly stated in the PRD. This makes LLM inference visible
and independently auditable.

### 6.2 Validation Agent

**Input**: Generated spec crate source
**Output**: A structured report of errors, each with source location

Error categories:

| Category | Source | Example |
|---|---|---|
| Referential integrity | Tree-sitter | `RoleId("foo")` not declared |
| Completeness | Tree-sitter | `Unresolved::Pending` in codegen-ready path |
| Soundness | Tree-sitter | Entity with no Terminal state |
| Structural | `rustc` | Type mismatch |
| Behavioral | Generated tests | Permission check fails |

Each error is returned to the spec agent with exact source location and
error category. The spec agent does not receive the full spec back — only
the error and its location. This prevents re-hallucinating already-correct
sections.

### 6.3 The Clarification Loop

```
PRD prose
    │
    ▼
Spec Agent → spec crate (may contain Unresolved nodes)
    │
    ▼
Validation Agent
    ├── Referential errors → back to Spec Agent (self-correctable)
    ├── Structural errors  → back to Spec Agent (self-correctable)
    └── Unresolved nodes  → Clarification Agent
                                │
                                ▼
                           Structured questions to human/client
                                │
                                ▼
                           Answers incorporated into PRD
                                │
                                ▼
                           Spec Agent re-runs affected nodes only
```

The clarification agent clusters `Unresolved` nodes by theme before asking
questions — it does not ask one question per node. Related gaps are resolved
together. This mirrors how a good engineer would interrogate a shallow PRD.

**Key property**: the loop terminates when the validation agent finds zero
`Unresolved::Pending` nodes in any codegen-ready path, zero referential
errors, and all structural checks pass. At that point, the spec is complete
enough to generate code. "Complete enough" is defined by the spec, not by
the agent.

---

## 7. `biz_core` Expansion Mechanism

`biz_core` starts minimal. It grows from **observed `Unresolved` patterns**,
not from anticipation.

The process:

```
1. Spec agent emits Unresolved("need TransitionCondition for time-based rules")
2. Multiple projects emit similar Unresolvedss
3. Human reviews the cluster
4. New variant added: TransitionCondition::AfterDuration(Duration)
5. biz_core minor version bumped
6. All specs with matching Unresolved re-run; Pending → Resolved
```

Rules for expansion:

- A new variant requires at least one real PRD that needed it
- A new variant must be expressible without breaking existing specs (additive)
- A breaking change (rename, removal) is a major version bump; all specs
  must be regenerated
- Every variant in `biz_core` has a `provenance` in the changelog: the project
  and PRD excerpt that motivated it

Over time, `biz_core` becomes an **archaeological record of business logic
patterns** encountered across all projects. Each version of `biz_core` represents
accumulated structural knowledge. This is a compounding asset — later projects
benefit from patterns discovered by earlier ones.

### Versioning policy

```toml
# Semver for biz_core:
# PATCH: bug fixes, doc improvements, no API change
# MINOR: new variants, new types, additive only
# MAJOR: any breaking change to existing types or variants
```

The `biz_core` changelog is itself a structured document — each entry records
the variant added, the `Unresolved` pattern that motivated it, and the PRD
excerpt that first surfaced it. The changelog is queryable.

---

## 8. Open Questions

These are explicitly deferred. They are not design gaps — they are known unknowns
that require further iteration.

### 8.1 CLI / System Actor (Bootstrap problem)

Who is the first admin? Registration is self-serve, but someone must bootstrap
the admin role. The type system has no `SystemActor` concept yet.

Direction: a `SystemActor` variant in `biz_core::auth` for actions taken by
CLI, CI, or SSH — outside the normal auth flow. Bootstrap actions (seed admin,
run migrations) are `SystemActor` actions, not `Role` actions.

### 8.2 View Layer

`ViewComponent` was sketched but not designed. It is significantly more complex
than the auth and state machine layers — conditional visibility, pagination
contracts, real-time semantics, form validation. Deferred to a separate design
session.

Direction: views are the last layer to design, after entities, permissions, and
state machines are stable. The view layer depends on all others; the others do
not depend on the view layer.

### 8.3 Codegen Layer

The codegen layer consumes the spec crate and emits application code (or wires
runtime types). Its input contract is well-defined (the spec crate API). Its
output mechanism — what framework, what conventions, how it maps `State` to a
database column — is not yet specified.

Direction: codegen is a separate agent that reads the spec crate as a library
and emits code per target (Axum handler, SQLx migration, etc.). Multiple codegen
targets are possible from the same spec.

### 8.4 Tree-sitter Query Library Versioning

Tree-sitter queries must be co-versioned with `biz_core`. A `biz_core` minor
version that adds a new type must ship with queries covering that type. The
mechanism for this co-versioning is not yet specified.

Direction: the query library lives in the `biz_core` repository as a sibling
crate (`biz_core_queries`), versioned together.

### 8.5 Multi-agent Correlated Error Risk

Two LLMs trained on similar data will make correlated errors. The mitigation —
the spec agent sees only the PRD; the test-generating agent sees only the spec,
not the PRD — ensures the only communication channel between them is the typed
IR. Agreement under this constraint is meaningful signal. Disagreement surfaces
ambiguity. This property should be preserved in all agent configurations.

---

## Appendix: Design Principles Summary

1. **Encode business constraints as types, not conventions.**
   If a business rule can be expressed as a type, it must be. `AtLeastOne<T>`
   over `Vec<T>`. Enum over `Option`. Newtype over `String`.

2. **Incompleteness is first-class.**
   `Unresolved` is not a failure mode — it is the system working correctly.
   A shallow PRD produces many `Unresolved` nodes. That is the correct output.

3. **Inference is visible.**
   `Provenance::Inferred` marks every LLM assumption. Inferred nodes are weaker
   than stated nodes and are prioritised for human review.

4. **The spec is the runtime.**
   No separate IR. No sync problem. The `biz_core` types used in the spec are
   the types used in the running application.

5. **Tree-sitter queries replace the IR query layer.**
   The source tree is the queryable artifact. Queries are co-versioned with
   `biz_core` and are first-class artifacts, not afterthoughts.

6. **`biz_core` grows from evidence.**
   Every variant has a provenance. The expansion mechanism is observable,
   reviewable, and reversible. Anticipatory design is avoided.

7. **Agent separation is load-bearing.**
   The spec agent and the validation agent have strictly separated concerns.
   The spec agent does not validate. The validation agent does not generate.
   The clarification agent does not do either. This separation is what makes
   the loop's outputs verifiable.

