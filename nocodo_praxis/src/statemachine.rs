//! Lifecycle modeling for domain entities.
//!
//! Types for defining state machines: which states an entity can be in, which
//! transitions are permitted between them, and under what conditions transitions
//! may occur.
//!
//! State machine soundness is verifiable by tree-sitter query:
//! - Does every entity have at least one `Terminal` state?
//! - Are there any `Unresolved` transition conditions?
//! - Can every non-terminal state reach a terminal state?
//!
//! # Examples
//!
//! ```
//! use nocodo_praxis::auth::RoleId;
//! use nocodo_praxis::primitives::AtLeastOne;
//! use nocodo_praxis::provenance::Provenance;
//! use nocodo_praxis::statemachine::{
//!     State, StateId, Transition, TransitionCondition, Transitions,
//! };
//!
//! static PROV: &[Provenance] = &[Provenance::Conversation {
//!     id: "prd-1",
//!     excerpt: "Tasks go from todo to in_progress to done.",
//! }];
//!
//! // A terminal state — no outgoing transitions
//! let done = State {
//!     id: StateId("done"),
//!     description: "Task completed.",
//!     transitions: Transitions::Terminal,
//!     provenance: PROV,
//! };
//! assert!(done.is_terminal());
//!
//! // A state with outgoing transitions (using static for tail to satisfy 'static lifetime)
//! static TODO_TAIL: [Transition; 1] = [Transition {
//!     to: StateId("cancelled"),
//!     permitted_roles: AtLeastOne { head: RoleId("admin"), tail: &[] },
//!     condition: TransitionCondition::Unresolved(
//!         "PRD does not specify whether members can cancel their own tasks",
//!     ),
//!     provenance: PROV,
//! }];
//!
//! let todo = State {
//!     id: StateId("todo"),
//!     description: "Task created, not yet started",
//!     transitions: Transitions::To(AtLeastOne {
//!         head: Transition {
//!             to: StateId("in_progress"),
//!             permitted_roles: AtLeastOne { head: RoleId("member"), tail: &[] },
//!             condition: TransitionCondition::OnlyIfAssignedToSelf,
//!             provenance: PROV,
//!         },
//!         tail: &TODO_TAIL,
//!     }),
//!     provenance: PROV,
//! };
//! assert!(!todo.is_terminal());
//!
//! // Detect unresolved transitions
//! let states = [todo];
//! let unresolved = nocodo_praxis::statemachine::find_unresolved_transitions(&states);
//! assert_eq!(unresolved.len(), 1);
//! ```

