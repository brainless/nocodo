//! Spec gap detection for the clarification loop.
//!
//! After Praxis Writer generates code, this module scans for gaps — missing data
//! that prevents the spec from being complete. Gaps flow back to PO for
//! re-interviewing, then Praxis Writer re-runs with the filled data.
//!
//! Two gap sources:
//! - **PersonaNote level**: `incomplete_reason` set on structured notes (pre-generation)
//! - **Generated code level**: `Unresolved::Pending` patterns in LLM output (post-generation)

use serde::{Deserialize, Serialize};

use crate::storage::spec_schemas::PersonaNote;

/// A gap in the spec that needs user input to resolve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecGap {
    /// What kind of artifact has the gap (e.g. "persona").
    pub artifact_type: String,
    /// Identifier of the specific artifact (e.g. "admin").
    pub artifact_id: String,
    /// Which field or aspect is missing (e.g. "goals", "pain_points").
    pub field: String,
    /// Human-readable reason for the gap.
    pub reason: String,
    /// Whether the user explicitly declined to answer this in a prior iteration.
    #[serde(default)]
    pub user_declined: bool,
}

/// Find gaps in persona notes that have `incomplete_reason` set.
///
/// Each incomplete note produces one gap per missing dimension (goals, pain_points).
/// The field names are inferred from the `incomplete_reason` text — if it mentions
/// "goals" or "pain_points" specifically, we use those; otherwise we produce a
/// generic "data" gap.
pub fn find_gaps_from_persona_notes(personas: &[PersonaNote]) -> Vec<SpecGap> {
    let mut gaps = Vec::new();

    for persona in personas {
        if let Some(ref reason) = persona.incomplete_reason {
            let fields = infer_missing_fields(reason);
            for field in fields {
                gaps.push(SpecGap {
                    artifact_type: "persona".to_string(),
                    artifact_id: persona.id.clone(),
                    field,
                    reason: reason.clone(),
                    user_declined: false,
                });
            }
        }
    }

    gaps
}

/// Scan generated Rust code for `Unresolved::Pending { reason: "..." }` patterns.
///
/// This catches gaps the LLM emitted in the generated praxis spec code.
/// Simple regex/string scanning — not tree-sitter (that comes in Phase 4i).
pub fn find_pending_gaps_in_code(code: &str) -> Vec<SpecGap> {
    let mut gaps = Vec::new();

    // Match patterns like:
    //   Unresolved::Pending {
    //       reason: "Maximum title length not specified",
    //       ...
    //   }
    // Also handles single-line: Unresolved::Pending { reason: "...", ... }
    let mut chars = code.char_indices().peekable();

    while let Some((i, ch)) = chars.next() {
        // Look for "Unresolved::Pending"
        if ch == 'U' && code[i..].starts_with("Unresolved::Pending") {
            // Find the opening brace
            let after_keyword = &code[i + "Unresolved::Pending".len()..];
            let brace_pos = match after_keyword.find('{') {
                Some(p) => p,
                None => continue,
            };

            // Find the matching closing brace
            let content_start = brace_pos + 1;
            let mut depth = 1u32;
            let mut content_end = None;
            for (j, c) in after_keyword[content_start..].char_indices() {
                match c {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            content_end = Some(content_start + j);
                            break;
                        }
                    }
                    _ => {}
                }
            }

            let Some(end) = content_end else { continue };
            let block = &after_keyword[..end];

            // Extract reason string
            if let Some(reason) = extract_reason(block) {
                // Try to figure out what artifact this belongs to by looking
                // backwards in the code for identifiers
                let context = &code[..i];
                let artifact_id = extract_nearest_identifier(context);
                let field = extract_field_name(block);

                gaps.push(SpecGap {
                    artifact_type: "persona".to_string(),
                    artifact_id,
                    field,
                    reason: reason.to_string(),
                    user_declined: false,
                });
            }
        }
    }

    gaps
}

