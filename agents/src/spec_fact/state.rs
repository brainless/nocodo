//! Deterministic `StateFact` → `nocodo_praxis` Rust source serialization.
//!
//! See `internal-docs/MILESTONE_PRAXIS_v1.md` §5.6 (Type 4: State +
//! Transition). No LLM call, just string formatting — mirrors
//! `spec_fact::persona`.
//!
//! Shape check against `nocodo_praxis::statemachine` (unlike `UserPersona`,
//! `Permission`, and `Role`, this type's fields mostly match the milestone
//! doc's illustrative snippet as-is):
//! - `State::provenance` and `Transition::provenance` are bare
//!   `&'static [Provenance]` slices — the doc's `&[CONV_SESSION_42]` is
//!   correct here, no `AtLeastOne` wrapping needed.
//! - `State::transitions` is `Transitions::To(AtLeastOne<Transition>)` for a
//!   non-terminal state, matching the doc.
//! - `Transition::permitted_roles` is `AtLeastOne<RoleId>`, matching the doc.
//!
//! Condition mapping: unknown/unspecified condition strings produce
//! `TransitionCondition::Unresolved("condition not specified")`, per the
//! milestone doc's explicit instruction in §5.6.

use serde::Deserialize;

use super::const_case;
use crate::error::AgentError;

/// PO's structured input for one outgoing transition. See
/// MILESTONE_PRAXIS_v1.md §5.6.
#[derive(Debug, Clone, Deserialize)]
pub struct TransitionFact {
    /// Target state id, e.g. `"in_progress"`.
    pub to: String,
    /// Roles permitted to trigger this transition. Must be non-empty —
    /// `Transition::permitted_roles` is structurally `AtLeastOne<RoleId>`.
    #[serde(default)]
    pub role_ids: Vec<String>,
    /// Condition name. Recognized values: `"always"`,
    /// `"only_if_assigned_to_self"`, `"only_if_assigned_to:{role_id}"`,
    /// `"requires_permission:{permission_id}"`. Anything else (including
    /// empty/omitted) maps to `TransitionCondition::Unresolved("condition
    /// not specified")`.
    #[serde(default)]
    pub condition: String,
}

/// PO's structured input for `record_spec_fact(fact_type: "state", ...)`.
#[derive(Debug, Clone, Deserialize)]
pub struct StateFact {
    /// Short identifier, e.g. `"todo"`.
    pub id: String,
    /// Human-readable description.
    pub description: String,
    /// Outgoing transitions. Empty means this is a terminal state
    /// (`Transitions::Terminal`) — structurally distinct from "we forgot to
    /// add transitions."
    #[serde(default)]
    pub transitions: Vec<TransitionFact>,
}

/// Deterministic imports every generated `states.rs` file needs.
const IMPORTS: &str = "use nocodo_praxis::auth::{RoleId, PermissionId};\nuse nocodo_praxis::primitives::AtLeastOne;\nuse nocodo_praxis::provenance::Provenance;\nuse nocodo_praxis::statemachine::{State, StateId, Transition, Transitions, TransitionCondition};\n";

/// Render a `TransitionCondition::...` expression for `condition`.
fn render_condition(condition: &str) -> String {
    if condition == "always" {
        return "TransitionCondition::Always".to_string();
    }
    if condition == "only_if_assigned_to_self" {
        return "TransitionCondition::OnlyIfAssignedToSelf".to_string();
    }
    if let Some(role) = condition.strip_prefix("only_if_assigned_to:") {
        return format!("TransitionCondition::OnlyIfAssignedTo(RoleId({role:?}))");
    }
    if let Some(perm) = condition.strip_prefix("requires_permission:") {
        return format!("TransitionCondition::RequiresPermission(PermissionId({perm:?}))");
    }
    // Unknown or unspecified — blocks codegen, feeds the clarification loop.
    "TransitionCondition::Unresolved(\"condition not specified\")".to_string()
}

