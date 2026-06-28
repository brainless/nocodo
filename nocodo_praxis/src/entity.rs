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
//! use nocodo_praxis::statemachine::{
//!     State, StateId, Transition, TransitionCondition, Transitions,
//! };
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
//! static TASK_STATES: [State; 2] = [
//!     State {
//!         id: StateId("todo"),
//!         description: "Task created, not yet started",
//!         transitions: Transitions::To(AtLeastOne {
//!             head: Transition {
//!                 to: StateId("done"),
//!                 permitted_roles: AtLeastOne { head: RoleId("member"), tail: &[] },
//!                 condition: TransitionCondition::Always,
//!                 provenance: PROV_SLICE,
//!             },
//!             tail: &[],
//!         }),
//!         provenance: PROV_SLICE,
//!     },
//!     State {
//!         id: StateId("done"),
//!         description: "Task completed.",
//!         transitions: Transitions::Terminal,
//!         provenance: PROV_SLICE,
//!     },
//! ];
//!
//! let task = Entity {
//!     id: EntityId("task"),
//!     description: "A unit of work that can be assigned and tracked",
//!     fields: &FIELDS,
//!     states: &TASK_STATES,
//!     invariants: &[],
//!     provenance: PROV_SLICE,
//! };
//!
//! // Check if any invariants are pending
//! assert!(task.has_pending_invariants());
//! ```

use super::primitives::Unresolved;
use super::provenance::Provenance;
use super::statemachine::State;

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
    /// The entity's lifecycle states.
    pub states: &'static [State],
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

    /// Find a state by its [`StateId`](super::statemachine::StateId) within
    /// this entity's state list.
    pub fn find_state(&self, id: &super::statemachine::StateId) -> Option<&State> {
        super::statemachine::find_state(id, self.states)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::{AtLeastOne, Unresolved};
    use crate::provenance::Provenance;
    use crate::statemachine::{State, StateId, Transition, TransitionCondition, Transitions};

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
    fn find_state_on_entity() {
        static STATES: [State; 1] = [State {
            id: StateId("todo"),
            description: "",
            transitions: Transitions::To(AtLeastOne {
                head: Transition {
                    to: StateId("done"),
                    permitted_roles: AtLeastOne {
                        head: crate::auth::RoleId("member"),
                        tail: &[],
                    },
                    condition: TransitionCondition::Always,
                    provenance: &[],
                },
                tail: &[],
            }),
            provenance: &[],
        }];
        let entity = Entity {
            id: EntityId("task"),
            description: "",
            fields: &[],
            states: &STATES,
            invariants: &[],
            provenance: &[],
        };
        assert!(entity.find_state(&StateId("todo")).is_some());
        assert!(entity.find_state(&StateId("nonexistent")).is_none());
    }
}