/// Extract the `reason: "..."` value from an Unresolved::Pending block.
fn extract_reason(block: &str) -> Option<&str> {
    // Find `reason:` then the next string literal
    let reason_pos = block.find("reason:")?;
    let after = block[reason_pos + 7..].trim_start();

    // Find opening quote
    let quote_start = after.find('"')?;
    let after_quote = &after[quote_start + 1..];

    // Find closing quote (handle escaped quotes)
    let mut end = 0;
    let mut escaped = false;
    for c in after_quote.chars() {
        if escaped {
            escaped = false;
            end += c.len_utf8();
            continue;
        }
        if c == '\\' {
            escaped = true;
            end += c.len_utf8();
            continue;
        }
        if c == '"' {
            break;
        }
        end += c.len_utf8();
    }

    Some(&after_quote[..end])
}

/// Extract the `field_name` or similar identifier from an Unresolved block.
fn extract_field_name(block: &str) -> String {
    // Look for common field patterns
    if block.contains("title") {
        "title".to_string()
    } else if block.contains("goals") || block.contains("goal") {
        "goals".to_string()
    } else if block.contains("pain_points") || block.contains("pain") {
        "pain_points".to_string()
    } else if block.contains("description") {
        "description".to_string()
    } else {
        "unknown".to_string()
    }
}

/// Walk backwards from current position to find the nearest identifier
/// (e.g., a const or static name like `PERSONA_ADMIN`).
fn extract_nearest_identifier(context: &str) -> String {
    // Look for the last identifier before the Unresolved usage
    let trimmed = context.trim_end();

    // Walk backwards to find an identifier pattern
    let mut ident = String::new();
    for c in trimmed.chars().rev() {
        if c.is_alphanumeric() || c == '_' {
            ident.push(c);
        } else if !ident.is_empty() {
            break;
        }
    }

    ident.chars().rev().collect()
}

/// Infer which fields are missing from an `incomplete_reason` string.
fn infer_missing_fields(reason: &str) -> Vec<String> {
    let lower = reason.to_lowercase();
    let mut fields = Vec::new();

    if lower.contains("goal") {
        fields.push("goals".to_string());
    }
    if lower.contains("pain") {
        fields.push("pain_points".to_string());
    }
    if lower.contains("description") {
        fields.push("description".to_string());
    }

    // If we couldn't infer specific fields, flag the whole persona
    if fields.is_empty() {
        fields.push("data".to_string());
    }

    fields
}

/// Build structured questions for PO from a list of gaps.
///
/// Groups gaps by artifact_id and produces one question per artifact
/// (clustering related gaps per RUNTIME.md §6.3 guidance).
pub fn gaps_to_questions(gaps: &[SpecGap]) -> Vec<GapQuestion> {
    // Group by artifact_id
    let mut by_artifact: std::collections::HashMap<&str, Vec<&SpecGap>> =
        std::collections::HashMap::new();
    for gap in gaps {
        by_artifact
            .entry(&gap.artifact_id)
            .or_default()
            .push(gap);
    }

    by_artifact
        .into_iter()
        .map(|(artifact_id, gaps)| {
            let fields: Vec<&str> = gaps.iter().map(|g| g.field.as_str()).collect();
            let reasons: Vec<&str> = gaps.iter().map(|g| g.reason.as_str()).collect();

            GapQuestion {
                artifact_id: artifact_id.to_string(),
                artifact_type: gaps[0].artifact_type.clone(),
                missing_fields: fields.into_iter().map(String::from).collect(),
                reason: reasons.join("; "),
            }
        })
        .collect()
}

