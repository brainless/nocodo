//! `state_transition` mode — generates one `TransitionGuard` implementation
//! per call from a natural-language action description.
//!
//! See `internal-docs/MILESTONE_PRAXIS_v1.md` §6 and §10 Phase 6. Follows the
//! same single-shot pattern as `praxis_auth.rs`: build a system prompt with
//! `nocodo_praxis` type documentation + concrete examples, build a user
//! prompt carrying the action description plus already-declared spec statics
//! and project model type context, fire one LLM call, then deterministically
//! post-process (`extract_transition_metadata`) — no LLM in the
//! post-processing step.

use std::path::Path;

use crate::praxis_doc;
use crate::static_checker::{extract_id_refs, IdKind};

/// Build the system prompt for `state_transition` mode.
///
/// Includes the `nocodo_praxis` type reference for `TransitionGuard`,
/// `TransitionError`, `HasRole`, `HasState` (via `statemachine_types_reference`),
/// `RoleId`/`HasRole` (via `auth_types_reference`), and `EntityId` (via
/// `entity_types_reference`), plus the strict output contract and worked
/// examples. `workspace_root` is passed through to the doc extractor.
pub fn build_system_prompt(workspace_root: Option<&Path>) -> String {
    let statemachine_ref = praxis_doc::statemachine_types_reference(workspace_root);
    let auth_ref = praxis_doc::auth_types_reference(workspace_root);
    let entity_ref = praxis_doc::entity_types_reference(workspace_root);

    format!(
        r#"You write a single nocodo_praxis transition guard implementation for a Rust project.

## Output contract

Return ONLY valid Rust code. No imports (`use` lines are stripped and
deterministic imports are prepended). No markdown fences. No explanation
outside of Rust comments.

Output exactly:
1. One unit `struct {{Name}};` — PascalCase, named after the action (e.g. `AssignTask`).
2. One `impl TransitionGuard<Input, Output> for {{Name}}` block with:
   - `const ENTITY: EntityId = EntityId("...");`
   - `const FROM: StateId = StateId("...");`
   - `const TO: StateId = StateId("...");`
   - `fn guard(input: Input) -> Result<Output, TransitionError> {{ ... }}`

Every `RoleId("x")`, `StateId("y")`, `EntityId("z")` you reference inside the
function body should reuse the same string literal as the corresponding
`const` above where applicable — do not invent unrelated ids. If the action
description does not give you enough information to decide a role, a state,
or a business rule, do NOT guess: return
`Err(TransitionError::Unresolved {{ reason: "..." }})` describing exactly what
is missing. The static checker downstream treats every `RoleId`/`StateId`/
`EntityId` you reference as a claim that a matching spec static exists — if
one doesn't exist yet, that's fine, it becomes a gap the user gets asked
about next; do not avoid referencing an id just because you're not sure it's
already declared.

## Types you must use

{statemachine_ref}

{auth_ref}

{entity_ref}

## Example 1 — role + state guarded transition

struct AssignTask;

impl TransitionGuard<(User, Task, User), Task> for AssignTask {{
    const ENTITY: EntityId = EntityId("task");
    const FROM: StateId = StateId("todo");
    const TO: StateId = StateId("in_progress");

    fn guard((actor, task, assignee): (User, Task, User)) -> Result<Task, TransitionError> {{
        if !actor.has_role(RoleId("admin")) {{
            return Err(TransitionError::Forbidden {{
                required: RoleId("admin"),
                found: Some(actor.role()),
            }});
        }}
        if !task.is_in_state(StateId("todo")) {{
            return Err(TransitionError::InvalidState {{
                entity: EntityId("task"),
                expected: StateId("todo"),
                found: Some(task.state()),
            }});
        }}
        Ok(Task {{
            assignee: Some(assignee.id),
            status: StateId("in_progress"),
            ..task
        }})
    }}
}}

## Example 2 — missing information -> Unresolved, not a guess

struct CancelTask;

impl TransitionGuard<(User, Task), Task> for CancelTask {{
    const ENTITY: EntityId = EntityId("task");
    const FROM: StateId = StateId("todo");
    const TO: StateId = StateId("cancelled");

    fn guard((actor, task): (User, Task)) -> Result<Task, TransitionError> {{
        // The description didn't say who is allowed to cancel a task.
        let _ = (actor, &task);
        Err(TransitionError::Unresolved {{
            reason: "who is allowed to cancel a task from 'todo' was not specified",
        }})
    }}
}}

## Example 3 — permission-gated transition

struct ApproveExpense;

impl TransitionGuard<(User, Expense), Expense> for ApproveExpense {{
    const ENTITY: EntityId = EntityId("expense");
    const FROM: StateId = StateId("submitted");
    const TO: StateId = StateId("approved");

    fn guard((actor, expense): (User, Expense)) -> Result<Expense, TransitionError> {{
        if !actor.has_role(RoleId("finance_manager")) {{
            return Err(TransitionError::ForbiddenPermission {{
                required: PermissionId("approve_expense"),
            }});
        }}
        if !expense.is_in_state(StateId("submitted")) {{
            return Err(TransitionError::InvalidState {{
                entity: EntityId("expense"),
                expected: StateId("submitted"),
                found: Some(expense.state()),
            }});
        }}
        Ok(Expense {{ status: StateId("approved"), ..expense }})
    }}
}}

Now write the transition guard for the user's action description. Reuse
existing role/state/entity ids from the "Already declared" context below when
they match; introduce new ones only when the action description clearly
implies them."#
    )
}

