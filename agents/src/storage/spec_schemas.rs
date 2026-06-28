//! Structured note content types for PO-to-agent communication.
//!
//! When PO gathers requirements, it records structured notes (personas,
//! permissions, etc.) via `record_project_note`. These types define the
//! JSON schemas for each content kind, so downstream agents (PM, Praxis Writer)
//! can deserialize them deterministically.
//!
//! # Usage
//!
//! ```ignore
//! use agents::storage::spec_schemas::{PersonaNote, SpecNoteContent};
//!
//! let persona = PersonaNote {
//!     id: "admin".into(),
//!     name: "Admin".into(),
//!     description: "Team admin".into(),
//!     goals: vec!["Manage team".into()],
//!     pain_points: vec!["No visibility".into()],
//!     provenance: "session-42".into(),
//! };
//!
//! let json = serde_json::to_string(&persona).unwrap();
//! // Store json as the `note` field in record_project_note
//! ```

use serde::{Deserialize, Serialize};

/// Wrapper for structured note content stored in `project_note.note`.
///
/// Serialized to JSON and stored as the note text. The variant determines
/// how downstream consumers parse the content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SpecNoteContent {
    /// A user persona note, produced by PO's persona interview mode.
    #[serde(rename = "persona")]
    Persona(PersonaNote),
    // Future:
    // Permission(PermissionNote),
    // Role(RoleNote),
}

/// A user persona captured during requirements intake.
///
/// Each field maps to a corresponding field on `nocodo_praxis::auth::UserPersona`.
/// `provenance_message_id` is an FK to `user_chat_message.id` — the exact user
/// message where this persona was described. `None` means inferred by the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaNote {
    /// Short identifier, e.g. `"admin"`, `"member"`.
    pub id: String,
    /// Human-readable display name, e.g. `"Team Admin"`.
    pub name: String,
    /// One to two sentences describing this persona's role and context.
    pub description: String,
    /// What this persona needs to accomplish with the software.
    #[serde(default)]
    pub goals: Vec<String>,
    /// What frustrates this persona without the software.
    #[serde(default)]
    pub pain_points: Vec<String>,
    /// FK to user_chat_message.id where the user described this persona.
    /// `None` if the LLM inferred this persona from context.
    #[serde(default)]
    pub provenance_message_id: Option<i64>,
    /// When set, some required data was not provided by the user.
    /// Maps to `Unresolved::Pending { reason }` in generated praxis code.
    #[serde(default)]
    pub incomplete_reason: Option<String>,
}

impl PersonaNote {
    /// Convert this note into a `nocodo_praxis::auth::UserPersona`.
    ///
    /// Each string field is leaked to produce `&'static str` references
    /// compatible with praxis types. Uses `Provenance::Conversation` when
    /// `provenance_message_id` is set, or `Provenance::Inferred` otherwise.
    pub fn to_user_persona(&self) -> nocodo_praxis::auth::UserPersona {
        use nocodo_praxis::auth::{PersonaId, UserPersona};
        use nocodo_praxis::primitives::AtLeastOne;
        use nocodo_praxis::provenance::Provenance;

        let prov = match self.provenance_message_id {
            Some(msg_id) => Provenance::Conversation {
                id: static_str(&format!("message-{}", msg_id)),
                excerpt: static_str(&format!("Persona '{}': {}", self.name, self.description)),
            },
            None => Provenance::Inferred {
                reason: static_str("LLM inferred this persona from conversation context"),
                from: &[],
            },
        };

        let goals: Vec<&'static str> = self.goals.iter().map(|s| static_str(s)).collect();
        let pain_points: Vec<&'static str> =
            self.pain_points.iter().map(|s| static_str(s)).collect();

        UserPersona {
            id: PersonaId(static_str(&self.id)),
            name: static_str(&self.name),
            description: static_str(&self.description),
            goals: leak_vec(goals),
            pain_points: leak_vec(pain_points),
            provenance: AtLeastOne {
                head: prov,
                tail: &[],
            },
        }
    }
}

