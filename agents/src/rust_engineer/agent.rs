use std::path::PathBuf;

use llm_sdk::{
    client::LlmClient,
    llama_cpp::LlamaCppClient,
    types::{CompletionRequest, ContentBlock, Message, Role},
};

use super::modes::{diesel_model, diesel_model_struct, diesel_schema};
use crate::{
    code_extractor::{extract_struct, find_dependent_types, find_struct_file, list_impl_fns},
    error::AgentError,
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
}

#[derive(Debug)]
pub struct DieselSchemaOutput {
    pub system_prompt: String,
    pub prompt: String,
    pub raw_response: String,
    /// Code extracted from the response (think-stripped, fence-unwrapped).
    pub code: Option<String>,
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
        })
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

fn strip_imports(code: &str) -> String {
    code.lines()
        .filter(|line| !line.trim_start().starts_with("use "))
        .collect::<Vec<_>>()
        .join("\n")
}
