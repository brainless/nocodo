//! Deterministic `RoleFact` → `nocodo_praxis` Rust source serialization.
//!
//! See `internal-docs/MILESTONE_PRAXIS_v1.md` §5.5 (Type 3: Role). No LLM
//! call, just string formatting — mirrors `spec_fact::persona`.
//!
//! Per the milestone doc's explicit note in §5.5: this serializer does **not**
//! validate that referenced `PermissionId` / `PersonaId` values exist in the
//! spec — that's the static checker's job (Phase 5, C2: referential
//! integrity). It keeps the serializer simple and the checker load-bearing.
//!
//! Shape note: like `UserPersona::provenance` and `Permission::provenance`,
//! `nocodo_praxis::auth::Role::provenance` is `AtLeastOne<Provenance>`, not
//! the bare `&[CONV_SESSION_42]` slice shown in the milestone doc's §5.5
//! illustrative snippet. `permissions` and `personas` *are* bare slices
//! (`&'static [PermissionId]` / `&'static [PersonaId]`) — the doc is correct
//! there.

use serde::Deserialize;

use super::const_case;
use crate::error::AgentError;

/// PO's structured input for `record_spec_fact(fact_type: "role", ...)`.
/// See MILESTONE_PRAXIS_v1.md §5.5.
#[derive(Debug, Clone, Deserialize)]
pub struct RoleFact {
    /// Short identifier, e.g. `"admin"`.
    pub id: String,
    /// Human-readable description.
    pub description: String,
    /// Permission ids this role owns directly (not validated against
    /// declared permissions — see module docs).
    #[serde(default)]
    pub permission_ids: Vec<String>,
    /// Persona ids that typically hold this role (not validated against
    /// declared personas — see module docs).
    #[serde(default)]
    pub persona_ids: Vec<String>,
    /// `"flat"`, `"inherits"`, or `"union"`. Selects the `RoleSemantics`
    /// variant.
    pub semantics: String,
    /// Required when `semantics == "inherits"`: the parent role's id.
    #[serde(default)]
    pub parent_role_id: Option<String>,
    /// Required (non-empty) when `semantics == "union"`: the role ids whose
    /// permissions are combined. Not part of the milestone doc's illustrative
    /// JSON (which only demonstrates `"inherits"`), but `RoleSemantics::Union`
    /// needs a role list from somewhere — PO supplies it here.
    #[serde(default)]
    pub union_role_ids: Vec<String>,
}

/// Deterministic imports every generated `roles.rs` file needs.
const IMPORTS: &str = "use nocodo_praxis::auth::{RoleId, Role, RoleSemantics, PermissionId, PersonaId};\nuse nocodo_praxis::primitives::AtLeastOne;\nuse nocodo_praxis::provenance::Provenance;\n";

/// Render the `semantics: RoleSemantics::...` field value for `fact`.
///
/// Fallible: `"inherits"` requires `parent_role_id`, `"union"` requires a
/// non-empty `union_role_ids`, and any other string is an unsupported
/// semantics value. All three are user/PO data errors, not programmer
/// errors, so they're reported via `AgentError` rather than a panic.
fn render_semantics(fact: &RoleFact) -> Result<String, AgentError> {
    match fact.semantics.as_str() {
        "flat" => Ok("RoleSemantics::Flat".to_string()),
        "inherits" => match &fact.parent_role_id {
            Some(parent) => Ok(format!(
                "RoleSemantics::Inherits {{ parent: RoleId({parent:?}) }}"
            )),
            None => Err(AgentError::Other(format!(
                "record_spec_fact: role '{}' has semantics \"inherits\" but no parent_role_id",
                fact.id
            ))),
        },
        "union" => {
            if fact.union_role_ids.is_empty() {
                return Err(AgentError::Other(format!(
                    "record_spec_fact: role '{}' has semantics \"union\" but union_role_ids is empty",
                    fact.id
                )));
            }
            let of = fact
                .union_role_ids
                .iter()
                .map(|id| format!("RoleId({id:?})"))
                .collect::<Vec<_>>()
                .join(", ");
            Ok(format!("RoleSemantics::Union {{ of: &[{of}] }}"))
        }
        other => Err(AgentError::Other(format!(
            "record_spec_fact: role '{}' has unsupported semantics \"{other}\" \
             (expected \"flat\", \"inherits\", or \"union\")",
            fact.id
        ))),
    }
}

