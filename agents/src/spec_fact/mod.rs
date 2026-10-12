//! `record_spec_fact` — the bridge between "PO gathered a fact from the user
//! in chat" and "valid `nocodo_praxis` Rust code exists in the managed project."
//!
//! See `internal-docs/MILESTONE_PRAXIS_v1.md` §5. PO emits structured JSON; this
//! module deserializes it into the matching Rust struct, runs a deterministic
//! (no-LLM) serializer, and writes the result into the project via
//! `schema_codegen`'s atomic-write helpers.
//!
//! **Phase 4 status:** all five `fact_type`s are implemented —
//! `"persona"` (Phase 3), `"permission"`, `"role"`, `"state"`, `"entity"`
//! (Phase 4, see §5.4-5.7). Each type's serializer is deterministic (no LLM
//! call) and lives in its own submodule, following the pattern
//! `persona.rs` established: a `{type}_body_rust` formatter, a
//! `{type}_to_rust` whole-file formatter, and a `merge_{type}_file` that
//! appends-or-no-ops into an existing spec file.

pub mod entity;
pub mod persona;
pub mod permission;
pub mod role;
pub mod state;

#[cfg(test)]
pub(crate) mod compile_check;

use std::path::Path;

use crate::code_writer;
use crate::error::AgentError;
use crate::storage::spec_schemas::PersonaNote;

/// Upper-snake-case a string for use as a Rust `const`/`static` identifier
/// fragment: keep ASCII alphanumerics, uppercase them, replace everything
/// else with `_`.
///
/// Shared by every `record_spec_fact` serializer (persona, permission, role,
/// state, entity) — see `persona.rs`'s original definition, which this
/// replaces as the single source of truth.
pub(crate) fn const_case(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect()
}

