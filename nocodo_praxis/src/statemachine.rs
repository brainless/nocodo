use super::auth::{PermissionId, RoleId};
use super::primitives::AtLeastOne;
use super::provenance::Provenance;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StateId(pub &'static str);

#[derive(Debug, Clone)]
pub enum TransitionCondition {
    Always,
    OnlyIfAssignedToSelf,
    OnlyIfAssignedTo(RoleId),
    RequiresPermission(PermissionId),
    All(&'static [TransitionCondition]),
    Any(&'static [TransitionCondition]),
    Unresolved(&'static str),
}

#[derive(Debug, Clone)]
pub struct Transition {
    pub to: StateId,
    pub permitted_roles: AtLeastOne<RoleId>,
    pub condition: TransitionCondition,
    pub provenance: &'static [Provenance],
}

#[derive(Debug, Clone)]
pub enum Transitions {
    Terminal,
    To(AtLeastOne<Transition>),
}

#[derive(Debug, Clone)]
pub struct State {
    pub id: StateId,
    pub description: &'static str,
    pub transitions: Transitions,
    pub provenance: &'static [Provenance],
}

impl State {
    pub fn is_terminal(&self) -> bool {
        matches!(self.transitions, Transitions::Terminal)
    }
}

pub fn find_state<'a>(id: &StateId, states: &'a [State]) -> Option<&'a State> {
    states.iter().find(|s| &s.id == id)
}

pub fn has_terminal_state(states: &[State]) -> bool {
    states.iter().any(|s| s.is_terminal())
}

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