/// Build the user prompt for one `state_transition` call.
///
/// `model_types` is the relevant project model type context (e.g. concatenated
/// Diesel model struct source under `backend/src/models/`) so the LLM knows
/// field names and types it can construct/return. `existing_spec` is the
/// concatenated already-declared `nocodo_praxis` spec statics
/// (`backend/src/praxis/specs/*.rs`) so the LLM can reuse ids instead of
/// inventing new ones. Both may be empty strings early in a project's loop —
/// the model is expected to fall back to `TransitionError::Unresolved` or
/// reasonable guesses about field names when context is thin.
pub fn build_user_prompt(action_description: &str, model_types: &str, existing_spec: &str) -> String {
    let mut out = String::new();
    out.push_str("## Action to implement\n\n");
    out.push_str(action_description.trim());
    out.push_str("\n\n");

    out.push_str("## Project model types (for field names/types; may be empty)\n\n");
    if model_types.trim().is_empty() {
        out.push_str("(none generated yet)\n\n");
    } else {
        out.push_str(model_types.trim());
        out.push_str("\n\n");
    }

    out.push_str("## Already declared nocodo_praxis spec statics (reuse these ids where they match; may be empty)\n\n");
    if existing_spec.trim().is_empty() {
        out.push_str("(none recorded yet)\n\n");
    } else {
        out.push_str(existing_spec.trim());
        out.push('\n');
    }

    out
}

// ---------------------------------------------------------------------------
// Post-processing (deterministic, no LLM)
// ---------------------------------------------------------------------------

/// Deterministic metadata extracted from a generated `state_transition` code
/// block — MILESTONE_PRAXIS_v1.md §6.3 point 3/7. Used both to name the
/// written file and to feed the static checker's C3 (guard coverage) summary.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct TransitionMetadata {
    /// The guard struct's name, e.g. `"AssignTask"`, parsed from
    /// `impl TransitionGuard<...> for {Name}`.
    pub struct_name: Option<String>,
    /// `EntityId("...")` from the guard's `const ENTITY`.
    pub entity: Option<String>,
    /// `StateId("...")` from the guard's `const FROM`.
    pub from_state: Option<String>,
    /// `StateId("...")` from the guard's `const TO`.
    pub to_state: Option<String>,
    /// Every `RoleId("...")` literal referenced anywhere in the code.
    pub referenced_roles: Vec<String>,
    /// Every `StateId("...")` literal referenced anywhere in the code
    /// (includes `FROM`/`TO` themselves).
    pub referenced_states: Vec<String>,
    /// Every `EntityId("...")` literal referenced anywhere in the code
    /// (includes `ENTITY` itself).
    pub referenced_entities: Vec<String>,
    /// `true` if the generated code contains a
    /// `TransitionError::Unresolved { reason: ... }` return — the guard
    /// deliberately declined to implement (part of) the transition because
    /// the action description didn't say enough.
    pub has_unresolved: bool,
}