fn id_slice_literal(prefix: &str, ids: &[String]) -> String {
    let inner = ids
        .iter()
        .map(|id| format!("{prefix}({id:?})"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("&[{inner}]")
}

/// Emit the `const CONV_{SESSION}` / `pub const ROLE_{ID}` / `pub static
/// ROLE_{ID}_DEF` block for one role — everything except the leading `use`
/// imports.
pub fn role_body_rust(fact: &RoleFact, session_id: &str) -> Result<String, AgentError> {
    let semantics = render_semantics(fact)?;

    // See persona.rs's `persona_body_rust` for why this is suffixed with the
    // fact's own id (Praxis v1 Phase 7 bug fix — avoids duplicate `const
    // CONV_{session}` when two facts share a session).
    let conv_const = format!("CONV_{}_{}", const_case(session_id), const_case(&fact.id));
    let role_const = format!("ROLE_{}", const_case(&fact.id));
    let role_static = format!("{role_const}_DEF");

    let mut out = String::new();
    out.push_str(&format!("const {conv_const}: Provenance = Provenance::Conversation {{\n"));
    out.push_str(&format!("    id: {session_id:?},\n"));
    out.push_str(&format!("    excerpt: {:?},\n", fact.description));
    out.push_str("};\n\n");

    out.push_str(&format!(
        "pub const {role_const}: RoleId = RoleId({:?});\n\n",
        fact.id
    ));

    out.push_str(&format!("pub static {role_static}: Role = Role {{\n"));
    out.push_str(&format!("    id: {role_const},\n"));
    out.push_str(&format!("    description: {:?},\n", fact.description));
    out.push_str(&format!("    semantics: {semantics},\n"));
    out.push_str(&format!(
        "    permissions: {},\n",
        id_slice_literal("PermissionId", &fact.permission_ids)
    ));
    out.push_str(&format!(
        "    personas: {},\n",
        id_slice_literal("PersonaId", &fact.persona_ids)
    ));
    out.push_str(&format!(
        "    provenance: AtLeastOne {{ head: {conv_const}, tail: &[] }},\n"
    ));
    out.push_str("};\n");

    Ok(out)
}

/// Serialize a single `RoleFact` into a complete, standalone Rust source
/// file: deterministic imports followed by the role's `const`/`static` block.
pub fn role_to_rust(fact: &RoleFact, session_id: &str) -> Result<String, AgentError> {
    Ok(format!("{IMPORTS}\n{}", role_body_rust(fact, session_id)?))
}

/// Merge a new role's body block into an existing `roles.rs` file's contents
/// (or start a fresh file with imports if `existing` is `None`). Idempotent:
/// re-recording the same role id is a no-op.
pub fn merge_role_file(
    existing: Option<&str>,
    fact: &RoleFact,
    session_id: &str,
) -> Result<String, AgentError> {
    let role_const_decl = format!("pub const ROLE_{}:", const_case(&fact.id));

    match existing {
        Some(current) if current.contains(&role_const_decl) => Ok(current.to_string()),
        Some(current) => {
            let body = role_body_rust(fact, session_id)?;
            Ok(format!("{}\n{}", current.trim_end(), body))
        }
        None => role_to_rust(fact, session_id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec_fact::compile_check::assert_compiles_against_nocodo_praxis;

    fn inherits_fact() -> RoleFact {
        RoleFact {
            id: "admin".into(),
            description: "Team admin".into(),
            permission_ids: vec!["create_task".into(), "assign_task".into()],
            persona_ids: vec!["admin".into()],
            semantics: "inherits".into(),
            parent_role_id: Some("member".into()),
            union_role_ids: vec![],
        }
    }

    #[test]
    fn role_to_rust_inherits_contains_expected_identifiers() {
        let fact = inherits_fact();
        let code = role_to_rust(&fact, "session-42").unwrap();

        assert!(code.contains("use nocodo_praxis::auth::{RoleId, Role, RoleSemantics, PermissionId, PersonaId};"));
        assert!(code.contains("pub const ROLE_ADMIN: RoleId = RoleId(\"admin\");"));
        assert!(code.contains("pub static ROLE_ADMIN_DEF: Role = Role {"));
        assert!(code.contains("semantics: RoleSemantics::Inherits { parent: RoleId(\"member\") },"));
        assert!(code.contains(r#"permissions: &[PermissionId("create_task"), PermissionId("assign_task")],"#));
        assert!(code.contains(r#"personas: &[PersonaId("admin")],"#));
        assert!(code.contains("provenance: AtLeastOne { head: CONV_SESSION_42_ADMIN, tail: &[] },"));
        assert_eq!(code.matches('{').count(), code.matches('}').count());
    }

    #[test]
    fn role_to_rust_inherits_compiles_against_nocodo_praxis() {
        let fact = inherits_fact();
        let code = role_to_rust(&fact, "session-42").unwrap();
        assert_compiles_against_nocodo_praxis(&code);
    }

    #[test]
    fn role_to_rust_flat_compiles_against_nocodo_praxis() {
        let fact = RoleFact {
            id: "all_authenticated".into(),
            description: "Any user with a valid session".into(),
            permission_ids: vec!["view_all_tasks".into()],
            persona_ids: vec![],
            semantics: "flat".into(),
            parent_role_id: None,
            union_role_ids: vec![],
        };
        let code = role_to_rust(&fact, "session-1").unwrap();
        assert!(code.contains("semantics: RoleSemantics::Flat,"));
        assert!(code.contains("personas: &[],"));
        assert_compiles_against_nocodo_praxis(&code);
    }

    #[test]
    fn role_to_rust_union_compiles_against_nocodo_praxis() {
        let fact = RoleFact {
            id: "moderator".into(),
            description: "Combines member and reviewer privileges".into(),
            permission_ids: vec![],
            persona_ids: vec![],
            semantics: "union".into(),
            parent_role_id: None,
            union_role_ids: vec!["member".into(), "reviewer".into()],
        };
        let code = role_to_rust(&fact, "session-1").unwrap();
        assert!(code.contains(r#"semantics: RoleSemantics::Union { of: &[RoleId("member"), RoleId("reviewer")] },"#));
        assert_compiles_against_nocodo_praxis(&code);
    }

    #[test]
    fn role_to_rust_inherits_without_parent_errors() {
        let fact = RoleFact {
            id: "admin".into(),
            description: "Team admin".into(),
            permission_ids: vec![],
            persona_ids: vec![],
            semantics: "inherits".into(),
            parent_role_id: None,
            union_role_ids: vec![],
        };
        let err = role_to_rust(&fact, "session-1").unwrap_err();
        assert!(err.to_string().contains("no parent_role_id"));
    }

    #[test]
    fn role_to_rust_union_without_role_ids_errors() {
        let fact = RoleFact {
            id: "moderator".into(),
            description: "desc".into(),
            permission_ids: vec![],
            persona_ids: vec![],
            semantics: "union".into(),
            parent_role_id: None,
            union_role_ids: vec![],
        };
        let err = role_to_rust(&fact, "session-1").unwrap_err();
        assert!(err.to_string().contains("union_role_ids is empty"));
    }

    #[test]
    fn role_to_rust_unknown_semantics_errors() {
        let fact = RoleFact {
            id: "admin".into(),
            description: "desc".into(),
            permission_ids: vec![],
            persona_ids: vec![],
            semantics: "bogus".into(),
            parent_role_id: None,
            union_role_ids: vec![],
        };
        let err = role_to_rust(&fact, "session-1").unwrap_err();
        assert!(err.to_string().contains("unsupported semantics"));
    }

    #[test]
    fn merge_role_file_appends_second_role() {
        let admin = inherits_fact();
        let member = RoleFact {
            id: "member".into(),
            description: "Registered team member".into(),
            permission_ids: vec!["create_task".into()],
            persona_ids: vec!["member".into()],
            semantics: "flat".into(),
            parent_role_id: None,
            union_role_ids: vec![],
        };

        let file_one = merge_role_file(None, &admin, "session-42").unwrap();
        let file_two = merge_role_file(Some(&file_one), &member, "session-43").unwrap();

        assert!(file_two.contains("pub const ROLE_ADMIN: RoleId"));
        assert!(file_two.contains("pub const ROLE_MEMBER: RoleId"));
        assert_eq!(
            file_two
                .matches("use nocodo_praxis::auth::{RoleId, Role, RoleSemantics, PermissionId, PersonaId};")
                .count(),
            1
        );
        assert_compiles_against_nocodo_praxis(&file_two);
    }

    #[test]
    fn merge_role_file_is_idempotent_for_same_role() {
        let admin = inherits_fact();
        let file_one = merge_role_file(None, &admin, "session-42").unwrap();
        let file_two = merge_role_file(Some(&file_one), &admin, "session-42").unwrap();
        assert_eq!(file_one, file_two);
    }
}