/// Leak a `String` to get a `&'static str`. Only for use in spec definitions
/// (static data that lives for the lifetime of the process).
fn static_str(s: &str) -> &'static str {
    Box::leak(s.to_string().into_boxed_str())
}

/// Leak a `Vec<T>` to get a `&'static [T]`.
fn leak_vec<T>(v: Vec<T>) -> &'static [T] {
    Box::leak(v.into_boxed_slice())
}

/// PM's structured output for Praxis Writer (RustEngineer `praxis_auth` mode) consumption.
///
/// Assembled by the backend after PM finalises a session: PM writes the task
/// instructions as free text; Rust reads the persona notes from the database and
/// hydrates this struct. Stored as JSON in `task.description` for any task with
/// `assigned_to_agent = "praxis_engineer"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PraxisWriterTaskSpec {
    /// The RustEngineer mode to invoke, e.g. `"praxis_auth"`.
    pub mode: String,
    /// Persona data extracted from PO's structured project notes.
    pub personas: Vec<PersonaNote>,
    /// PM's natural-language instructions to the Praxis Writer.
    pub instructions: String,
    /// IDs of the `project_note` rows that informed this task.
    pub source_note_ids: Vec<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persona_note_serialization_roundtrips() {
        let note = PersonaNote {
            id: "admin".into(),
            name: "Admin".into(),
            description: "Team admin who manages members".into(),
            goals: vec!["Manage team".into(), "Assign tasks".into()],
            pain_points: vec!["No visibility".into()],
            provenance_message_id: Some(42),
            incomplete_reason: None,
        };

        let json = serde_json::to_string(&note).unwrap();
        let parsed: PersonaNote = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "admin");
        assert_eq!(parsed.goals.len(), 2);
        assert_eq!(parsed.provenance_message_id, Some(42));
    }

    #[test]
    fn persona_note_with_incomplete_reason() {
        let note = PersonaNote {
            id: "admin".into(),
            name: "Admin".into(),
            description: "Team admin".into(),
            goals: vec![],
            pain_points: vec![],
            provenance_message_id: Some(1),
            incomplete_reason: Some("User couldn't list goals for admin".into()),
        };

        let json = serde_json::to_string(&note).unwrap();
        let parsed: PersonaNote = serde_json::from_str(&json).unwrap();
        assert_eq!(
            parsed.incomplete_reason.as_deref(),
            Some("User couldn't list goals for admin")
        );
    }

    #[test]
    fn persona_note_to_user_persona() {
        let note = PersonaNote {
            id: "member".into(),
            name: "Member".into(),
            description: "Regular team member".into(),
            goals: vec!["Track tasks".into()],
            pain_points: vec![],
            provenance_message_id: Some(1),
            incomplete_reason: None,
        };

        let persona = note.to_user_persona();
        assert_eq!(persona.id.0, "member");
        assert_eq!(persona.name, "Member");
        assert_eq!(persona.goals.len(), 1);
    }

    #[test]
    fn inferred_persona_uses_inferred_provenance() {
        let note = PersonaNote {
            id: "viewer".into(),
            name: "Viewer".into(),
            description: "Read-only user".into(),
            goals: vec![],
            pain_points: vec![],
            provenance_message_id: None,
            incomplete_reason: None,
        };

        let persona = note.to_user_persona();
        let prov = &persona.provenance.head;
        assert!(
            matches!(prov, nocodo_praxis::provenance::Provenance::Inferred { .. }),
            "Expected Inferred provenance, got: {prov:?}"
        );
    }

    #[test]
    fn spec_note_content_serialization() {
        let content = SpecNoteContent::Persona(PersonaNote {
            id: "admin".into(),
            name: "Admin".into(),
            description: "desc".into(),
            goals: vec![],
            pain_points: vec![],
            provenance_message_id: Some(1),
            incomplete_reason: None,
        });

        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"type\":\"persona\""));

        let parsed: SpecNoteContent = serde_json::from_str(&json).unwrap();
        match parsed {
            SpecNoteContent::Persona(p) => assert_eq!(p.id, "admin"),
        }
    }
}
