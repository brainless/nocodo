use std::path::PathBuf;

use llm_sdk::{
    client::LlmClient,
    llama_cpp::LlamaCppClient,
    types::{CompletionRequest, ContentBlock, Message, Role},
};

use super::modes::{diesel_model, diesel_model_struct, diesel_schema, praxis_auth, state_transition};
use crate::{
    code_extractor::{extract_struct, find_dependent_types, find_struct_file, list_impl_fns},
    error::AgentError,
    spec_gap::{find_gaps_from_persona_notes, find_pending_gaps_in_code, SpecGap},
    storage::spec_schemas::PersonaNote,
};

// ---------------------------------------------------------------------------
// Public result types
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct DieselModelFnOutput {
    pub prompt: String,
    pub raw_response: String,
    /// Code extracted from the response (think-stripped, fence-unwrapped).
    pub code: Option<String>,
}

#[derive(Debug)]
pub struct DieselModelStructOutput {
    pub system_prompt: String,
    pub prompt: String,
    pub raw_response: String,
    /// Code extracted from the response (think-stripped, fence-unwrapped).
    pub code: Option<String>,
    /// Relative file path if written to disk (None when `apply` is false).
    pub file_path: Option<String>,
}

#[derive(Debug)]
pub struct DieselSchemaOutput {
    pub system_prompt: String,
    pub prompt: String,
    pub raw_response: String,
    /// Code extracted from the response (think-stripped, fence-unwrapped).
    pub code: Option<String>,
    /// Relative file path if written to disk (None when `apply` is false).
    pub file_path: Option<String>,
}

#[derive(Debug)]
pub struct PraxisAuthOutput {
    pub system_prompt: String,
    pub prompt: String,
    pub raw_response: String,
    /// Extracted Rust code (think-stripped, fence-unwrapped).
    pub code: Option<String>,
    /// Relative paths of files written to disk (empty when `apply` is false).
    pub files_written: Vec<String>,
    /// Gaps detected in the input notes or generated code.
    pub gaps: Vec<SpecGap>,
}

#[derive(Debug)]
pub struct StateTransitionOutput {
    pub system_prompt: String,
    pub prompt: String,
    pub raw_response: String,
    /// Extracted Rust code (think-stripped, fence-unwrapped, imports prepended).
    pub code: Option<String>,
    /// Deterministic metadata parsed out of `code` (§6.3 point 3/7). Default
    /// (all-`None`/empty) when `code` is `None`.
    pub metadata: state_transition::TransitionMetadata,
    /// Relative path of the file written to disk (`None` when `apply` is
    /// `false`, or when `code` was empty).
    pub file_path: Option<String>,
}

#[derive(Debug)]
pub enum RustEngineerResult {
    Code(String),
    Empty,
}

// ---------------------------------------------------------------------------
// Agent
// ---------------------------------------------------------------------------

pub struct RustEngineerAgent {
    client: LlamaCppClient,
    model: String,
    project_path: PathBuf,
}

impl RustEngineerAgent {
    pub fn new(
        model: impl Into<String>,
        base_url: Option<String>,
        project_path: impl Into<PathBuf>,
    ) -> Result<Self, AgentError> {
        let client = LlamaCppClient::new().map_err(|e| AgentError::Config(e.to_string()))?;
        let client = match base_url {
            Some(url) => client.with_base_url(url),
            None => client,
        };
        Ok(Self {
            client,
            model: model.into(),
            project_path: project_path.into(),
        })
    }

    // -----------------------------------------------------------------------
    // Mode: Diesel model impl function
    // -----------------------------------------------------------------------

