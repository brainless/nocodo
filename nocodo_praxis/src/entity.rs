use super::primitives::Unresolved;
use super::provenance::Provenance;
use super::statemachine::State;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId(pub &'static str);

#[derive(Debug, Clone)]
pub enum Assignee<Id> {
    Unassigned,
    AssignedTo(Id),
    Unresolved(&'static str),
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: &'static str,
    pub description: &'static str,
    pub invariants: &'static [Unresolved<&'static str>],
    pub provenance: &'static [Provenance],
}

#[derive(Debug, Clone)]
pub struct Entity {
    pub id: EntityId,
    pub description: &'static str,
    pub fields: &'static [Field],
    pub states: &'static [State],
    pub invariants: &'static [Unresolved<&'static str>],
    pub provenance: &'static [Provenance],
}

impl Entity {
    pub fn has_pending_invariants(&self) -> bool {
        self.fields.iter().any(|f| f.invariants.iter().any(|i| i.blocks_codegen()))
            || self.invariants.iter().any(|i| i.blocks_codegen())
    }

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
                        head: Provenance::Conversation { id: "t", excerpt: "x" },
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