/// A clustered question for PO about a specific artifact's gaps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GapQuestion {
    /// Which artifact needs more data (e.g. "admin").
    pub artifact_id: String,
    /// What kind of artifact (e.g. "persona").
    pub artifact_type: String,
    /// Which fields are missing.
    pub missing_fields: Vec<String>,
    /// Combined reason text for the LLM prompt.
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_persona(id: &str, incomplete: Option<&str>) -> PersonaNote {
        PersonaNote {
            id: id.to_string(),
            name: format!("{} Name", id),
            description: "desc".to_string(),
            goals: vec![],
            pain_points: vec![],
            provenance_message_id: Some(1),
            incomplete_reason: incomplete.map(String::from),
        }
    }

    #[test]
    fn gaps_from_complete_persona_is_empty() {
        let personas = vec![make_persona("admin", None)];
        assert!(find_gaps_from_persona_notes(&personas).is_empty());
    }

    #[test]
    fn gaps_from_persona_with_goals_missing() {
        let personas = vec![make_persona("admin", Some("Goals not specified"))];
        let gaps = find_gaps_from_persona_notes(&personas);
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].artifact_id, "admin");
        assert_eq!(gaps[0].field, "goals");
    }

    #[test]
    fn gaps_from_persona_with_multiple_fields() {
        let personas = vec![make_persona(
            "admin",
            Some("Goals and pain points not collected"),
        )];
        let gaps = find_gaps_from_persona_notes(&personas);
        assert_eq!(gaps.len(), 2);
        assert!(gaps.iter().any(|g| g.field == "goals"));
        assert!(gaps.iter().any(|g| g.field == "pain_points"));
    }

    #[test]
    fn gaps_from_generic_reason() {
        let personas = vec![make_persona("viewer", Some("User skipped this persona"))];
        let gaps = find_gaps_from_persona_notes(&personas);
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].field, "data");
    }

    #[test]
    fn gaps_from_multiple_personas() {
        let personas = vec![
            make_persona("admin", Some("Goals not specified")),
            make_persona("member", None),
            make_persona("viewer", Some("Pain points not collected")),
        ];
        let gaps = find_gaps_from_persona_notes(&personas);
        assert_eq!(gaps.len(), 2);
        assert!(gaps.iter().any(|g| g.artifact_id == "admin"));
        assert!(gaps.iter().any(|g| g.artifact_id == "viewer"));
    }

    #[test]
    fn find_pending_gaps_in_code_finds_unresolved() {
        let code = r#"
pub static ADMIN_PERSONA: UserPersona = UserPersona {
    id: PERSONA_ADMIN,
    goals: &[],
    pain_points: &[],
};

Unresolved::Pending {
    reason: "Goals not specified for admin",
    provenance: ...,
}
"#;
        let gaps = find_pending_gaps_in_code(code);
        assert_eq!(gaps.len(), 1);
        assert_eq!(gaps[0].reason, "Goals not specified for admin");
    }

    #[test]
    fn find_pending_gaps_in_code_empty_on_clean_output() {
        let code = r#"
pub static ADMIN_PERSONA: UserPersona = UserPersona {
    id: PERSONA_ADMIN,
    name: "Admin",
    goals: &["Manage team"],
};
"#;
        let gaps = find_pending_gaps_in_code(code);
        assert!(gaps.is_empty());
    }

    #[test]
    fn extract_reason_from_block() {
        let block = r#"reason: "Maximum title length not specified",
            provenance: AtLeastOne { head: ... }"#;
        assert_eq!(
            extract_reason(block),
            Some("Maximum title length not specified")
        );
    }

    #[test]
    fn gaps_to_questions_clusters_by_artifact() {
        let gaps = vec![
            SpecGap {
                artifact_type: "persona".into(),
                artifact_id: "admin".into(),
                field: "goals".into(),
                reason: "Goals not specified".into(),
                user_declined: false,
            },
            SpecGap {
                artifact_type: "persona".into(),
                artifact_id: "admin".into(),
                field: "pain_points".into(),
                reason: "Pain points not collected".into(),
                user_declined: false,
            },
            SpecGap {
                artifact_type: "persona".into(),
                artifact_id: "viewer".into(),
                field: "data".into(),
                reason: "User skipped viewer".into(),
                user_declined: true,
            },
        ];
        let questions = gaps_to_questions(&gaps);
        assert_eq!(questions.len(), 2);

        let admin_q = questions.iter().find(|q| q.artifact_id == "admin").unwrap();
        assert_eq!(admin_q.missing_fields.len(), 2);
        assert!(admin_q.missing_fields.contains(&"goals".to_string()));
        assert!(admin_q.missing_fields.contains(&"pain_points".to_string()));

        let viewer_q = questions.iter().find(|q| q.artifact_id == "viewer").unwrap();
        assert_eq!(viewer_q.missing_fields.len(), 1);
    }
}