/// Render one `Transition { ... }` literal (no trailing comma; callers add
/// list separators).
fn render_transition(t: &TransitionFact) -> Result<String, AgentError> {
    let Some((head_role, tail_roles)) = t.role_ids.split_first() else {
        return Err(AgentError::Other(format!(
            "record_spec_fact: transition to '{}' has no role_ids — \
             Transition::permitted_roles requires at least one role",
            t.to
        )));
    };
    let tail = tail_roles
        .iter()
        .map(|r| format!("RoleId({r:?})"))
        .collect::<Vec<_>>()
        .join(", ");

    Ok(format!(
        "Transition {{\n            to: StateId({to:?}),\n            permitted_roles: AtLeastOne {{ head: RoleId({head:?}), tail: &[{tail}] }},\n            condition: {condition},\n            provenance: &[CONV],\n        }}",
        to = t.to,
        head = head_role,
        condition = render_condition(&t.condition),
    ))
}

/// Emit the `const CONV_{SESSION}` / `pub const STATE_{ID}` / `pub static
/// STATE_{ID}_DEF` block for one state — everything except the leading `use`
/// imports.
pub fn state_body_rust(fact: &StateFact, session_id: &str) -> Result<String, AgentError> {
    // See persona.rs's `persona_body_rust` for why this is suffixed with the
    // fact's own id (Praxis v1 Phase 7 bug fix — avoids duplicate `const
    // CONV_{session}` when two facts share a session).
    let conv_const = format!("CONV_{}_{}", const_case(session_id), const_case(&fact.id));
    let state_const = format!("STATE_{}", const_case(&fact.id));
    let state_static = format!("{state_const}_DEF");

    let mut out = String::new();
    out.push_str(&format!("const {conv_const}: Provenance = Provenance::Conversation {{\n"));
    out.push_str(&format!("    id: {session_id:?},\n"));
    out.push_str(&format!("    excerpt: {:?},\n", fact.description));
    out.push_str("};\n\n");

    out.push_str(&format!(
        "pub const {state_const}: StateId = StateId({:?});\n\n",
        fact.id
    ));

    out.push_str(&format!("pub static {state_static}: State = State {{\n"));
    out.push_str(&format!("    id: {state_const},\n"));
    out.push_str(&format!("    description: {:?},\n", fact.description));

    if fact.transitions.is_empty() {
        out.push_str("    transitions: Transitions::Terminal,\n");
    } else {
        let mut rendered = Vec::with_capacity(fact.transitions.len());
        for t in &fact.transitions {
            // Render with the real provenance const name substituted in.
            let piece = render_transition(t)?.replace("&[CONV]", &format!("&[{conv_const}]"));
            rendered.push(piece);
        }
        let (head, tail) = rendered.split_first().expect("checked non-empty above");
        let tail_joined = tail
            .iter()
            .map(|t| format!("{t},"))
            .collect::<Vec<_>>()
            .join("\n            ");
        out.push_str("    transitions: Transitions::To(AtLeastOne {\n");
        out.push_str(&format!("        head: {head},\n"));
        if tail.is_empty() {
            out.push_str("        tail: &[],\n");
        } else {
            out.push_str(&format!("        tail: &[\n            {tail_joined}\n        ],\n"));
        }
        out.push_str("    }),\n");
    }

    out.push_str(&format!("    provenance: &[{conv_const}],\n"));
    out.push_str("};\n");

    Ok(out)
}

/// Serialize a single `StateFact` into a complete, standalone Rust source
/// file: deterministic imports followed by the state's `const`/`static`
/// block.
pub fn state_to_rust(fact: &StateFact, session_id: &str) -> Result<String, AgentError> {
    Ok(format!("{IMPORTS}\n{}", state_body_rust(fact, session_id)?))
}

