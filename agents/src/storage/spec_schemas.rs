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
/// The `provenance` field records which session or source produced this note.
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
    /// Source reference — session ID or `"inferred"` tag.
    pub provenance: String,
}

impl PersonaNote {
    /// Convert this note into a `nocodo_praxis::auth::UserPersona`.
    ///
    /// Each string field is leaked to produce `&'static str` references
    /// compatible with praxis types. Uses `Provenance::Conversation` when
    /// `provenance` is a session ID, or `Provenance::Inferred` when tagged
    /// as inferred.
    pub fn to_user_persona(&self) -> nocodo_praxis::auth::UserPersona {
        use nocodo_praxis::auth::{PersonaId, UserPersona};
        use nocodo_praxis::primitives::AtLeastOne;
        use nocodo_praxis::provenance::Provenance;

        let prov = if self.provenance == "inferred" {
            Provenance::Inferred {
                reason: static_str("LLM inferred this persona from conversation context"),
                from: &[],
            }
        } else {
            Provenance::Conversation {
                id: static_str(&self.provenance),
                excerpt: static_str(&format!(
                    "Persona '{}': {}",
                    self.name, self.description
                )),
            }
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
            provenance: "session-42".into(),
        };

        let json = serde_json::to_string(&note).unwrap();
        let parsed: PersonaNote = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "admin");
        assert_eq!(parsed.goals.len(), 2);
    }

    #[test]
    fn persona_note_to_user_persona() {
        let note = PersonaNote {
            id: "member".into(),
            name: "Member".into(),
            description: "Regular team member".into(),
            goals: vec!["Track tasks".into()],
            pain_points: vec![],
            provenance: "session-1".into(),
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
            provenance: "inferred".into(),
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
            provenance: "s1".into(),
        });

        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"type\":\"persona\""));

        let parsed: SpecNoteContent = serde_json::from_str(&json).unwrap();
        match parsed {
            SpecNoteContent::Persona(p) => assert_eq!(p.id, "admin"),
        }
    }
}
