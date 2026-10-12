//! Deterministic `EntityFact` → `nocodo_praxis` Rust source serialization.
//!
//! See `internal-docs/MILESTONE_PRAXIS_v1.md` §5.7 (Type 5: Entity). No LLM
//! call, just string formatting — mirrors `spec_fact::persona`.
//!
//! Shape check against `nocodo_praxis::entity`: `Entity::fields`,
//! `Entity::invariants`, `Entity::provenance`, and `Field::invariants` /
//! `Field::provenance` are all bare `&'static [_]` slices — the milestone
//! doc's §5.7 snippet is correct on that front, unlike the `AtLeastOne`
//! mismatches Phase 3/4 found for `UserPersona`, `Permission`, and `Role`
//! provenance.
//!
//! The one real deviation: the doc's example shows
//! `Unresolved::Pending { reason, provenance: &[Provenance::Inferred {..}] }`
//! — a bare slice for the *inner* `provenance` field. But
//! `Unresolved::Pending`'s `provenance` field is `AtLeastOne<Provenance>`
//! (see `nocodo_praxis/src/primitives.rs`), not a bare slice. We serialize
//! against the real type: `AtLeastOne { head: Provenance::Inferred {..}, tail: &[] }`.

use serde::Deserialize;

use super::const_case;

/// One field on an entity. See MILESTONE_PRAXIS_v1.md §5.7.
#[derive(Debug, Clone, Deserialize)]
pub struct FieldFact {
    /// Field name, e.g. `"title"`.
    pub name: String,
    /// Human-readable description of what this field represents.
    pub description: String,
    /// If the user stated an explicit constraint (e.g. `"max 200 chars"`),
    /// it becomes `Unresolved::Resolved(...)`. `None` (the default — the
    /// milestone doc's illustrative JSON never sets this) means no
    /// constraint was stated, so codegen emits `Unresolved::Pending { .. }`,
    /// a gap for the static checker to surface.
    #[serde(default)]
    pub invariant: Option<String>,
}

/// PO's structured input for `record_spec_fact(fact_type: "entity", ...)`.
#[derive(Debug, Clone, Deserialize)]
pub struct EntityFact {
    /// Short identifier, e.g. `"task"`.
    pub id: String,
    /// Human-readable description.
    pub description: String,
    /// The entity's data fields.
    #[serde(default)]
    pub fields: Vec<FieldFact>,
    /// Ids of the states in this entity's lifecycle (declared separately via
    /// `fact_type: "state"`; not validated here — see the static checker's
    /// C2 referential integrity check, Phase 5).
    #[serde(default)]
    pub state_ids: Vec<String>,
}

/// Deterministic imports every generated `entities.rs` file needs.
const IMPORTS: &str = "use nocodo_praxis::entity::{Entity, EntityId, Field};\nuse nocodo_praxis::primitives::{AtLeastOne, Unresolved};\nuse nocodo_praxis::provenance::Provenance;\nuse nocodo_praxis::statemachine::StateId;\n";

/// Render one `Field { ... }` literal.
fn render_field(field: &FieldFact, session_id: &str, conv_const: &str) -> String {
    let invariants = match &field.invariant {
        Some(invariant) => format!("&[Unresolved::Resolved({invariant:?})]"),
        None => format!(
            "&[Unresolved::Pending {{\n                reason: {reason:?},\n                provenance: AtLeastOne {{\n                    head: Provenance::Inferred {{\n                        reason: \"Every field needs an explicit constraint or an explicit statement that none apply\",\n                        from: &[{session_id:?}],\n                    }},\n                    tail: &[],\n                }},\n            }}]",
            reason = format!("Constraints for '{}' not specified", field.name),
        ),
    };

    format!(
        "Field {{\n            name: {name:?},\n            description: {description:?},\n            invariants: {invariants},\n            provenance: &[{conv_const}],\n        }}",
        name = field.name,
        description = field.description,
    )
}

/// Emit the `const CONV_{SESSION}` / `pub const ENTITY_{ID}` / `pub static
/// ENTITY_{ID}_DEF` block for one entity — everything except the leading
/// `use` imports.
pub fn entity_body_rust(fact: &EntityFact, session_id: &str) -> String {
    // See persona.rs's `persona_body_rust` for why this is suffixed with the
    // fact's own id (Praxis v1 Phase 7 bug fix — avoids duplicate `const
    // CONV_{session}` when two facts share a session).
    let conv_const = format!("CONV_{}_{}", const_case(session_id), const_case(&fact.id));
    let entity_const = format!("ENTITY_{}", const_case(&fact.id));
    let entity_static = format!("{entity_const}_DEF");

    let mut out = String::new();
    out.push_str(&format!("const {conv_const}: Provenance = Provenance::Conversation {{\n"));
    out.push_str(&format!("    id: {session_id:?},\n"));
    out.push_str(&format!("    excerpt: {:?},\n", fact.description));
    out.push_str("};\n\n");

    out.push_str(&format!(
        "pub const {entity_const}: EntityId = EntityId({:?});\n\n",
        fact.id
    ));

    out.push_str(&format!("pub static {entity_static}: Entity = Entity {{\n"));
    out.push_str(&format!("    id: {entity_const},\n"));
    out.push_str(&format!("    description: {:?},\n", fact.description));

    if fact.fields.is_empty() {
        out.push_str("    fields: &[],\n");
    } else {
        let rendered = fact
            .fields
            .iter()
            .map(|f| format!("{},", render_field(f, session_id, &conv_const)))
            .collect::<Vec<_>>()
            .join("\n        ");
        out.push_str(&format!("    fields: &[\n        {rendered}\n    ],\n"));
    }

    if fact.state_ids.is_empty() {
        out.push_str("    states: &[],\n");
    } else {
        let states = fact
            .state_ids
            .iter()
            .map(|s| format!("StateId({s:?})"))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!("    states: &[{states}],\n"));
    }

    out.push_str("    invariants: &[],\n");
    out.push_str(&format!("    provenance: &[{conv_const}],\n"));
    out.push_str("};\n");

    out
}

