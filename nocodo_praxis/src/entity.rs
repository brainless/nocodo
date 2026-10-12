//! Domain objects: entities, their fields, and their invariants.
//!
//! An [`Entity`] is a domain object with named fields, a lifecycle defined by
//! [`State`]s, and cross-field/cross-entity invariants. Incompleteness in any
//! field invariant or entity invariant is surfaced through [`Unresolved`].
//!
//! [`State`]: super::statemachine::State
//! [`Unresolved`]: super::primitives::Unresolved
//!
//! # Examples
//!
//! ```
//! use nocodo_praxis::auth::RoleId;
//! use nocodo_praxis::entity::{Entity, EntityId, Field};
//! use nocodo_praxis::primitives::{AtLeastOne, Unresolved};
//! use nocodo_praxis::provenance::Provenance;
//! use nocodo_praxis::statemachine::StateId;
//!
//! static PROV_SLICE: &[Provenance] = &[Provenance::Conversation {
//!     id: "prd-1",
//!     excerpt: "A task has a title and a status.",
//! }];
//!
//! static TITLE_INVARIANTS: [Unresolved<&str>; 1] = [Unresolved::Pending {
//!     reason: "Maximum title length not specified in PRD",
//!     provenance: AtLeastOne {
//!         head: Provenance::Inferred {
//!             reason: "All text fields require a length bound for storage",
//!             from: &["prd-1"],
//!         },
//!         tail: &[],
//!     },
//! }];
//!
//! static FIELDS: [Field; 2] = [
//!     Field {
//!         name: "title",
//!         description: "Short human-readable name for the task",
//!         invariants: &TITLE_INVARIANTS,
//!         provenance: PROV_SLICE,
//!     },
//!     Field {
//!         name: "status",
//!         description: "Current lifecycle state of the task",
//!         invariants: &[],
//!         provenance: PROV_SLICE,
//!     },
//! ];
//!
//! // States are declared independently (e.g. via a separate `record_spec_fact`
//! // call per state — see MILESTONE_PRAXIS_v1.md §5.6/§5.7). `Entity::states`
//! // references them by `StateId`, the same cross-reference pattern used
//! // everywhere else in this crate (`Role::permissions: &[PermissionId]`,
//! // `Transition::to: StateId`, etc.) — not by embedding full `State` values,
//! // which would require every entity to duplicate (and keep in sync) the
//! // complete state machine of every state it lists.
//!
//! let task = Entity {
//!     id: EntityId("task"),
//!     description: "A unit of work that can be assigned and tracked",
//!     fields: &FIELDS,
//!     states: &[StateId("todo"), StateId("done")],
//!     invariants: &[],
//!     provenance: PROV_SLICE,
//! };
//!
//! // Check if any invariants are pending
//! assert!(task.has_pending_invariants());
//! assert!(task.has_state(StateId("todo")));
//! assert!(!task.has_state(StateId("archived")));
//! ```

use super::primitives::Unresolved;
use super::provenance::Provenance;
use super::statemachine::StateId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// A unique identifier for an entity (e.g. `EntityId("task")`).
pub struct EntityId(pub &'static str);

#[derive(Debug, Clone)]
/// An assignee that cannot be in an ambiguous state.
///
/// `Option<UserId>` is banned — `None` is ambiguous between "unassigned" and
/// "not yet decided." This enum is explicit about each case.
pub enum Assignee<Id> {
    /// No one is assigned.
    Unassigned,
    /// A specific entity is assigned.
    AssignedTo(Id),
    /// Assignment policy not yet decided. Blocks codegen.
    Unresolved(&'static str),
}

#[derive(Debug, Clone)]
/// A field on an entity, with its invariants.
///
/// Each invariant is an [`Unresolved`] — either a concrete rule that codegen
/// can enforce, or a pending question that must be answered first.
pub struct Field {
    /// Field name (e.g. `"title"`, `"status"`).
    pub name: &'static str,
    /// Human-readable description of what this field represents.
    pub description: &'static str,
    /// Field-level invariants. Each is either resolved (a concrete rule like
    /// `"max 200 chars"`) or pending (a question like `"max length?"`).
    pub invariants: &'static [Unresolved<&'static str>],
    /// Where this field and its invariants were defined.
    pub provenance: &'static [Provenance],
}

#[derive(Debug, Clone)]
/// A domain entity with named fields, lifecycle states, and invariants.
///
/// An entity models a business object: a Task, a User, an Order. Its lifecycle
/// is defined by its `states` (a state machine). Cross-field and cross-entity
/// rules live in `invariants`.
pub struct Entity {
    /// Unique identifier (e.g. `EntityId("task")`).
    pub id: EntityId,
    /// Human-readable description of what this entity represents.
    pub description: &'static str,
    /// The entity's data fields.
    pub fields: &'static [Field],
    /// The entity's lifecycle states, referenced by [`StateId`] — [`State`]
    /// definitions are declared independently (their own `static`s) and
    /// looked up via a spec-wide index, not embedded here. This mirrors
    /// every other cross-reference in this crate (`Role::permissions:
    /// &[PermissionId]`, `Transition::to: StateId`): a full `State` carries
    /// its own transitions and can't be duplicated per-entity without the
    /// copies drifting out of sync.
    ///
    /// [`State`]: super::statemachine::State
    pub states: &'static [StateId],
    /// Cross-field or cross-entity invariants (e.g. "a task cannot have more
    /// than one active assignee").
    pub invariants: &'static [Unresolved<&'static str>],
    /// Where this entity was defined in the PRD.
    pub provenance: &'static [Provenance],
}

impl Entity {
    /// Returns `true` if any field invariant or entity invariant is still
    /// pending or blocked — codegen cannot proceed until these are resolved.
    pub fn has_pending_invariants(&self) -> bool {
        self.fields
            .iter()
            .any(|f| f.invariants.iter().any(|i| i.blocks_codegen()))
            || self.invariants.iter().any(|i| i.blocks_codegen())
    }

    /// Returns `true` if `id` is one of this entity's declared lifecycle
    /// states.
    pub fn has_state(&self, id: StateId) -> bool {
        self.states.contains(&id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::{AtLeastOne, Unresolved};
    use crate::provenance::Provenance;
    use crate::statemachine::StateId;

    #[test]
    fn entity_with_pending_field_invariant_returns_true() {
        let entity = Entity {
            id: EntityId("task"),
            description: "",
            fields: &[Field {
                name: "title",
                description: "",
                invariants: &[Unresolved::Pending {
                    reason: "max length not specified",
                    provenance: AtLeastOne {
                        head: Provenance::Conversation {
                            id: "t",
                            excerpt: "x",
                        },
                        tail: &[],
                    },
                }],
                provenance: &[],
            }],
            states: &[],
            invariants: &[],
            provenance: &[],
        };
        assert!(entity.has_pending_invariants());
    }

    #[test]
    fn entity_with_all_resolved_invariants_returns_false() {
        let entity = Entity {
            id: EntityId("task"),
            description: "",
            fields: &[Field {
                name: "title",
                description: "",
                invariants: &[Unresolved::Resolved("max 200 chars")],
                provenance: &[],
            }],
            states: &[],
            invariants: &[],
            provenance: &[],
        };
        assert!(!entity.has_pending_invariants());
    }

    #[test]
    fn has_state_on_entity() {
        let entity = Entity {
            id: EntityId("task"),
            description: "",
            fields: &[],
            states: &[StateId("todo"), StateId("done")],
            invariants: &[],
            provenance: &[],
        };
        assert!(entity.has_state(StateId("todo")));
        assert!(!entity.has_state(StateId("nonexistent")));
    }
}
