//! Deterministic `PermissionFact` → `nocodo_praxis` Rust source serialization.
//!
//! See `internal-docs/MILESTONE_PRAXIS_v1.md` §5.4 (Type 2: Permission). No LLM
//! call, just string formatting — mirrors `spec_fact::persona`.
//!
//! Note: the milestone doc's §5.4 illustrative example shows
//! `provenance: &[CONV_SESSION_42]` (a bare slice) on `Permission`, but
//! `nocodo_praxis::auth::Permission::provenance` is actually typed
//! `AtLeastOne<Provenance>` (see `nocodo_praxis/src/auth.rs`) — the same
//! deviation Phase 3 found for `UserPersona::provenance`. We serialize against
//! the real type.

use serde::Deserialize;

/// PO's structured input for `record_spec_fact(fact_type: "permission", ...)`.
/// See MILESTONE_PRAXIS_v1.md §5.4.
#[derive(Debug, Clone, Deserialize)]
pub struct PermissionFact {
    /// Short identifier, e.g. `"create_task"`.
    pub id: String,
    /// Human-readable description of what this permission grants.
    pub description: String,
}

/// Deterministic imports every generated `permissions.rs` file needs.
const IMPORTS: &str = "use nocodo_praxis::auth::{PermissionId, Permission};\nuse nocodo_praxis::primitives::AtLeastOne;\nuse nocodo_praxis::provenance::Provenance;\n";

use super::const_case;

/// Emit the `const CONV_{SESSION}` / `pub const PERM_{ID}` / `pub static
/// PERM_{ID}_DEF` block for one permission — everything except the leading
/// `use` imports.
pub fn permission_body_rust(fact: &PermissionFact, session_id: &str) -> String {
    // See persona.rs's `persona_body_rust` for why this is suffixed with the
    // fact's own id (Praxis v1 Phase 7 bug fix — avoids duplicate `const
    // CONV_{session}` when two facts share a session).
    let conv_const = format!("CONV_{}_{}", const_case(session_id), const_case(&fact.id));
    let perm_const = format!("PERM_{}", const_case(&fact.id));
    let perm_static = format!("{perm_const}_DEF");

    let mut out = String::new();
    out.push_str(&format!("const {conv_const}: Provenance = Provenance::Conversation {{\n"));
    out.push_str(&format!("    id: {session_id:?},\n"));
    out.push_str(&format!("    excerpt: {:?},\n", fact.description));
    out.push_str("};\n\n");

    out.push_str(&format!(
        "pub const {perm_const}: PermissionId = PermissionId({:?});\n\n",
        fact.id
    ));

    out.push_str(&format!("pub static {perm_static}: Permission = Permission {{\n"));
    out.push_str(&format!("    id: {perm_const},\n"));
    out.push_str(&format!("    description: {:?},\n", fact.description));
    out.push_str(&format!(
        "    provenance: AtLeastOne {{ head: {conv_const}, tail: &[] }},\n"
    ));
    out.push_str("};\n");

    out
}

/// Serialize a single `PermissionFact` into a complete, standalone Rust
/// source file: deterministic imports followed by the permission's
/// `const`/`static` block.
pub fn permission_to_rust(fact: &PermissionFact, session_id: &str) -> String {
    format!("{IMPORTS}\n{}", permission_body_rust(fact, session_id))
}

/// Merge a new permission's body block into an existing `permissions.rs`
/// file's contents (or start a fresh file with imports if `existing` is
/// `None`). Idempotent: re-recording the same permission id is a no-op.
pub fn merge_permission_file(
    existing: Option<&str>,
    fact: &PermissionFact,
    session_id: &str,
) -> String {
    let perm_const_decl = format!("pub const PERM_{}:", const_case(&fact.id));
    let body = permission_body_rust(fact, session_id);

    match existing {
        Some(current) if current.contains(&perm_const_decl) => current.to_string(),
        Some(current) => format!("{}\n{}", current.trim_end(), body),
        None => permission_to_rust(fact, session_id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec_fact::compile_check::assert_compiles_against_nocodo_praxis;

    fn create_task_fact() -> PermissionFact {
        PermissionFact {
            id: "create_task".into(),
            description: "Create a new task".into(),
        }
    }

    #[test]
    fn permission_to_rust_contains_expected_identifiers() {
        let fact = create_task_fact();
        let code = permission_to_rust(&fact, "session-42");

        assert!(code.contains("use nocodo_praxis::auth::{PermissionId, Permission};"));
        assert!(code.contains("const CONV_SESSION_42_CREATE_TASK: Provenance = Provenance::Conversation {"));
        assert!(code.contains("pub const PERM_CREATE_TASK: PermissionId = PermissionId(\"create_task\");"));
        assert!(code.contains("pub static PERM_CREATE_TASK_DEF: Permission = Permission {"));
        assert!(code.contains("id: PERM_CREATE_TASK,"));
        assert!(code.contains("description: \"Create a new task\","));
        assert!(code.contains("provenance: AtLeastOne { head: CONV_SESSION_42_CREATE_TASK, tail: &[] },"));
        assert_eq!(code.matches('{').count(), code.matches('}').count());
    }

    #[test]
    fn permission_to_rust_compiles_against_nocodo_praxis() {
        let fact = create_task_fact();
        let code = permission_to_rust(&fact, "session-42");
        assert_compiles_against_nocodo_praxis(&code);
    }

    #[test]
    fn merge_permission_file_appends_second_permission() {
        let create = create_task_fact();
        let assign = PermissionFact {
            id: "assign_task".into(),
            description: "Assign a task to a team member".into(),
        };

        let file_one = merge_permission_file(None, &create, "session-42");
        let file_two = merge_permission_file(Some(&file_one), &assign, "session-43");

        assert!(file_two.contains("pub const PERM_CREATE_TASK: PermissionId"));
        assert!(file_two.contains("pub const PERM_ASSIGN_TASK: PermissionId"));
        assert_eq!(
            file_two
                .matches("use nocodo_praxis::auth::{PermissionId, Permission};")
                .count(),
            1
        );
        assert_compiles_against_nocodo_praxis(&file_two);
    }

    #[test]
    fn merge_permission_file_is_idempotent_for_same_permission() {
        let create = create_task_fact();
        let file_one = merge_permission_file(None, &create, "session-42");
        let file_two = merge_permission_file(Some(&file_one), &create, "session-42");
        assert_eq!(file_one, file_two);
    }
}