/// Serialize a single `EntityFact` into a complete, standalone Rust source
/// file: deterministic imports followed by the entity's `const`/`static`
/// block.
pub fn entity_to_rust(fact: &EntityFact, session_id: &str) -> String {
    format!("{IMPORTS}\n{}", entity_body_rust(fact, session_id))
}

/// Merge a new entity's body block into an existing `entities.rs` file's
/// contents (or start a fresh file with imports if `existing` is `None`).
/// Idempotent: re-recording the same entity id is a no-op.
pub fn merge_entity_file(existing: Option<&str>, fact: &EntityFact, session_id: &str) -> String {
    let entity_const_decl = format!("pub const ENTITY_{}:", const_case(&fact.id));
    let body = entity_body_rust(fact, session_id);

    match existing {
        Some(current) if current.contains(&entity_const_decl) => current.to_string(),
        Some(current) => format!("{}\n{}", current.trim_end(), body),
        None => entity_to_rust(fact, session_id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec_fact::compile_check::assert_compiles_against_nocodo_praxis;

    fn task_fact() -> EntityFact {
        EntityFact {
            id: "task".into(),
            description: "A unit of work that can be assigned and tracked".into(),
            fields: vec![
                FieldFact {
                    name: "title".into(),
                    description: "Short name".into(),
                    invariant: None,
                },
                FieldFact {
                    name: "status".into(),
                    description: "Current lifecycle state".into(),
                    invariant: None,
                },
            ],
            state_ids: vec![
                "todo".into(),
                "in_progress".into(),
                "done".into(),
                "cancelled".into(),
            ],
        }
    }

    #[test]
    fn entity_to_rust_contains_expected_identifiers() {
        let fact = task_fact();
        let code = entity_to_rust(&fact, "session-42");

        assert!(code.contains("pub const ENTITY_TASK: EntityId = EntityId(\"task\");"));
        assert!(code.contains("pub static ENTITY_TASK_DEF: Entity = Entity {"));
        assert!(code.contains("name: \"title\","));
        assert!(code.contains("invariants: &[Unresolved::Pending {"));
        assert!(code.contains("reason: \"Constraints for 'title' not specified\","));
        assert!(code.contains("provenance: AtLeastOne {"));
        assert!(code.contains("head: Provenance::Inferred {"));
        assert!(code.contains(r#"from: &["session-42"],"#));
        assert!(code.contains("states: &[StateId(\"todo\"), StateId(\"in_progress\"), StateId(\"done\"), StateId(\"cancelled\")],"));
        assert!(code.contains("invariants: &[],"));
        assert_eq!(code.matches('{').count(), code.matches('}').count());
    }

    #[test]
    fn entity_to_rust_pending_invariants_compiles_against_nocodo_praxis() {
        let fact = task_fact();
        let code = entity_to_rust(&fact, "session-42");
        assert_compiles_against_nocodo_praxis(&code);
    }

    #[test]
    fn entity_to_rust_resolved_invariant_compiles_against_nocodo_praxis() {
        let fact = EntityFact {
            id: "task".into(),
            description: "desc".into(),
            fields: vec![FieldFact {
                name: "title".into(),
                description: "Short name".into(),
                invariant: Some("max 200 chars".into()),
            }],
            state_ids: vec![],
        };
        let code = entity_to_rust(&fact, "session-1");
        assert!(code.contains(r#"invariants: &[Unresolved::Resolved("max 200 chars")],"#));
        assert_compiles_against_nocodo_praxis(&code);
    }

    #[test]
    fn entity_to_rust_no_fields_no_states_compiles_against_nocodo_praxis() {
        let fact = EntityFact {
            id: "empty".into(),
            description: "desc".into(),
            fields: vec![],
            state_ids: vec![],
        };
        let code = entity_to_rust(&fact, "session-1");
        assert!(code.contains("fields: &[],"));
        assert!(code.contains("states: &[],"));
        assert_compiles_against_nocodo_praxis(&code);
    }

    #[test]
    fn merge_entity_file_appends_second_entity() {
        let task = task_fact();
        let user = EntityFact {
            id: "user".into(),
            description: "A registered account".into(),
            fields: vec![],
            state_ids: vec![],
        };

        let file_one = merge_entity_file(None, &task, "session-42");
        let file_two = merge_entity_file(Some(&file_one), &user, "session-43");

        assert!(file_two.contains("pub const ENTITY_TASK: EntityId"));
        assert!(file_two.contains("pub const ENTITY_USER: EntityId"));
        assert_eq!(
            file_two
                .matches("use nocodo_praxis::entity::{Entity, EntityId, Field};")
                .count(),
            1
        );
        assert_compiles_against_nocodo_praxis(&file_two);
    }

    #[test]
    fn merge_entity_file_is_idempotent_for_same_entity() {
        let task = task_fact();
        let file_one = merge_entity_file(None, &task, "session-42");
        let file_two = merge_entity_file(Some(&file_one), &task, "session-42");
        assert_eq!(file_one, file_two);
    }
}
