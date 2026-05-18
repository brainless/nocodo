use std::path::PathBuf;
use std::sync::Arc;

use llm_sdk::{
    client::LlmClient,
    tools::{Tool, ToolChoice},
    types::{CompletionRequest, Message, Role},
};

use crate::{
    error::AgentError,
    storage::{AgentStorage, ChatMessage, ContextStorage, TaskStatus, TaskStorage},
    utils::{
        cargo::collect_cargo_dependencies,
        context::normalize_backend_context_json,
        file_ops,
        tools::{CommentaryParams, ListFilesParams, ReadFileParams, UpdateTaskStatusParams},
    },
};

const AGENT_TYPE: &str = "backend_engineer";
const MAX_NUDGES: u32 = 8;

#[derive(Debug)]
pub enum BackendEngineerResponse {
    ContextSaved { context: String },
    Stopped(String),
}

pub struct BackendEngineerAgent {
    llm_client: Arc<dyn LlmClient>,
    storage: Arc<dyn AgentStorage>,
    context_storage: Arc<dyn ContextStorage>,
    task_storage: Arc<dyn TaskStorage>,
    model: String,
    project_id: i64,
    project_path: PathBuf,
}

impl BackendEngineerAgent {
    pub fn new(
        llm_client: Arc<dyn LlmClient>,
        storage: Arc<dyn AgentStorage>,
        context_storage: Arc<dyn ContextStorage>,
        task_storage: Arc<dyn TaskStorage>,
        model: impl Into<String>,
        project_id: i64,
        project_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            llm_client,
            storage,
            context_storage,
            task_storage,
            model: model.into(),
            project_id,
            project_path: project_path.into(),
        }
    }

    pub async fn run_for_task(
        &self,
        session_id: i64,
        task_id: i64,
    ) -> Result<BackendEngineerResponse, AgentError> {
        self.run_loop(session_id, task_id).await
    }

    async fn run_loop(
        &self,
        session_id: i64,
        task_id: i64,
    ) -> Result<BackendEngineerResponse, AgentError> {
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

        let open_file_alias_tool = Tool::from_type::<ReadFileParams>()
            .name("repo_browser.open_file")
            .description(
                "Alias of read_file. Read file contents at the path relative to project root.",
            )
            .build();

        let commentary_tool = Tool::from_type::<CommentaryParams>()
            .name("commentary")
            .description(
                "Optional commentary tool. Use plain assistant text instead when possible.",
            )
            .build();

        let update_task_status_tool = Tool::from_type::<UpdateTaskStatusParams>()
            .name("update_task_status")
            .description(
                "Update the status of the current task. Use \"in_progress\" when starting, \
                 \"done\" when the context has been saved.",
            )
            .build();

        let text_row = |session_id: i64, content: String| ChatMessage {
            id: None,
            session_id,
            role: "assistant".to_string(),
            agent_type: Some(AGENT_TYPE.to_string()),
            content,
            tool_call_id: None,
            tool_name: None,
            turn_id: None,
            created_at: 0,
        };

        let cargo_deps = collect_cargo_dependencies(&self.project_path, "backend/Cargo.toml");
        let system_prompt = super::prompts::system_prompt(&cargo_deps);

        let mut nudges: u32 = 0;
        let mut last_tool_signature: Option<String> = None;
        let mut same_tool_call_streak: u32 = 0;

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
                    open_file_alias_tool.clone(),
                    update_task_status_tool.clone(),
                    commentary_tool.clone(),
                ]),
                tool_choice: Some(ToolChoice::Auto),
                response_format: None,
            };

            let response = match self.llm_client.complete(request).await {
                Ok(resp) => resp,
                Err(llm_err) => {
                    let err_str = llm_err.to_string();
                    if err_str.contains("validation failed")
                        || err_str.contains("missing properties")
                    {
                        log::warn!(
                            "[backend_engineer] LLM tool validation error (attempt {}): {}. Falling back to text output.",
                            nudges, err_str
                        );
                        self.storage
                            .create_message(ChatMessage {
                                id: None,
                                session_id,
                                role: "user".to_string(),
                                agent_type: None,
                                content: "A tool call had a validation error. Instead of using tools now, output your complete JSON context summary directly as plain text.".to_string(),
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
                            let signature = format!(
                                "{}:{}",
                                tool_name,
                                serde_json::to_string(tool_call.arguments())?
                            );
                            if last_tool_signature.as_deref() == Some(&signature) {
                                same_tool_call_streak += 1;
                            } else {
                                last_tool_signature = Some(signature);
                                same_tool_call_streak = 0;
                            }
                            if same_tool_call_streak >= 2 {
                                let _ = self
                                    .task_storage
                                    .update_task_status(task_id, TaskStatus::Blocked)
                                    .await;
                                return Ok(BackendEngineerResponse::Stopped(
                                    "stopped due to repeated identical tool call".to_string(),
                                ));
                            }
                            let params: ListFilesParams =
                                tool_call.parse_arguments().map_err(AgentError::Llm)?;
                            let result = file_ops::list_files(&self.project_path, &params.path);
                            let mut turn = Vec::new();
                            if !assistant_text.is_empty() {
                                turn.push(text_row(session_id, assistant_text.clone()));
                            }
                            turn.push(ChatMessage {
                                id: None,
                                session_id,
                                role: "assistant".to_string(),
                                agent_type: Some(AGENT_TYPE.to_string()),
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

                        "read_file" | "repo_browser.read_file" | "repo_browser.open_file" => {
                            let signature = format!(
                                "{}:{}",
                                tool_name,
                                serde_json::to_string(tool_call.arguments())?
                            );
                            if last_tool_signature.as_deref() == Some(&signature) {
                                same_tool_call_streak += 1;
                            } else {
                                last_tool_signature = Some(signature);
                                same_tool_call_streak = 0;
                            }
                            if same_tool_call_streak >= 2 {
                                let _ = self
                                    .task_storage
                                    .update_task_status(task_id, TaskStatus::Blocked)
                                    .await;
                                return Ok(BackendEngineerResponse::Stopped(
                                    "stopped due to repeated identical tool call".to_string(),
                                ));
                            }
                            let params: ReadFileParams =
                                tool_call.parse_arguments().map_err(AgentError::Llm)?;
                            let result = file_ops::read_file(&self.project_path, &params.path);
                            let mut turn = Vec::new();
                            if !assistant_text.is_empty() {
                                turn.push(text_row(session_id, assistant_text.clone()));
                            }
                            turn.push(ChatMessage {
                                id: None,
                                session_id,
                                role: "assistant".to_string(),
                                agent_type: Some(AGENT_TYPE.to_string()),
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
                            let signature = format!(
                                "{}:{}",
                                tool_name,
                                serde_json::to_string(tool_call.arguments())?
                            );
                            if last_tool_signature.as_deref() == Some(&signature) {
                                same_tool_call_streak += 1;
                            } else {
                                last_tool_signature = Some(signature);
                                same_tool_call_streak = 0;
                            }
                            if same_tool_call_streak >= 2 {
                                let _ = self
                                    .task_storage
                                    .update_task_status(task_id, TaskStatus::Blocked)
                                    .await;
                                return Ok(BackendEngineerResponse::Stopped(
                                    "stopped due to repeated identical tool call".to_string(),
                                ));
                            }
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
                                agent_type: Some(AGENT_TYPE.to_string()),
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

                        "commentary" => {
                            let params: CommentaryParams =
                                tool_call.parse_arguments().map_err(AgentError::Llm)?;
                            let content = params
                                .text
                                .unwrap_or_else(|| "Commentary received.".to_string());
                            let mut turn = Vec::new();
                            if !assistant_text.is_empty() {
                                turn.push(text_row(session_id, assistant_text.clone()));
                            }
                            turn.push(ChatMessage {
                                id: None,
                                session_id,
                                role: "assistant".to_string(),
                                agent_type: Some(AGENT_TYPE.to_string()),
                                content: serde_json::to_string(tool_call.arguments())?,
                                tool_call_id: Some(call_id.clone()),
                                tool_name: Some("commentary".to_string()),
                                turn_id: None,
                                created_at: 0,
                            });
                            turn.push(ChatMessage {
                                id: None,
                                session_id,
                                role: "tool".to_string(),
                                agent_type: None,
                                content,
                                tool_call_id: Some(call_id),
                                tool_name: Some("commentary".to_string()),
                                turn_id: None,
                                created_at: 0,
                            });
                            self.storage.create_turn(turn).await?;
                        }

                        unknown => {
                            return Err(AgentError::Other(format!(
                                "backend_engineer called unknown tool: {}",
                                unknown
                            )));
                        }
                    }
                }
                continue;
            }

            // No tool call — if the response looks like JSON context, save it.
            if !assistant_text.is_empty() {
                let trimmed = assistant_text.trim();
                if trimmed.starts_with('{') || trimmed.starts_with('[') {
                    let normalized = normalize_backend_context_json(trimmed);
                    self.context_storage
                        .save_context(self.project_id, AGENT_TYPE, &normalized)
                        .await?;
                    self.storage
                        .create_turn(vec![text_row(session_id, assistant_text.clone())])
                        .await?;
                    let _ = self
                        .task_storage
                        .update_task_status(task_id, TaskStatus::Done)
                        .await;
                    return Ok(BackendEngineerResponse::ContextSaved {
                        context: normalized,
                    });
                }

                self.storage
                    .create_turn(vec![text_row(session_id, assistant_text.clone())])
                    .await?;
            }

            nudges += 1;
            if nudges >= MAX_NUDGES {
                return Err(AgentError::Other(
                    "backend_engineer did not produce context after multiple nudges".to_string(),
                ));
            }

            self.storage
                .create_message(ChatMessage {
                    id: None,
                    session_id,
                    role: "user".to_string(),
                    agent_type: None,
                    content: "Continue exploring backend files and respond with the complete JSON context summary as plain text only.".to_string(),
                    tool_call_id: None,
                    tool_name: None,
                    turn_id: None,
                    created_at: 0,
                })
                .await?;
        }
    }
}
