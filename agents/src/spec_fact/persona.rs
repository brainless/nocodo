//! Deterministic `PersonaNote` → `nocodo_praxis` Rust source serialization.
//!
//! See `internal-docs/MILESTONE_PRAXIS_v1.md` §5.3 (Type 1: UserPersona). This is
//! the first `record_spec_fact` type — no LLM call, just string formatting. The
//! `PersonaNote → UserPersona` *runtime* conversion (`PersonaNote::to_user_persona`)
//! already existed from the prior milestone (`agents/src/storage/spec_schemas.rs`);
//! this module instead emits *source text* declaring the equivalent `const`/`static`
//! items so they can be written into the managed project.

use super::{const_case, str_slice_literal};
use crate::storage::spec_schemas::PersonaNote;

/// Deterministic imports every generated `personas.rs` file needs.
///
/// Note: the milestone doc's §5.3 example shows `provenance: &[CONV_X]` (a bare
/// slice), but `nocodo_praxis::auth::UserPersona::provenance` is actually typed
/// `AtLeastOne<Provenance>` (see `nocodo_praxis/src/auth.rs`). We serialize
/// against the real type, so `primitives::AtLeastOne` is imported too — this is
/// a deliberate deviation from the doc's illustrative snippet, not an oversight.
const IMPORTS: &str = "use nocodo_praxis::auth::{PersonaId, UserPersona};\nuse nocodo_praxis::primitives::AtLeastOne;\nuse nocodo_praxis::provenance::Provenance;\n";

/// Emit the `const CONV_{ID}` / `pub const PERSONA_{ID}` / `pub static {ID}_persona`
/// block for one persona — everything except the leading `use` imports.
///
/// Exposed separately from [`persona_to_rust`] so callers merging several
/// personas into one `personas.rs` file can write the imports once and append
/// one body block per persona.
pub fn persona_body_rust(note: &PersonaNote, session_id: &str) -> String {
    // Bug fix (Praxis v1 Phase 7): the conv const must be unique per fact,
    // not just per session — recording two personas in the same chat session
    // (the normal case; a whole PO loop is one session) previously produced
    // two `const CONV_{session}` declarations with the identical name in the
    // merged `personas.rs`, a duplicate-definition compile error (E0428).
    // Suffixing with the fact's own id keeps names unique while still
    // grouping by session for readability.
    let conv_const = format!("CONV_{}_{}", const_case(session_id), const_case(&note.id));
    let persona_const = format!("PERSONA_{}", const_case(&note.id));
    let persona_static = format!("{}_persona", const_case(&note.id));

    let mut out = String::new();
    out.push_str(&format!("const {conv_const}: Provenance = Provenance::Conversation {{\n"));
    out.push_str(&format!("    id: {session_id:?},\n"));
    out.push_str(&format!("    excerpt: {:?},\n", note.description));
    out.push_str("};\n\n");

    out.push_str(&format!(
        "pub const {persona_const}: PersonaId = PersonaId({:?});\n\n",
        note.id
    ));

    out.push_str("#[allow(non_upper_case_globals)]\n");
    out.push_str(&format!("pub static {persona_static}: UserPersona = UserPersona {{\n"));
    out.push_str(&format!("    id: {persona_const},\n"));
    out.push_str(&format!("    name: {:?},\n", note.name));
    out.push_str(&format!("    description: {:?},\n", note.description));
    out.push_str(&format!("    goals: {},\n", str_slice_literal(&note.goals)));
    out.push_str(&format!(
        "    pain_points: {},\n",
        str_slice_literal(&note.pain_points)
    ));
    out.push_str(&format!(
        "    provenance: AtLeastOne {{ head: {conv_const}, tail: &[] }},\n"
    ));
    out.push_str("};\n");

    out
}

/// Serialize a single `PersonaNote` into a complete, standalone Rust source
/// file: deterministic imports followed by the persona's `const`/`static`
/// block. See MILESTONE_PRAXIS_v1.md §5.3 for the expected shape.
///
/// `session_id` becomes the `Provenance::Conversation { id, .. }` value and
/// seeds the `CONV_{ID}` constant name — pass whatever chat session identifier
/// the fact was recorded in.
pub fn persona_to_rust(note: &PersonaNote, session_id: &str) -> String {
    format!("{IMPORTS}\n{}", persona_body_rust(note, session_id))
}