/// Format a list of strings as a Rust `&["a", "b"]` slice literal, with each
/// element escaped via `Debug` (handles embedded quotes/newlines safely).
pub(crate) fn str_slice_literal(items: &[String]) -> String {
    let inner = items
        .iter()
        .map(|s| format!("{s:?}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!("&[{inner}]")
}

/// Parameters for the `record_spec_fact` tool (MILESTONE_PRAXIS_v1.md §5.9).
///
/// PO does not know Rust syntax — it emits structured JSON with field names
/// and values. `fact_type` selects which struct `data` deserializes into and
/// which serializer runs. Only `"persona"` is handled in Phase 3.
///
/// Derives `JsonSchema` so it can be exposed directly as a tool definition to
/// the LLM (Phase 3.5 — `agents/src/product_owner/agent.rs::run_loop`).
#[derive(Debug, Clone, serde::Deserialize, schemars::JsonSchema)]
pub struct RecordSpecFactParams {
    /// Type of spec fact: `"persona"` (Phase 3). Future: `"permission"`,
    /// `"role"`, `"state"`, `"entity"` (Phase 4).
    pub fact_type: String,
    /// The structured data — schema depends on `fact_type`.
    pub data: serde_json::Value,
}

/// Result of a successful `record_spec_fact` call: where the code landed and
/// which identifier it declared, so the caller (PO, or a test) can report
/// back or reference it in a follow-up.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct RecordSpecFactResult {
    /// Project-relative path written, e.g. `backend/src/praxis/specs/personas.rs`.
    pub file_path: String,
    /// The fact's identifier, e.g. `"admin"`.
    pub identifier: String,
}

/// Handle a `record_spec_fact` call: deserialize `data` per `fact_type`, run
/// the matching deterministic serializer, and atomically write the result
/// into `project_root` via `schema_codegen`.
///
/// `session_id` is the chat session the fact was recorded in — it becomes the
/// `Provenance::Conversation { id, .. }` value in the generated code.
///
/// Recording a persona that already exists in `personas.rs` (same `id`) is a
/// no-op for that persona's block — see `persona::merge_persona_file`.
pub fn record_spec_fact(
    project_root: &Path,
    session_id: &str,
    params: RecordSpecFactParams,
) -> Result<RecordSpecFactResult, AgentError> {
    match params.fact_type.as_str() {
        "persona" => record_persona_fact(project_root, session_id, params.data),
        "permission" => record_permission_fact(project_root, session_id, params.data),
        "role" => record_role_fact(project_root, session_id, params.data),
        "state" => record_state_fact(project_root, session_id, params.data),
        "entity" => record_entity_fact(project_root, session_id, params.data),
        other => Err(AgentError::Other(format!(
            "record_spec_fact: unsupported fact_type '{other}' — expected one of \
             'persona', 'permission', 'role', 'state', 'entity'"
        ))),
    }
}

fn record_persona_fact(
    project_root: &Path,
    session_id: &str,
    data: serde_json::Value,
) -> Result<RecordSpecFactResult, AgentError> {
    let note: PersonaNote = serde_json::from_value(data)?;

    let rel_path = "backend/src/praxis/specs/personas.rs";
    let abs_path = project_root.join(rel_path);
    let existing = std::fs::read_to_string(&abs_path).ok();

    let merged = persona::merge_persona_file(existing.as_deref(), &note, session_id);

    let file_path = code_writer::write_praxis_spec(project_root, "personas", &merged)?;

    Ok(RecordSpecFactResult {
        file_path,
        identifier: note.id,
    })
}

fn record_permission_fact(
    project_root: &Path,
    session_id: &str,
    data: serde_json::Value,
) -> Result<RecordSpecFactResult, AgentError> {
    let fact: permission::PermissionFact = serde_json::from_value(data)?;

    let rel_path = "backend/src/praxis/specs/permissions.rs";
    let abs_path = project_root.join(rel_path);
    let existing = std::fs::read_to_string(&abs_path).ok();

    let merged = permission::merge_permission_file(existing.as_deref(), &fact, session_id);

    let file_path = code_writer::write_praxis_spec(project_root, "permissions", &merged)?;

    Ok(RecordSpecFactResult {
        file_path,
        identifier: fact.id,
    })
}

fn record_role_fact(
    project_root: &Path,
    session_id: &str,
    data: serde_json::Value,
) -> Result<RecordSpecFactResult, AgentError> {
    let fact: role::RoleFact = serde_json::from_value(data)?;

    let rel_path = "backend/src/praxis/specs/roles.rs";
    let abs_path = project_root.join(rel_path);
    let existing = std::fs::read_to_string(&abs_path).ok();

    let merged = role::merge_role_file(existing.as_deref(), &fact, session_id)?;

    let file_path = code_writer::write_praxis_spec(project_root, "roles", &merged)?;

    Ok(RecordSpecFactResult {
        file_path,
        identifier: fact.id,
    })
}

fn record_state_fact(
    project_root: &Path,
    session_id: &str,
    data: serde_json::Value,
) -> Result<RecordSpecFactResult, AgentError> {
    let fact: state::StateFact = serde_json::from_value(data)?;

    let rel_path = "backend/src/praxis/specs/states.rs";
    let abs_path = project_root.join(rel_path);
    let existing = std::fs::read_to_string(&abs_path).ok();

    let merged = state::merge_state_file(existing.as_deref(), &fact, session_id)?;

    let file_path = code_writer::write_praxis_spec(project_root, "states", &merged)?;

    Ok(RecordSpecFactResult {
        file_path,
        identifier: fact.id,
    })
}

fn record_entity_fact(
    project_root: &Path,
    session_id: &str,
    data: serde_json::Value,
) -> Result<RecordSpecFactResult, AgentError> {
    let fact: entity::EntityFact = serde_json::from_value(data)?;

    let rel_path = "backend/src/praxis/specs/entities.rs";
    let abs_path = project_root.join(rel_path);
    let existing = std::fs::read_to_string(&abs_path).ok();

    let merged = entity::merge_entity_file(existing.as_deref(), &fact, session_id);

    let file_path = code_writer::write_praxis_spec(project_root, "entities", &merged)?;

    Ok(RecordSpecFactResult {
        file_path,
        identifier: fact.id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    /// A fresh scratch directory per test, so parallel tests don't collide.
    fn scratch_project_root() -> std::path::PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "nocodo-agents-spec-fact-test-{}-{}",
            std::process::id(),
            n
        ));
        std::fs::create_dir_all(&dir).expect("create scratch dir");
        dir
    }

    fn persona_json(id: &str, name: &str) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "name": name,
            "description": format!("{name} description"),
            "goals": ["Do the thing"],
            "pain_points": ["Can't do the thing"],
            "provenance_message_id": 42,
        })
    }

    #[test]
    fn record_spec_fact_rejects_unknown_fact_type() {
        let root = scratch_project_root();
        let err = record_spec_fact(
            &root,
            "session-1",
            RecordSpecFactParams {
                fact_type: "transition_guard".into(),
                data: serde_json::json!({}),
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("unsupported fact_type"));
    }

    #[test]
    fn record_spec_fact_persona_writes_file_and_registers_module() {
        let root = scratch_project_root();
        let result = record_spec_fact(
            &root,
            "session-42",
            RecordSpecFactParams {
                fact_type: "persona".into(),
                data: persona_json("admin", "Team Admin"),
            },
        )
        .expect("record_spec_fact should succeed");

        assert_eq!(result.file_path, "backend/src/praxis/specs/personas.rs");
        assert_eq!(result.identifier, "admin");

        let written = std::fs::read_to_string(root.join(&result.file_path)).unwrap();
        assert!(written.contains("pub const PERSONA_ADMIN: PersonaId = PersonaId(\"admin\");"));
        assert!(written.contains("pub static ADMIN_persona: UserPersona"));

        // mod.rs registration (schema_codegen::register_praxis_module).
        let praxis_mod =
            std::fs::read_to_string(root.join("backend/src/praxis/mod.rs")).unwrap();
        assert!(praxis_mod.contains("pub mod specs;"));
        let specs_mod =
            std::fs::read_to_string(root.join("backend/src/praxis/specs/mod.rs")).unwrap();
        assert!(specs_mod.contains("pub mod personas;"));
    }

    #[test]
    fn record_spec_fact_persona_accumulates_multiple_personas() {
        let root = scratch_project_root();
        record_spec_fact(
            &root,
            "session-42",
            RecordSpecFactParams {
                fact_type: "persona".into(),
                data: persona_json("admin", "Team Admin"),
            },
        )
        .unwrap();
        record_spec_fact(
            &root,
            "session-43",
            RecordSpecFactParams {
                fact_type: "persona".into(),
                data: persona_json("member", "Team Member"),
            },
        )
        .unwrap();

        let written = std::fs::read_to_string(
            root.join("backend/src/praxis/specs/personas.rs"),
        )
        .unwrap();
        assert!(written.contains("pub const PERSONA_ADMIN"));
        assert!(written.contains("pub const PERSONA_MEMBER"));
        assert!(written.contains("CONV_SESSION_42"));
        assert!(written.contains("CONV_SESSION_43"));
    }

    #[test]
    fn record_spec_fact_rejects_malformed_persona_data() {
        let root = scratch_project_root();
        let err = record_spec_fact(
            &root,
            "session-1",
            RecordSpecFactParams {
                fact_type: "persona".into(),
                data: serde_json::json!({ "name": "missing id and description" }),
            },
        )
        .unwrap_err();
        // serde_json::Error wrapped via AgentError::Serde (#[from]).
        assert!(matches!(err, AgentError::Serde(_)));
    }

    #[test]
    fn record_spec_fact_permission_writes_file_and_registers_module() {
        let root = scratch_project_root();
        let result = record_spec_fact(
            &root,
            "session-42",
            RecordSpecFactParams {
                fact_type: "permission".into(),
                data: serde_json::json!({
                    "id": "create_task",
                    "description": "Create a new task",
                }),
            },
        )
        .expect("record_spec_fact should succeed");

        assert_eq!(result.file_path, "backend/src/praxis/specs/permissions.rs");
        assert_eq!(result.identifier, "create_task");

        let written = std::fs::read_to_string(root.join(&result.file_path)).unwrap();
        assert!(written.contains("pub const PERM_CREATE_TASK: PermissionId = PermissionId(\"create_task\");"));

        let specs_mod =
            std::fs::read_to_string(root.join("backend/src/praxis/specs/mod.rs")).unwrap();
        assert!(specs_mod.contains("pub mod permissions;"));
    }

    #[test]
    fn record_spec_fact_role_writes_file() {
        let root = scratch_project_root();
        let result = record_spec_fact(
            &root,
            "session-42",
            RecordSpecFactParams {
                fact_type: "role".into(),
                data: serde_json::json!({
                    "id": "admin",
                    "description": "Team admin",
                    "permission_ids": ["create_task", "assign_task"],
                    "persona_ids": ["admin"],
                    "semantics": "inherits",
                    "parent_role_id": "member",
                }),
            },
        )
        .expect("record_spec_fact should succeed");

        assert_eq!(result.file_path, "backend/src/praxis/specs/roles.rs");
        assert_eq!(result.identifier, "admin");

        let written = std::fs::read_to_string(root.join(&result.file_path)).unwrap();
        assert!(written.contains("pub const ROLE_ADMIN: RoleId = RoleId(\"admin\");"));
        assert!(written.contains("RoleSemantics::Inherits { parent: RoleId(\"member\") }"));
    }

    #[test]
    fn record_spec_fact_role_rejects_inherits_without_parent() {
        let root = scratch_project_root();
        let err = record_spec_fact(
            &root,
            "session-1",
            RecordSpecFactParams {
                fact_type: "role".into(),
                data: serde_json::json!({
                    "id": "admin",
                    "description": "Team admin",
                    "semantics": "inherits",
                }),
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("no parent_role_id"));
    }

    #[test]
    fn record_spec_fact_state_writes_file() {
        let root = scratch_project_root();
        let result = record_spec_fact(
            &root,
            "session-42",
            RecordSpecFactParams {
                fact_type: "state".into(),
                data: serde_json::json!({
                    "id": "todo",
                    "description": "Task created, not yet started",
                    "transitions": [
                        {
                            "to": "in_progress",
                            "role_ids": ["member", "admin"],
                            "condition": "only_if_assigned_to_self",
                        },
                        {
                            "to": "cancelled",
                            "role_ids": ["admin"],
                            "condition": "always",
                        },
                    ],
                }),
            },
        )
        .expect("record_spec_fact should succeed");

        assert_eq!(result.file_path, "backend/src/praxis/specs/states.rs");
        assert_eq!(result.identifier, "todo");

        let written = std::fs::read_to_string(root.join(&result.file_path)).unwrap();
        assert!(written.contains("pub const STATE_TODO: StateId = StateId(\"todo\");"));
        assert!(written.contains("Transitions::To(AtLeastOne {"));
        assert!(written.contains("TransitionCondition::OnlyIfAssignedToSelf"));
    }

    #[test]
    fn record_spec_fact_state_rejects_transition_without_roles() {
        let root = scratch_project_root();
        let err = record_spec_fact(
            &root,
            "session-1",
            RecordSpecFactParams {
                fact_type: "state".into(),
                data: serde_json::json!({
                    "id": "todo",
                    "description": "desc",
                    "transitions": [{ "to": "done", "role_ids": [], "condition": "always" }],
                }),
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("no role_ids"));
    }

    #[test]
    fn record_spec_fact_entity_writes_file() {
        let root = scratch_project_root();
        let result = record_spec_fact(
            &root,
            "session-42",
            RecordSpecFactParams {
                fact_type: "entity".into(),
                data: serde_json::json!({
                    "id": "task",
                    "description": "A unit of work that can be assigned and tracked",
                    "fields": [
                        { "name": "title", "description": "Short name" },
                        { "name": "status", "description": "Current lifecycle state" },
                    ],
                    "state_ids": ["todo", "in_progress", "done", "cancelled"],
                }),
            },
        )
        .expect("record_spec_fact should succeed");

        assert_eq!(result.file_path, "backend/src/praxis/specs/entities.rs");
        assert_eq!(result.identifier, "task");

        let written = std::fs::read_to_string(root.join(&result.file_path)).unwrap();
        assert!(written.contains("pub const ENTITY_TASK: EntityId = EntityId(\"task\");"));
        assert!(written.contains("Unresolved::Pending {"));
        assert!(written.contains("StateId(\"todo\")"));
    }
}
