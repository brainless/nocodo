use std::path::PathBuf;
use std::sync::Arc;

use llm_sdk::{
    client::LlmClient,
    tools::{Tool, ToolChoice},
    types::{CompletionRequest, Message, Role},
};

use super::prompts::ContextType;
use super::tools::{ListFilesParams, ReadFileParams, UpdateTaskStatusParams};
use crate::{
    error::AgentError,
    storage::{AgentStorage, AgentType, ChatMessage, ContextStorage, TaskStatus, TaskStorage},
};

const MAX_NUDGES: u32 = 8;

#[derive(Debug)]
pub enum ContextAgentResponse {
    ContextSaved { context: String },
    Stopped(String),
}

pub struct ContextAgent {
    llm_client: Arc<dyn LlmClient>,
    storage: Arc<dyn AgentStorage>,
    context_storage: Arc<dyn ContextStorage>,
    task_storage: Arc<dyn TaskStorage>,
    model: String,
    project_id: i64,
    project_path: PathBuf,
    context_type: ContextType,
}

impl ContextAgent {
    pub fn new(
        llm_client: Arc<dyn LlmClient>,
        storage: Arc<dyn AgentStorage>,
        context_storage: Arc<dyn ContextStorage>,
        task_storage: Arc<dyn TaskStorage>,
        model: impl Into<String>,
        project_id: i64,
        project_path: impl Into<PathBuf>,
        context_type: ContextType,
    ) -> Self {
        Self {
            llm_client,
            storage,
            context_storage,
            task_storage,
            model: model.into(),
            project_id,
            project_path: project_path.into(),
            context_type,
        }
    }

    pub async fn run_for_task(
        &self,
        session_id: i64,
        task_id: i64,
    ) -> Result<ContextAgentResponse, AgentError> {
        self.run_loop(session_id, task_id).await
    }