/// Parse `code` (the extracted, think-stripped, fence-unwrapped LLM output
/// for one `state_transition` call) into a [`TransitionMetadata`].
///
/// Regex-based, mirroring `agents/src/static_checker/mod.rs`'s approach —
/// tractable because this mode always emits one exact shape (one struct, one
/// `impl TransitionGuard` block). Missing pieces (e.g. no `impl
/// TransitionGuard` found at all) simply leave the corresponding field
/// `None`/empty rather than erroring; the caller (`run_state_transition`)
/// decides what to do with an incomplete result.
pub fn extract_transition_metadata(code: &str) -> TransitionMetadata {
    use regex::Regex;

    let struct_name = Regex::new(r"impl\s+TransitionGuard\s*<[^{]*>\s*for\s+(\w+)")
        .expect("valid regex")
        .captures(code)
        .map(|c| c[1].to_string());

    let entity = Regex::new(r#"const ENTITY:\s*EntityId\s*=\s*EntityId\(\s*"((?:[^"\\]|\\.)*)"\s*\);"#)
        .expect("valid regex")
        .captures(code)
        .map(|c| c[1].to_string());

    let from_state = Regex::new(r#"const FROM:\s*StateId\s*=\s*StateId\(\s*"((?:[^"\\]|\\.)*)"\s*\);"#)
        .expect("valid regex")
        .captures(code)
        .map(|c| c[1].to_string());

    let to_state = Regex::new(r#"const TO:\s*StateId\s*=\s*StateId\(\s*"((?:[^"\\]|\\.)*)"\s*\);"#)
        .expect("valid regex")
        .captures(code)
        .map(|c| c[1].to_string());

    let refs = extract_id_refs(code);
    let mut referenced_roles: Vec<String> = Vec::new();
    let mut referenced_states: Vec<String> = Vec::new();
    let mut referenced_entities: Vec<String> = Vec::new();
    for r in refs.all() {
        let (bucket, id) = match r.kind {
            IdKind::Role => (&mut referenced_roles, &r.id),
            IdKind::State => (&mut referenced_states, &r.id),
            IdKind::Entity => (&mut referenced_entities, &r.id),
            // Permission/Persona ids can appear inside a guard body too
            // (e.g. `TransitionError::ForbiddenPermission { required:
            // PermissionId("...") }`) but state_transition's summary
            // (MILESTONE_PRAXIS_v1.md §6.3 point 7) only tracks roles,
            // states, and entities — the C3 check cares about the
            // ENTITY/FROM/TO triple, not permissions.
            _ => continue,
        };
        if !bucket.contains(id) {
            bucket.push(id.clone());
        }
    }

    let has_unresolved = code.contains("TransitionError::Unresolved");

    TransitionMetadata {
        struct_name,
        entity,
        from_state,
        to_state,
        referenced_roles,
        referenced_states,
        referenced_entities,
        has_unresolved,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_GUARD: &str = r#"
struct AssignTask;

impl TransitionGuard<(User, Task, User), Task> for AssignTask {
    const ENTITY: EntityId = EntityId("task");
    const FROM: StateId = StateId("todo");
    const TO: StateId = StateId("in_progress");

    fn guard((actor, task, assignee): (User, Task, User)) -> Result<Task, TransitionError> {
        if !actor.has_role(RoleId("admin")) {
            return Err(TransitionError::Forbidden {
                required: RoleId("admin"),
                found: Some(actor.role()),
            });
        }
        if !task.is_in_state(StateId("todo")) {
            return Err(TransitionError::InvalidState {
                entity: EntityId("task"),
                expected: StateId("todo"),
                found: Some(task.state()),
            });
        }
        Ok(Task { assignee: Some(assignee.id), status: StateId("in_progress"), ..task })
    }
}
"#;

    #[test]
    fn build_system_prompt_contains_output_contract_and_types() {
        let prompt = build_system_prompt(None);
        assert!(prompt.contains("TransitionGuard"));
        assert!(prompt.contains("TransitionError"));
        assert!(prompt.contains("HasRole"));
        assert!(prompt.contains("HasState"));
        assert!(prompt.contains("EntityId"));
        assert!(prompt.contains("No markdown fences"));
        assert!(prompt.contains("Unresolved"));
    }

    #[test]
    fn build_user_prompt_includes_action_and_context() {
        let prompt = build_user_prompt("Admins can assign a task to a member", "struct Task { .. }", "pub const ROLE_ADMIN: RoleId = RoleId(\"admin\");");
        assert!(prompt.contains("Admins can assign a task to a member"));
        assert!(prompt.contains("struct Task"));
        assert!(prompt.contains("ROLE_ADMIN"));
    }

    #[test]
    fn build_user_prompt_handles_empty_context() {
        let prompt = build_user_prompt("Admins can cancel a task", "", "");
        assert!(prompt.contains("(none generated yet)"));
        assert!(prompt.contains("(none recorded yet)"));
    }

    #[test]
    fn extract_transition_metadata_parses_full_guard() {
        let meta = extract_transition_metadata(SAMPLE_GUARD);
        assert_eq!(meta.struct_name.as_deref(), Some("AssignTask"));
        assert_eq!(meta.entity.as_deref(), Some("task"));
        assert_eq!(meta.from_state.as_deref(), Some("todo"));
        assert_eq!(meta.to_state.as_deref(), Some("in_progress"));
        assert!(meta.referenced_roles.contains(&"admin".to_string()));
        assert!(meta.referenced_states.contains(&"todo".to_string()));
        assert!(meta.referenced_states.contains(&"in_progress".to_string()));
        assert!(meta.referenced_entities.contains(&"task".to_string()));
        assert!(!meta.has_unresolved);
    }

    #[test]
    fn extract_transition_metadata_detects_unresolved() {
        let code = r#"
struct CancelTask;
impl TransitionGuard<(User, Task), Task> for CancelTask {
    const ENTITY: EntityId = EntityId("task");
    const FROM: StateId = StateId("todo");
    const TO: StateId = StateId("cancelled");
    fn guard((actor, task): (User, Task)) -> Result<Task, TransitionError> {
        let _ = (actor, task);
        Err(TransitionError::Unresolved { reason: "who can cancel is not specified" })
    }
}
"#;
        let meta = extract_transition_metadata(code);
        assert!(meta.has_unresolved);
        assert_eq!(meta.struct_name.as_deref(), Some("CancelTask"));
    }

    #[test]
    fn extract_transition_metadata_empty_on_no_guard() {
        let meta = extract_transition_metadata("fn main() {}");
        assert_eq!(meta, TransitionMetadata::default());
    }

    #[test]
    fn extract_transition_metadata_dedupes_repeated_role_refs() {
        let code = r#"RoleId("admin") RoleId("admin") RoleId("member")"#;
        let meta = extract_transition_metadata(code);
        assert_eq!(meta.referenced_roles.len(), 2);
    }
}