/// Merge a new persona's body block into an existing `personas.rs` file's
/// contents (or start a fresh file with imports if `existing` is `None`).
///
/// Idempotent: if the persona's `pub const PERSONA_{ID}` declaration is
/// already present in `existing`, the file is returned unchanged rather than
/// appending a duplicate (re-recording the same persona is a no-op).
pub fn merge_persona_file(existing: Option<&str>, note: &PersonaNote, session_id: &str) -> String {
    let persona_const_decl = format!("pub const PERSONA_{}:", const_case(&note.id));
    let body = persona_body_rust(note, session_id);

    match existing {
        Some(current) if current.contains(&persona_const_decl) => current.to_string(),
        Some(current) => format!("{}\n{}", current.trim_end(), body),
        None => persona_to_rust(note, session_id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn admin_note() -> PersonaNote {
        PersonaNote {
            id: "admin".into(),
            name: "Team Admin".into(),
            description: "Manages team and task assignment".into(),
            goals: vec!["Manage team membership".into(), "Create tasks".into()],
            pain_points: vec!["Manual tracking".into(), "No visibility".into()],
            provenance_message_id: Some(42),
            incomplete_reason: None,
        }
    }

    #[test]
    fn persona_to_rust_contains_expected_identifiers() {
        let note = admin_note();
        let code = persona_to_rust(&note, "session-42");

        assert!(code.contains("use nocodo_praxis::auth::{PersonaId, UserPersona};"));
        assert!(code.contains("use nocodo_praxis::primitives::AtLeastOne;"));
        assert!(code.contains("use nocodo_praxis::provenance::Provenance;"));
        assert!(code.contains("const CONV_SESSION_42_ADMIN: Provenance = Provenance::Conversation {"));
        assert!(code.contains("id: \"session-42\","));
        assert!(code.contains("excerpt: \"Manages team and task assignment\","));
        assert!(code.contains("pub const PERSONA_ADMIN: PersonaId = PersonaId(\"admin\");"));
        assert!(code.contains("pub static ADMIN_persona: UserPersona = UserPersona {"));
        assert!(code.contains("id: PERSONA_ADMIN,"));
        assert!(code.contains("name: \"Team Admin\","));
        assert!(code.contains(r#"goals: &["Manage team membership", "Create tasks"],"#));
        assert!(code.contains(r#"pain_points: &["Manual tracking", "No visibility"],"#));
        assert!(code.contains("provenance: AtLeastOne { head: CONV_SESSION_42_ADMIN, tail: &[] },"));
        // Well-formed: braces balance.
        assert_eq!(
            code.matches('{').count(),
            code.matches('}').count(),
            "unbalanced braces in generated code:\n{code}"
        );
    }

    #[test]
    fn persona_to_rust_handles_empty_goals_and_pain_points() {
        let mut note = admin_note();
        note.goals.clear();
        note.pain_points.clear();
        let code = persona_to_rust(&note, "session-1");
        assert!(code.contains("goals: &[],"));
        assert!(code.contains("pain_points: &[],"));
    }

    #[test]
    fn persona_to_rust_escapes_quotes_in_strings() {
        let mut note = admin_note();
        note.description = r#"Manages the "core" team"#.into();
        let code = persona_to_rust(&note, "session-1");
        assert!(code.contains(r#"excerpt: "Manages the \"core\" team","#));
    }

    #[test]
    fn merge_persona_file_appends_second_persona() {
        let admin = admin_note();
        let mut member = admin_note();
        member.id = "member".into();
        member.name = "Team Member".into();
        member.description = "Tracks their own tasks".into();

        let file_one = merge_persona_file(None, &admin, "session-42");
        let file_two = merge_persona_file(Some(&file_one), &member, "session-43");

        assert!(file_two.contains("pub const PERSONA_ADMIN: PersonaId"));
        assert!(file_two.contains("pub const PERSONA_MEMBER: PersonaId"));
        assert!(file_two.contains("CONV_SESSION_42_ADMIN"));
        assert!(file_two.contains("CONV_SESSION_43_MEMBER"));
        // Imports appear exactly once even after merging.
        assert_eq!(
            file_two
                .matches("use nocodo_praxis::auth::{PersonaId, UserPersona};")
                .count(),
            1
        );
    }

    #[test]
    fn merge_persona_file_is_idempotent_for_same_persona() {
        let admin = admin_note();
        let file_one = merge_persona_file(None, &admin, "session-42");
        let file_two = merge_persona_file(Some(&file_one), &admin, "session-42");
        assert_eq!(file_one, file_two);
    }
}