    async fn run_loop(
        &self,
        session_id: i64,
        task_id: i64,
    ) -> Result<ContextAgentResponse, AgentError> {
        let list_files_tool = Tool::from_type::<ListFilesParams>()
            .name("list_files")
            .description(
                "List files and directories at the given path relative to the project root. \
                 Pass an empty string for the project root, or a subdirectory path like 'backend/src'. \
                 Returns a listing of files and directories.",
            )
            .build();

        let read_file_tool = Tool::from_type::<ReadFileParams>()
            .name("read_file")
            .description(
                "Read the contents of a file at the given path relative to the project root. \
                 Use this to examine source files, Cargo.toml, config files, migrations, etc.",
            )
            .build();

        let update_task_status_tool = Tool::from_type::<UpdateTaskStatusParams>()
            .name("update_task_status")
            .description(
                "Update the status of the current task. Use \"in_progress\" when starting, \
                 \"done\" when the context has been saved.",
            )
            .build();

        let agent_type_str = match self.context_type {
            ContextType::Backend => AgentType::BackendContext.as_str().to_string(),
            ContextType::AdminGui => AgentType::AdminGuiContext.as_str().to_string(),
        };

        let text_row = |session_id: i64, content: String| ChatMessage {
            id: None,
            session_id,
            role: "assistant".to_string(),
            agent_type: Some(agent_type_str.clone()),
            content,
            tool_call_id: None,
            tool_name: None,
            turn_id: None,
            created_at: 0,
        };

        let system_prompt = match self.context_type {
            ContextType::Backend => super::prompts::backend_system_prompt(),
            ContextType::AdminGui => super::prompts::admin_gui_system_prompt(),
        };

        let mut nudges: u32 = 0;

        loop {
            let history = self.storage.get_messages(session_id).await?;
            let llm_messages: Vec<Message> = history
                .into_iter()
                .map(|m| {
                    let role = match m.role.as_str() {
                        "assistant" => Role::Assistant,
                        "tool" => Role::Tool,
                        _ => Role::User,
                    };
                    Message {
                        role,
                        content: vec![llm_sdk::types::ContentBlock::Text { text: m.content }],
                        tool_call_id: m.tool_call_id,
                        tool_name: m.tool_name,
                    }
                })
                .collect();

            let request = CompletionRequest {
                messages: llm_messages,
                max_tokens: 8192,
                model: self.model.clone(),
                system: Some(system_prompt.clone()),
                temperature: Some(0.2),
                top_p: None,
                stop_sequences: None,
                tools: Some(vec![
                    list_files_tool.clone(),
                    read_file_tool.clone(),
                    update_task_status_tool.clone(),
                ]),
                tool_choice: Some(ToolChoice::Auto),
                response_format: None,
            };

            let response = match self.llm_client.complete(request).await {
                Ok(resp) => resp,
                Err(llm_err) => {
                    // If the LLM returns a tool-call validation error, try again with
                    // a simpler prompt that asks the model to output context as plain text.
                    let err_str = llm_err.to_string();
                    if err_str.contains("validation failed") || err_str.contains("missing properties") {
                        log::warn!(
                            "[context_agent] LLM tool validation error (attempt {}): {}. Falling back to text output.",
                            nudges, err_str
                        );
                        // Send a nudge asking for plain text output
                        self.storage
                            .create_message(ChatMessage {
                                id: None,
                                session_id,
                                role: "user".to_string(),
                                agent_type: None,
                                content: "The write_context tool had a validation error. Instead of using write_context, please output your complete JSON context summary directly as plain text. Do not use any tools — just respond with the JSON object.".to_string(),
                                tool_call_id: None,
                                tool_name: None,
                                turn_id: None,
                                created_at: 0,
                            })
                            .await?;
                        nudges += 1;
                        if nudges >= MAX_NUDGES {
                            return Err(AgentError::Llm(llm_err));
                        }
                        continue;
                    }
                    return Err(AgentError::Llm(llm_err));
                }
            };

            let assistant_text = response
                .content
                .iter()
                .filter_map(|b| match b {
                    llm_sdk::types::ContentBlock::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("");

            if let Some(tool_calls) = response.tool_calls {
                for tool_call in tool_calls {
                    let tool_name = tool_call.name();
                    let call_id = tool_call.id().to_string();

                    match tool_name {
                        "list_files" | "repo_browser.list_files" => {
                            let params: ListFilesParams =
                                tool_call.parse_arguments().map_err(AgentError::Llm)?;
                            let result = self.execute_list_files(&params.path);
                            let mut turn = Vec::new();
                            if !assistant_text.is_empty() {
                                turn.push(text_row(session_id, assistant_text.clone()));
                            }
                            turn.push(ChatMessage {
                                id: None,
                                session_id,
                                role: "assistant".to_string(),
                                agent_type: Some(agent_type_str.clone()),
                                content: serde_json::to_string(tool_call.arguments())?,
                                tool_call_id: Some(call_id.clone()),
                                tool_name: Some("list_files".to_string()),
                                turn_id: None,
                                created_at: 0,
                            });
                            turn.push(ChatMessage {
                                id: None,
                                session_id,
                                role: "tool".to_string(),
                                agent_type: None,
                                content: result,
                                tool_call_id: Some(call_id),
                                tool_name: Some("list_files".to_string()),
                                turn_id: None,
                                created_at: 0,
                            });
                            self.storage.create_turn(turn).await?;
                        }

                        "read_file" | "repo_browser.read_file" => {
                            let params: ReadFileParams =
                                tool_call.parse_arguments().map_err(AgentError::Llm)?;
                            let result = self.execute_read_file(&params.path);
                            let mut turn = Vec::new();
                            if !assistant_text.is_empty() {
                                turn.push(text_row(session_id, assistant_text.clone()));
                            }
                            turn.push(ChatMessage {
                                id: None,
                                session_id,
                                role: "assistant".to_string(),
                                agent_type: Some(agent_type_str.clone()),
                                content: serde_json::to_string(tool_call.arguments())?,
                                tool_call_id: Some(call_id.clone()),
                                tool_name: Some("read_file".to_string()),
                                turn_id: None,
                                created_at: 0,
                            });
                            turn.push(ChatMessage {
                                id: None,
                                session_id,
                                role: "tool".to_string(),
                                agent_type: None,
                                content: result,
                                tool_call_id: Some(call_id),
                                tool_name: Some("read_file".to_string()),
                                turn_id: None,
                                created_at: 0,
                            });
                            self.storage.create_turn(turn).await?;
                        }

                        "update_task_status" | "task.update_task_status" => {
                            let params: UpdateTaskStatusParams =
                                tool_call.parse_arguments().map_err(AgentError::Llm)?;
                            let new_status = TaskStatus::from_str(&params.status);
                            let result_text = match self
                                .task_storage
                                .update_task_status(task_id, new_status)
                                .await
                            {
                                Ok(()) => format!("Task status updated to {}.", params.status),
                                Err(e) => format!("Failed to update task status: {}", e),
                            };

                            let mut turn = Vec::new();
                            if !assistant_text.is_empty() {
                                turn.push(text_row(session_id, assistant_text.clone()));
                            }
                            turn.push(ChatMessage {
                                id: None,
                                session_id,
                                role: "assistant".to_string(),
                                agent_type: Some(agent_type_str.clone()),
                                content: serde_json::to_string(tool_call.arguments())?,
                                tool_call_id: Some(call_id.clone()),
                                tool_name: Some("update_task_status".to_string()),
                                turn_id: None,
                                created_at: 0,
                            });
                            turn.push(ChatMessage {
                                id: None,
                                session_id,
                                role: "tool".to_string(),
                                agent_type: None,
                                content: result_text,
                                tool_call_id: Some(call_id),
                                tool_name: Some("update_task_status".to_string()),
                                turn_id: None,
                                created_at: 0,
                            });
                            self.storage.create_turn(turn).await?;
                        }

                        "write_context" | "context.write_context" => {
                            // Fallback: parse the context from whatever the model sends
                            let raw = tool_call.arguments();
                            let context_str = match raw.get("context") {
                                Some(v) => {
                                    // context field exists — use it
                                    if v.is_string() {
                                        v.as_str().unwrap_or_default().to_string()
                                    } else {
                                        v.to_string()
                                    }
                                }
                                None => {
                                    // The model sent arguments without the "context" key.
                                    // Use the entire arguments as the context.
                                    raw.to_string()
                                }
                            };

                            self.context_storage
                                .save_context(
                                    self.project_id,
                                    self.context_type.as_str(),
                                    &context_str,
                                )
                                .await?;

                            let result_text = format!(
                                "Context saved for project {}, type {}.",
                                self.project_id,
                                self.context_type.as_str()
                            );

                            let mut turn = Vec::new();
                            if !assistant_text.is_empty() {
                                turn.push(text_row(session_id, assistant_text.clone()));
                            }
                            turn.push(ChatMessage {
                                id: None,
                                session_id,
                                role: "assistant".to_string(),
                                agent_type: Some(agent_type_str.clone()),
                                content: serde_json::to_string(raw)?,
                                tool_call_id: Some(call_id.clone()),
                                tool_name: Some("write_context".to_string()),
                                turn_id: None,
                                created_at: 0,
                            });
                            turn.push(ChatMessage {
                                id: None,
                                session_id,
                                role: "tool".to_string(),
                                agent_type: None,
                                content: result_text,
                                tool_call_id: Some(call_id),
                                tool_name: Some("write_context".to_string()),
                                turn_id: None,
                                created_at: 0,
                            });
                            self.storage.create_turn(turn).await?;

                            let _ = self
                                .task_storage
                                .update_task_status(task_id, TaskStatus::Done)
                                .await;

                            return Ok(ContextAgentResponse::ContextSaved {
                                context: context_str,
                            });
                        }

                        unknown => {
                            return Err(AgentError::Other(format!(
                                "context_agent called unknown tool: {}",
                                unknown
                            )));
                        }
                    }
                }
                continue;
            }

            // No tool call — text-only response.
            // If the model gives a text response that looks like JSON context, save it.
            if !assistant_text.is_empty() {
                // Check if the text response looks like structured context (starts with {)
                let trimmed = assistant_text.trim();
                if trimmed.starts_with('{') || trimmed.starts_with('[') {
                    // Model returned context as text instead of a tool call — save it
                    self.context_storage
                        .save_context(
                            self.project_id,
                            self.context_type.as_str(),
                            trimmed,
                        )
                        .await?;

                    self.storage
                        .create_turn(vec![text_row(session_id, assistant_text.clone())])
                        .await?;

                    let _ = self
                        .task_storage
                        .update_task_status(task_id, TaskStatus::Done)
                        .await;

                    return Ok(ContextAgentResponse::ContextSaved {
                        context: trimmed.to_string(),
                    });
                }

                self.storage
                    .create_turn(vec![text_row(session_id, assistant_text.clone())])
                    .await?;
            }

            nudges += 1;
            if nudges >= MAX_NUDGES {
                return Err(AgentError::Other(
                    "context_agent did not produce context after multiple nudges".to_string(),
                ));
            }

            let nudge = match self.context_type {
                ContextType::Backend => {
                    "Continue exploring backend files and then respond with the complete JSON context summary wrapped in write_context(context=\"...\"). If write_context fails, just output the JSON directly."
                }
                ContextType::AdminGui => {
                    "Continue exploring admin-gui files and then respond with the complete JSON context summary wrapped in write_context(context=\"...\"). If write_context fails, just output the JSON directly."
                }
            };

            self.storage
                .create_message(ChatMessage {
                    id: None,
                    session_id,
                    role: "user".to_string(),
                    agent_type: None,
                    content: nudge.to_string(),
                    tool_call_id: None,
                    tool_name: None,
                    turn_id: None,
                    created_at: 0,
                })
                .await?;
        }
    }

    fn execute_list_files(&self, relative_path: &str) -> String {
        let base = &self.project_path;
        let target = if relative_path.is_empty() {
            base.clone()
        } else {
            base.join(relative_path)
        };

        let entries = match std::fs::read_dir(&target) {
            Ok(rd) => rd,
            Err(e) => return format!("Error reading directory: {}", e),
        };

        let mut dirs = Vec::new();
        let mut files = Vec::new();

        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            if let Ok(ft) = entry.file_type() {
                if ft.is_dir() {
                    dirs.push(format!("{}/", name));
                } else {
                    files.push(name);
                }
            }
        }

        dirs.sort();
        files.sort();

        let mut result = String::new();
        if !dirs.is_empty() {
            result.push_str("Directories:\n");
            for d in &dirs {
                result.push_str(&format!("  {}\n", d));
            }
        }
        if !files.is_empty() {
            result.push_str("Files:\n");
            for f in &files {
                result.push_str(&format!("  {}\n", f));
            }
        }
        if result.is_empty() {
            result = "(empty directory)\n".to_string();
        }
        result
    }

    fn execute_read_file(&self, relative_path: &str) -> String {
        let target = self.project_path.join(relative_path);
        match std::fs::read_to_string(&target) {
            Ok(content) => {
                let lines: Vec<&str> = content.lines().collect();
                if lines.len() > 500 {
                    format!(
                        "(File has {} lines, showing first 500)\n{}",
                        lines.len(),
                        lines[..500].join("\n")
                    )
                } else {
                    content
                }
            }
            Err(e) => format!("Error reading file: {}", e),
        }
    }
}