use crate::praxis_doc;
use crate::storage::spec_schemas::PersonaNote;

/// Build the system prompt for `praxis_auth` mode.
///
/// Includes the full `nocodo_praxis::auth` type reference (doc comments + signatures)
/// and concrete construction examples. The user prompt supplies the actual persona
/// data to encode. workspace_root is passed through to the doc extractor.
pub fn build_system_prompt(workspace_root: Option<&std::path::Path>) -> String {
    let auth_ref = praxis_doc::auth_types_reference(workspace_root);
    let prim_ref = praxis_doc::primitives_reference(workspace_root);
    let prov_ref = praxis_doc::provenance_reference(workspace_root);

    format!(
        r#"You write nocodo_praxis persona spec code for a Rust project.

## Output contract

Return ONLY valid Rust code. No imports. No impl blocks. No fn. No mod. No explanation.
No markdown fences. No comments. Just the const and static items.

Output exactly:
1. One `const` per persona identifier (PersonaId).
2. One `static` per persona (UserPersona), using AtLeastOne for provenance.

Use `Provenance::Conversation` when the excerpt comes from a user conversation.
Use `Provenance::Inferred` when the LLM filled in a gap not stated by the user.
If a goals or pain_points list is empty because the user did not specify, set it to `&[]`.

## Types you must use

{prim_ref}

{prov_ref}

{auth_ref}

## Example output (two personas)

const PERSONA_MEMBER: PersonaId = PersonaId("member");
const PERSONA_ADMIN: PersonaId = PersonaId("admin");

static MEMBER_PERSONA: UserPersona = UserPersona {{
    id: PERSONA_MEMBER,
    name: "Team Member",
    description: "Anyone who has registered and confirmed their account.",
    goals: &["Track my own tasks", "See what my team is working on"],
    pain_points: &["No shared task view", "Manual status updates"],
    provenance: AtLeastOne {{
        head: Provenance::Conversation {{
            id: "session-1",
            excerpt: "Team members should be able to track their own tasks and see what others are doing.",
        }},
        tail: &[],
    }},
}};

static ADMIN_PERSONA: UserPersona = UserPersona {{
    id: PERSONA_ADMIN,
    name: "Admin",
    description: "Elevated user who manages the team and task assignment.",
    goals: &["Manage team membership", "Assign tasks to anyone", "Monitor overall progress"],
    pain_points: &["No way to reassign tasks", "Can't remove inactive members"],
    provenance: AtLeastOne {{
        head: Provenance::Conversation {{
            id: "session-1",
            excerpt: "Admin can create tasks, kick members out, assign to anyone.",
        }},
        tail: &[],
    }},
}};

Now write the persona spec from the user's data. Preserve all goals and pain_points exactly as listed.
Use Inferred provenance for any goals or pain_points you infer rather than copied from the input."#
    )
}

/// Build the user prompt supplying the persona data to encode.
pub fn build_user_prompt(personas: &[PersonaNote]) -> String {
    let mut out = String::from("Encode the following personas as nocodo_praxis statics:\n\n");

    for (i, p) in personas.iter().enumerate() {
        out.push_str(&format!("--- Persona {} ---\n", i + 1));
        out.push_str(&format!("id: \"{}\"\n", p.id));
        out.push_str(&format!("name: \"{}\"\n", p.name));
        out.push_str(&format!("description: \"{}\"\n", p.description));

        if p.goals.is_empty() {
            out.push_str("goals: (none specified)\n");
        } else {
            out.push_str("goals:\n");
            for g in &p.goals {
                out.push_str(&format!("  - {}\n", g));
            }
        }

        if p.pain_points.is_empty() {
            out.push_str("pain_points: (none specified)\n");
        } else {
            out.push_str("pain_points:\n");
            for pp in &p.pain_points {
                out.push_str(&format!("  - {}\n", pp));
            }
        }

        let prov = match p.provenance_message_id {
            Some(id) => format!("provenance_message_id: {} (use Conversation)", id),
            None => "provenance_message_id: null (use Inferred)".to_string(),
        };
        out.push_str(&format!("{}\n", prov));

        if let Some(ref reason) = p.incomplete_reason {
            out.push_str(&format!(
                "incomplete_reason: \"{}\" (use Unresolved::Pending for missing fields)\n",
                reason
            ));
        }
        out.push('\n');
    }

    out
}