    /// Generate a new Diesel model impl function by example.
    ///
    /// Finds `struct_name` in `project_path`, extracts its definition and
    /// existing impl functions as examples, then asks the model to write
    /// `fn_name` in the same style. Returns the full prompt, raw response,
    /// and extracted code for display/debugging.
    pub async fn diesel_model_fn(
        &self,
        struct_name: &str,
        fn_name: &str,
    ) -> Result<DieselModelFnOutput, AgentError> {
        let struct_file = find_struct_file(&self.project_path, struct_name)
            .map_err(AgentError::Other)?
            .ok_or_else(|| {
                AgentError::Other(format!("struct `{}` not found in project", struct_name))
            })?;

        let struct_block = extract_struct(&struct_file, struct_name)
            .map_err(AgentError::Other)?
            .ok_or_else(|| {
                AgentError::Other(format!("could not extract struct `{}`", struct_name))
            })?;

        let examples = list_impl_fns(&struct_file, struct_name).map_err(AgentError::Other)?;

        let dependent_types =
            find_dependent_types(&self.project_path, &struct_file, &struct_block.source)
                .map_err(AgentError::Other)?;

        log::info!(
            "[RustEngineer:diesel_model] struct={} fn={} examples={} dependent_types={}",
            struct_name,
            fn_name,
            examples.len(),
            dependent_types.len()
        );

        let (prompt, table_name) =
            diesel_model::build_prompt(&struct_block.source, &examples, &dependent_types, fn_name);

        let request = CompletionRequest {
            messages: vec![Message {
                role: Role::User,
                content: vec![ContentBlock::Text {
                    text: prompt.clone(),
                }],
                tool_call_id: None,
                tool_name: None,
            }],
            max_tokens: 512,
            model: self.model.clone(),
            system: None,
            temperature: Some(0.2),
            top_p: None,
            stop_sequences: None,
            tools: None,
            tool_choice: None,
            response_format: None,
        };

        let response = self
            .client
            .complete(request)
            .await
            .map_err(AgentError::Llm)?;

        let raw_response = response
            .content
            .iter()
            .filter_map(|b| match b {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");

        log::info!("[RustEngineer:diesel_model] raw_len={}", raw_response.len());

        let code = prepend_imports(&extract_code(&raw_response), &table_name);
        Ok(DieselModelFnOutput {
            prompt,
            raw_response,
            code: if code.trim().is_empty() {
                None
            } else {
                Some(code)
            },
        })
    }

    // -----------------------------------------------------------------------
    // Mode: Diesel model struct
    // -----------------------------------------------------------------------

    /// Generate or update a single Diesel model struct.
    ///
    /// The caller supplies the task prompt, including the current struct when
    /// updating. This keeps the stable system prompt compact for tiny models.
    pub async fn diesel_model_struct(
        &self,
        user_prompt: &str,
    ) -> Result<DieselModelStructOutput, AgentError> {
        let system_prompt = diesel_model_struct::build_system_prompt();
        let prompt = user_prompt.trim().to_string();

        log::info!(
            "[RustEngineer:diesel_model_struct] prompt_len={} system_len={}",
            prompt.len(),
            system_prompt.len()
        );

        let request = CompletionRequest {
            messages: vec![Message {
                role: Role::User,
                content: vec![ContentBlock::Text {
                    text: prompt.clone(),
                }],
                tool_call_id: None,
                tool_name: None,
            }],
            max_tokens: 768,
            model: self.model.clone(),
            system: Some(system_prompt.clone()),
            temperature: Some(0.2),
            top_p: None,
            stop_sequences: None,
            tools: None,
            tool_choice: None,
            response_format: None,
        };

        let response = self
            .client
            .complete(request)
            .await
            .map_err(AgentError::Llm)?;

        let raw_response = response
            .content
            .iter()
            .filter_map(|b| match b {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");

        log::info!(
            "[RustEngineer:diesel_model_struct] raw_len={}",
            raw_response.len()
        );

        let code = strip_imports(&extract_code(&raw_response));
        Ok(DieselModelStructOutput {
            system_prompt,
            prompt,
            raw_response,
            code: if code.trim().is_empty() {
                None
            } else {
                Some(code)
            },
            file_path: None,
        })
    }

    /// Like `diesel_model_struct` but also writes the generated code to disk
    /// via `code_writer::write_diesel_model`.
    pub async fn diesel_model_struct_write(
        &self,
        user_prompt: &str,
    ) -> Result<DieselModelStructOutput, AgentError> {
        let mut output = self.diesel_model_struct(user_prompt).await?;
        if let Some(ref code) = output.code {
            let file_path = crate::code_writer::write_diesel_model(&self.project_path, code)?;
            output.file_path = Some(file_path);
        }
        Ok(output)
    }

    // -----------------------------------------------------------------------
    // Mode: Diesel schema table definition
    // -----------------------------------------------------------------------

    /// Generate or update a single Diesel `table!` schema definition.
    ///
    /// The caller supplies the task prompt, including the current table block
    /// when updating. The model returns exactly one `diesel::table!` block.
    pub async fn diesel_schema(&self, user_prompt: &str) -> Result<DieselSchemaOutput, AgentError> {
        let system_prompt = diesel_schema::build_system_prompt();
        let prompt = user_prompt.trim().to_string();

        log::info!(
            "[RustEngineer:diesel_schema] prompt_len={} system_len={}",
            prompt.len(),
            system_prompt.len()
        );

        let request = CompletionRequest {
            messages: vec![Message {
                role: Role::User,
                content: vec![ContentBlock::Text {
                    text: prompt.clone(),
                }],
                tool_call_id: None,
                tool_name: None,
            }],
            max_tokens: 768,
            model: self.model.clone(),
            system: Some(system_prompt.clone()),
            temperature: Some(0.2),
            top_p: None,
            stop_sequences: None,
            tools: None,
            tool_choice: None,
            response_format: None,
        };

        let response = self
            .client
            .complete(request)
            .await
            .map_err(AgentError::Llm)?;

        let raw_response = response
            .content
            .iter()
            .filter_map(|b| match b {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");

        log::info!(
            "[RustEngineer:diesel_schema] raw_len={}",
            raw_response.len()
        );

        let code = strip_imports(&extract_code(&raw_response));
        Ok(DieselSchemaOutput {
            system_prompt,
            prompt,
            raw_response,
            code: if code.trim().is_empty() {
                None
            } else {
                Some(code)
            },
            file_path: None,
        })
    }

    /// Like `diesel_schema` but also writes the generated code to disk
    /// via `code_writer::write_diesel_schema`.
    pub async fn diesel_schema_write(
        &self,
        user_prompt: &str,
    ) -> Result<DieselSchemaOutput, AgentError> {
        let mut output = self.diesel_schema(user_prompt).await?;
        if let Some(ref code) = output.code {
            let file_path = crate::code_writer::write_diesel_schema(&self.project_path, code)?;
            output.file_path = Some(file_path);
        }
        Ok(output)
    }

    // -----------------------------------------------------------------------
    // Mode: Praxis auth spec (personas)
    // -----------------------------------------------------------------------

    /// Generate `nocodo_praxis` persona static definitions from structured `PersonaNote` data.
    ///
    /// When `apply` is true, writes the result to
    /// `backend/src/praxis/specs/personas.rs` and registers the module.
    /// Returns the prompt, raw response, extracted code, and written file paths
    /// for admin UI display and debugging.
    pub async fn run_praxis_auth(
        &self,
        personas: &[PersonaNote],
        apply: bool,
    ) -> Result<PraxisAuthOutput, AgentError> {
        let system_prompt = praxis_auth::build_system_prompt(self.project_path.parent());
        let prompt = praxis_auth::build_user_prompt(personas);

        log::info!(
            "[RustEngineer:praxis_auth] personas={} apply={} system_len={} prompt_len={}",
            personas.len(),
            apply,
            system_prompt.len(),
            prompt.len()
        );

        let request = CompletionRequest {
            messages: vec![Message {
                role: Role::User,
                content: vec![ContentBlock::Text {
                    text: prompt.clone(),
                }],
                tool_call_id: None,
                tool_name: None,
            }],
            max_tokens: 1024,
            model: self.model.clone(),
            system: Some(system_prompt.clone()),
            temperature: Some(0.2),
            top_p: None,
            stop_sequences: None,
            tools: None,
            tool_choice: None,
            response_format: None,
        };

        let response = self
            .client
            .complete(request)
            .await
            .map_err(AgentError::Llm)?;

        let raw_response = response
            .content
            .iter()
            .filter_map(|b| match b {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");

        log::info!("[RustEngineer:praxis_auth] raw_len={}", raw_response.len());

        let extracted = extract_code(&raw_response);
        let code = if extracted.trim().is_empty() {
            None
        } else {
            Some(prepend_praxis_auth_imports(&extracted))
        };

        let mut files_written = Vec::new();
        if apply {
            if let Some(ref final_code) = code {
                match crate::code_writer::write_praxis_spec(
                    &self.project_path,
                    "personas",
                    final_code,
                ) {
                    Ok(path) => {
                        log::info!("[RustEngineer:praxis_auth] wrote {}", path);
                        files_written.push(path);
                    }
                    Err(e) => log::warn!("[RustEngineer:praxis_auth] write error: {}", e),
                }
            }
        }

        // Scan for gaps: pre-generation (PersonaNote incomplete_reason) and
        // post-generation (Unresolved::Pending in generated code).
        let mut gaps = find_gaps_from_persona_notes(personas);
        if let Some(ref final_code) = code {
            let code_gaps = find_pending_gaps_in_code(final_code);
            gaps.extend(code_gaps);
        }
        if !gaps.is_empty() {
            log::info!(
                "[RustEngineer:praxis_auth] detected {} gaps: {:?}",
                gaps.len(),
                gaps.iter()
                    .map(|g| format!("{}.{}", g.artifact_id, g.field))
                    .collect::<Vec<_>>()
            );
        }

        Ok(PraxisAuthOutput {
            system_prompt,
            prompt,
            raw_response,
            code,
            files_written,
            gaps,
        })
    }

    // -----------------------------------------------------------------------
    // Mode: State transition guard (Praxis v1 §6 / §10 Phase 6)
    // -----------------------------------------------------------------------

    /// Generate one `TransitionGuard` implementation from a natural-language
    /// action description.
    ///
    /// # Model type dependency ordering — decision (MILESTONE_PRAXIS_v1.md
    /// §6.4, §10 Phase 6, §11 "Model struct ordering enforcement")
    ///
    /// The milestone poses three options for enforcing "a Diesel model
    /// struct for the referenced entity must exist before its transition
    /// guard is generated": (a) a hard check in this backend
    /// handler/method, (b) a static-checker-only check (C2/C3, deferred to
    /// after codegen), or (c) a PO-prompt instruction only.
    ///
    /// **Chosen approach: (c) + a soft, non-blocking diagnostic, not (a).**
    /// A hard backend check would require inferring *which* model struct the
    /// action description needs (e.g. "assign a task" -> `Task`) from free
    /// text without an LLM call — exactly the kind of guess this method's
    /// prompt already asks the LLM to avoid making. Implementing that
    /// inference reliably is a bigger, separate piece of work (fuzzy-matching
    /// action nouns to struct names) and a wrong guess would block valid
    /// generations. Per §6.4's "No placeholder types" principle, the loop
    /// already relies on the model referencing real ids and the checker
    /// catching gaps — extending that same posture to model structs is
    /// consistent, whereas a backend existence check is the odd one out.
    ///
    /// So: PO's loop prompt (`agents/src/product_owner/modes/loop_mode.rs`)
    /// is expected to instruct the LLM to generate the model struct via
    /// `diesel_model_struct` before triggering a transition guard that
    /// depends on it (this is a prompt-level instruction, not enforced here).
    /// As a cheap, *non-blocking* diagnostic, this method logs a warning
    /// when `backend/src/models/` doesn't exist or is empty at call time —
    /// useful operator signal, never a hard error. Full enforcement
    /// (referenced model type must resolve to a real struct) is left as a
    /// documented gap for Phase 7's end-to-end test / a future static
    /// checker extension (the current C2 check only covers
    /// `RoleId`/`StateId`/`EntityId`/`PermissionId`/`PersonaId` references,
    /// not Diesel model struct names).
    ///
    /// When `apply` is true, writes the result to
    /// `backend/src/praxis/transitions/{name}.rs` (name derived from the
    /// generated guard struct's name) and registers the module. Returns the
    /// prompt, raw response, extracted code, parsed metadata, and the
    /// written file path for admin UI display and debugging — same
    /// transparency contract as every other RustEngineer mode.
    pub async fn run_state_transition(
        &self,
        action_description: &str,
        apply: bool,
    ) -> Result<StateTransitionOutput, AgentError> {
        let models_dir = self.project_path.join("backend/src/models");
        let has_models = models_dir
            .read_dir()
            .map(|mut entries| entries.next().is_some())
            .unwrap_or(false);
        if !has_models {
            log::warn!(
                "[RustEngineer:state_transition] backend/src/models/ is empty or missing — \
                 generating a transition guard with no project model types in context. This is \
                 not blocked (see run_state_transition's doc comment on model-type ordering); \
                 the model may reference model type names that don't exist yet."
            );
        }

        let system_prompt = state_transition::build_system_prompt(self.project_path.parent());
        let model_types = read_concatenated_rs_dir(&models_dir);
        let existing_spec =
            read_concatenated_rs_dir(&self.project_path.join("backend/src/praxis/specs"));
        let prompt =
            state_transition::build_user_prompt(action_description, &model_types, &existing_spec);

        log::info!(
            "[RustEngineer:state_transition] apply={} system_len={} prompt_len={}",
            apply,
            system_prompt.len(),
            prompt.len()
        );

        let request = CompletionRequest {
            messages: vec![Message {
                role: Role::User,
                content: vec![ContentBlock::Text {
                    text: prompt.clone(),
                }],
                tool_call_id: None,
                tool_name: None,
            }],
            max_tokens: 1024,
            model: self.model.clone(),
            system: Some(system_prompt.clone()),
            temperature: Some(0.2),
            top_p: None,
            stop_sequences: None,
            tools: None,
            tool_choice: None,
            response_format: None,
        };

        let response = self
            .client
            .complete(request)
            .await
            .map_err(AgentError::Llm)?;

        let raw_response = response
            .content
            .iter()
            .filter_map(|b| match b {
                ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");

        log::info!(
            "[RustEngineer:state_transition] raw_len={}",
            raw_response.len()
        );

        let extracted = extract_code(&raw_response);
        let code = if extracted.trim().is_empty() {
            None
        } else {
            Some(prepend_state_transition_imports(&extracted))
        };

        let metadata = code
            .as_deref()
            .map(state_transition::extract_transition_metadata)
            .unwrap_or_default();

        let mut file_path = None;
        if apply {
            if let Some(ref final_code) = code {
                let name = transition_file_name(&metadata);
                match crate::code_writer::write_transition(&self.project_path, &name, final_code) {
                    Ok(path) => {
                        log::info!("[RustEngineer:state_transition] wrote {}", path);
                        file_path = Some(path);
                    }
                    Err(e) => log::warn!("[RustEngineer:state_transition] write error: {}", e),
                }
            }
        }

        Ok(StateTransitionOutput {
            system_prompt,
            prompt,
            raw_response,
            code,
            metadata,
            file_path,
        })
    }
}

// ---------------------------------------------------------------------------
// Response post-processing
// ---------------------------------------------------------------------------

/// Strip `<think>…</think>` reasoning block and unwrap code fences.
fn extract_code(text: &str) -> String {
    let text = if let Some(end) = text.find("</think>") {
        text[end + "</think>".len()..].trim_start()
    } else {
        text
    };

    if let Some(start) = text.find("```rust") {
        let after = &text[start + "```rust".len()..];
        if let Some(end) = after.find("```") {
            return after[..end].trim().to_string();
        }
    }
    if let Some(start) = text.find("```") {
        let after = &text[start + "```".len()..];
        if let Some(end) = after.find("```") {
            return after[..end].trim().to_string();
        }
    }

    text.trim().to_string()
}

/// Strip any `use` lines from the model output and prepend deterministic imports.
fn prepend_imports(code: &str, table_name: &Option<String>) -> String {
    let body = strip_imports(code);

    match table_name {
        Some(table) => format!("use diesel::prelude::*;\nuse crate::schema::{table};\n\n{body}"),
        None => body,
    }
}

/// Prepend deterministic imports for praxis auth spec files and strip any model-added `use` lines.
fn prepend_praxis_auth_imports(code: &str) -> String {
    let body = strip_imports(code);
    format!(
        "use nocodo_praxis::auth::{{PersonaId, UserPersona}};\n\
         use nocodo_praxis::primitives::AtLeastOne;\n\
         use nocodo_praxis::provenance::Provenance;\n\
         \n{body}"
    )
}

/// Prepend deterministic imports for a `state_transition` guard file and
/// strip any model-added `use` lines. Imports the trait/error vocabulary
/// (`nocodo_praxis::statemachine`) plus the auth vocabulary
/// (`nocodo_praxis::auth`) and `nocodo_praxis::entity::EntityId` — the same
/// set the system prompt documents as "types you must use".
fn prepend_state_transition_imports(code: &str) -> String {
    let body = strip_imports(code);
    format!(
        "use nocodo_praxis::auth::{{HasRole, RoleId}};\n\
         use nocodo_praxis::entity::EntityId;\n\
         use nocodo_praxis::statemachine::{{HasState, StateId, TransitionError, TransitionGuard}};\n\
         \n{body}"
    )
}

/// Read and concatenate every `.rs` file directly under `dir`, sorted by
/// path for determinism. Returns an empty string if `dir` doesn't exist —
/// not an error (mirrors `static_checker::load_transition_guard_code`).
fn read_concatenated_rs_dir(dir: &std::path::Path) -> String {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return String::new();
    };
    let mut paths: Vec<_> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    paths.sort();
    let mut out = String::new();
    for path in paths {
        if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            if let Ok(content) = std::fs::read_to_string(&path) {
                out.push_str(&content);
                out.push('\n');
            }
        }
    }
    out
}

/// Derive the `transitions/{name}.rs` file stem from parsed metadata: the
/// guard struct's name, snake_cased (e.g. `AssignTask` -> `assign_task`).
/// Falls back to `entity_from_to` when the struct name wasn't parsed (should
/// not happen for well-formed output, but keeps `write_transition` from
/// failing on a malformed generation), then to a fixed fallback name.
fn transition_file_name(metadata: &state_transition::TransitionMetadata) -> String {
    if let Some(ref name) = metadata.struct_name {
        return schema_codegen::to_snake_case(name);
    }
    match (&metadata.entity, &metadata.from_state, &metadata.to_state) {
        (Some(e), Some(f), Some(t)) => format!("{e}_{f}_to_{t}"),
        _ => "unnamed_transition".to_string(),
    }
}

fn strip_imports(code: &str) -> String {
    code.lines()
        .filter(|line| !line.trim_start().starts_with("use "))
        .collect::<Vec<_>>()
        .join("\n")
}