/// Merge a new state's body block into an existing `states.rs` file's
/// contents (or start a fresh file with imports if `existing` is `None`).
/// Idempotent: re-recording the same state id is a no-op.
pub fn merge_state_file(
    existing: Option<&str>,
    fact: &StateFact,
    session_id: &str,
) -> Result<String, AgentError> {
    let state_const_decl = format!("pub const STATE_{}:", const_case(&fact.id));

    match existing {
        Some(current) if current.contains(&state_const_decl) => Ok(current.to_string()),
        Some(current) => {
            let body = state_body_rust(fact, session_id)?;
            Ok(format!("{}\n{}", current.trim_end(), body))
        }
        None => state_to_rust(fact, session_id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec_fact::compile_check::assert_compiles_against_nocodo_praxis;

    fn todo_fact() -> StateFact {
        StateFact {
            id: "todo".into(),
            description: "Task created, not yet started".into(),
            transitions: vec![
                TransitionFact {
                    to: "in_progress".into(),
                    role_ids: vec!["member".into(), "admin".into()],
                    condition: "only_if_assigned_to_self".into(),
                },
                TransitionFact {
                    to: "cancelled".into(),
                    role_ids: vec!["admin".into()],
                    condition: "always".into(),
                },
            ],
        }
    }

    #[test]
    fn state_to_rust_contains_expected_identifiers() {
        let fact = todo_fact();
        let code = state_to_rust(&fact, "session-42").unwrap();

        assert!(code.contains("pub const STATE_TODO: StateId = StateId(\"todo\");"));
        assert!(code.contains("pub static STATE_TODO_DEF: State = State {"));
        assert!(code.contains("transitions: Transitions::To(AtLeastOne {"));
        assert!(code.contains("to: StateId(\"in_progress\"),"));
        assert!(code.contains("permitted_roles: AtLeastOne { head: RoleId(\"member\"), tail: &[RoleId(\"admin\")] },"));
        assert!(code.contains("condition: TransitionCondition::OnlyIfAssignedToSelf,"));
        assert!(code.contains("to: StateId(\"cancelled\"),"));
        assert!(code.contains("condition: TransitionCondition::Always,"));
        assert!(code.contains("provenance: &[CONV_SESSION_42_TODO],"));
        assert_eq!(code.matches('{').count(), code.matches('}').count());
    }

    #[test]
    fn state_to_rust_multi_transition_compiles_against_nocodo_praxis() {
        let fact = todo_fact();
        let code = state_to_rust(&fact, "session-42").unwrap();
        assert_compiles_against_nocodo_praxis(&code);
    }

    #[test]
    fn state_to_rust_terminal_state_compiles_against_nocodo_praxis() {
        let fact = StateFact {
            id: "done".into(),
            description: "Task completed".into(),
            transitions: vec![],
        };
        let code = state_to_rust(&fact, "session-1").unwrap();
        assert!(code.contains("transitions: Transitions::Terminal,"));
        assert_compiles_against_nocodo_praxis(&code);
    }

    #[test]
    fn state_to_rust_unresolved_condition_compiles_against_nocodo_praxis() {
        let fact = StateFact {
            id: "in_progress".into(),
            description: "Task is being worked on".into(),
            transitions: vec![TransitionFact {
                to: "todo".into(),
                role_ids: vec!["member".into()],
                condition: "".into(),
            }],
        };
        let code = state_to_rust(&fact, "session-1").unwrap();
        assert!(code.contains(r#"condition: TransitionCondition::Unresolved("condition not specified"),"#));
        assert_compiles_against_nocodo_praxis(&code);
    }

    #[test]
    fn state_to_rust_transition_with_no_roles_errors() {
        let fact = StateFact {
            id: "todo".into(),
            description: "desc".into(),
            transitions: vec![TransitionFact {
                to: "done".into(),
                role_ids: vec![],
                condition: "always".into(),
            }],
        };
        let err = state_to_rust(&fact, "session-1").unwrap_err();
        assert!(err.to_string().contains("no role_ids"));
    }

    #[test]
    fn merge_state_file_appends_second_state() {
        let todo = todo_fact();
        let done = StateFact {
            id: "done".into(),
            description: "Task completed".into(),
            transitions: vec![],
        };

        let file_one = merge_state_file(None, &todo, "session-42").unwrap();
        let file_two = merge_state_file(Some(&file_one), &done, "session-43").unwrap();

        assert!(file_two.contains("pub const STATE_TODO: StateId"));
        assert!(file_two.contains("pub const STATE_DONE: StateId"));
        assert_eq!(
            file_two
                .matches("use nocodo_praxis::statemachine::{State, StateId, Transition, Transitions, TransitionCondition};")
                .count(),
            1
        );
        assert_compiles_against_nocodo_praxis(&file_two);
    }

    #[test]
    fn merge_state_file_is_idempotent_for_same_state() {
        let todo = todo_fact();
        let file_one = merge_state_file(None, &todo, "session-42").unwrap();
        let file_two = merge_state_file(Some(&file_one), &todo, "session-42").unwrap();
        assert_eq!(file_one, file_two);
    }
}