use super::auth::{PermissionId, RoleId};
use super::primitives::AtLeastOne;
use super::provenance::Provenance;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// A unique identifier for a state (e.g. `StateId("in_progress")`).
pub struct StateId(pub &'static str);

#[derive(Debug, Clone)]
/// Under what circumstances may a transition occur?
///
/// Conditions are coupled to permissions — the state machine and the permission
/// system are not independent. A transition requires both the correct role
/// (via `permitted_roles` on [`Transition`]) and the correct condition.
pub enum TransitionCondition {
    /// The transition is always permitted for any valid role.
    Always,
    /// Only permitted when the acting user is assigned to the entity.
    OnlyIfAssignedToSelf,
    /// Only permitted when the acting user holds a specific role.
    OnlyIfAssignedTo(RoleId),
    /// The acting user must hold a specific permission.
    RequiresPermission(PermissionId),
    /// **All** of the listed conditions must hold.
    All(&'static [TransitionCondition]),
    /// **Any** of the listed conditions must hold.
    Any(&'static [TransitionCondition]),
    /// The PRD did not specify. Blocks codegen. Triggers the clarification loop.
    /// The string describes what is missing.
    Unresolved(&'static str),
}

#[derive(Debug, Clone)]
/// A single state transition: who may trigger it, to which state, and under
/// what condition.
pub struct Transition {
    /// The target state after this transition.
    pub to: StateId,
    /// Who may trigger this transition. Structurally requires at least one role
    /// (enforced by [`AtLeastOne`]).
    pub permitted_roles: AtLeastOne<RoleId>,
    /// Under what circumstance the transition is allowed.
    pub condition: TransitionCondition,
    /// Where this transition was defined in the PRD.
    pub provenance: &'static [Provenance],
}

#[derive(Debug, Clone)]
/// Outgoing transitions from a state.
///
/// - `Transitions::Terminal` states have no outgoing transitions — they are final.
///   This is structurally distinct from "we forgot to add transitions."
/// - `Transitions::To` states have at least one outgoing transition (enforced by
///   [`AtLeastOne`]).
pub enum Transitions {
    /// No exit from this state. Structurally enforced.
    Terminal,
    /// At least one outgoing transition.
    To(AtLeastOne<Transition>),
}

#[derive(Debug, Clone)]
/// A named state in an entity's lifecycle.
pub struct State {
    /// Unique identifier (e.g. `StateId("in_progress")`).
    pub id: StateId,
    /// Human-readable description of what this state means.
    pub description: &'static str,
    /// What states can be reached from here, if any.
    pub transitions: Transitions,
    /// Where this state was defined in the PRD.
    pub provenance: &'static [Provenance],
}

impl State {
    /// Returns `true` if this state has no outgoing transitions.
    pub fn is_terminal(&self) -> bool {
        matches!(self.transitions, Transitions::Terminal)
    }
}

/// Find a state by its [`StateId`] in a list of states.
pub fn find_state<'a>(id: &StateId, states: &'a [State]) -> Option<&'a State> {
    states.iter().find(|s| &s.id == id)
}

/// Returns `true` if any state in the list is terminal.
///
/// Every entity should have at least one terminal state (a soundness invariant).
/// Use this to verify that invariant.
pub fn has_terminal_state(states: &[State]) -> bool {
    states.iter().any(|s| s.is_terminal())
}

/// Collects all transitions with [`TransitionCondition::Unresolved`] conditions.
///
/// Returns `(state, transition)` pairs for each unresolved transition.
/// Use this to surface gaps that must feed back into the clarification loop.
pub fn find_unresolved_transitions(states: &[State]) -> Vec<(&State, &Transition)> {
    let mut unresolved = Vec::new();
    for state in states {
        if let Transitions::To(list) = &state.transitions {
            for transition in list.all() {
                if matches!(transition.condition, TransitionCondition::Unresolved(_)) {
                    unresolved.push((state, transition));
                }
            }
        }
    }
    unresolved
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::RoleId;
    use crate::provenance::Provenance;

    static PROV: &[Provenance] = &[];

    fn state_id(id: &'static str) -> StateId {
        StateId(id)
    }

    fn role_id(id: &'static str) -> RoleId {
        RoleId(id)
    }

    #[test]
    fn terminal_state_detection() {
        let done = State {
            id: state_id("done"),
            description: "",
            transitions: Transitions::Terminal,
            provenance: PROV,
        };
        assert!(done.is_terminal());

        let todo = State {
            id: state_id("todo"),
            description: "",
            transitions: Transitions::To(AtLeastOne {
                head: Transition {
                    to: state_id("in_progress"),
                    permitted_roles: AtLeastOne {
                        head: role_id("member"),
                        tail: &[],
                    },
                    condition: TransitionCondition::Always,
                    provenance: PROV,
                },
                tail: &[],
            }),
            provenance: PROV,
        };
        assert!(!todo.is_terminal());
    }

    #[test]
    fn has_terminal_state_detects_presence() {
        let states = [
            State {
                id: state_id("todo"),
                description: "",
                transitions: Transitions::To(AtLeastOne {
                    head: Transition {
                        to: state_id("done"),
                        permitted_roles: AtLeastOne { head: role_id("member"), tail: &[] },
                        condition: TransitionCondition::Always,
                        provenance: PROV,
                    },
                    tail: &[],
                }),
                provenance: PROV,
            },
            State {
                id: state_id("done"),
                description: "",
                transitions: Transitions::Terminal,
                provenance: PROV,
            },
        ];
        assert!(has_terminal_state(&states));
    }

    #[test]
    fn has_terminal_state_detects_absence() {
        let states = [
            State {
                id: state_id("todo"),
                description: "",
                transitions: Transitions::To(AtLeastOne {
                    head: Transition {
                        to: state_id("in_progress"),
                        permitted_roles: AtLeastOne { head: role_id("member"), tail: &[] },
                        condition: TransitionCondition::Always,
                        provenance: PROV,
                    },
                    tail: &[],
                }),
                provenance: PROV,
            },
        ];
        assert!(!has_terminal_state(&states));
    }

    #[test]
    fn find_unresolved_transitions_collects_pending() {
        let states = [
            State {
                id: state_id("in_progress"),
                description: "",
                transitions: Transitions::To(AtLeastOne {
                    head: Transition {
                        to: state_id("todo"),
                        permitted_roles: AtLeastOne { head: role_id("member"), tail: &[] },
                        condition: TransitionCondition::Unresolved("can tasks be un-started?"),
                        provenance: PROV,
                    },
                    tail: &[],
                }),
                provenance: PROV,
            },
        ];
        let unresolved = find_unresolved_transitions(&states);
        assert_eq!(unresolved.len(), 1);
    }
}
