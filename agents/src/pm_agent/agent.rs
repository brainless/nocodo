use std::sync::Arc;

use llm_sdk::{
    client::LlmClient,
    tools::{Tool, ToolChoice},
    types::{CompletionRequest, Message, Role},
};

use super::{
    prompts::{init_project_system_prompt, system_prompt},
    tools::{
        CreateEpicParams, CreateTaskParams, ListPendingReviewTasksParams,
        PmUpdateTaskStatusParams, SetProjectNameParams,
    },
};
use crate::{
    error::AgentError,
    storage::{AgentStorage, AgentType, ChatMessage, Epic, EpicStatus, Task, TaskStatus, TaskStorage},
};

const MAX_NUDGES: u32 = 3;

// ---------------------------------------------------------------------------
// Public result type
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum PmResponse {
    Text(String),
    Stopped(String),
}

// ---------------------------------------------------------------------------
// Agent
// ---------------------------------------------------------------------------

pub struct PmAgent {
    llm_client: Arc<dyn LlmClient>,
    storage: Arc<dyn AgentStorage>,
    task_storage: Arc<dyn TaskStorage>,
    model: String,
    project_id: i64,
}

impl PmAgent {
    pub fn new(
        llm_client: Arc<dyn LlmClient>,
        storage: Arc<dyn AgentStorage>,
        task_storage: Arc<dyn TaskStorage>,
        model: impl Into<String>,
        project_id: i64,
    ) -> Self {
        Self {
            llm_client,
            storage,
            task_storage,
            model: model.into(),
            project_id,
        }
    }

    /// Run the PM agent for an existing session + task.
    /// The caller creates the task + session and persists the user message first.
    pub async fn chat_with_session(
        &self,
        session_id: i64,
        task_id: i64,
    ) -> Result<PmResponse, AgentError> {
        self.run_loop(session_id, task_id, false).await
    }

    /// Run the PM agent for a brand-new project's first message.
    /// Uses the project-init system prompt: create Epic + schema_designer task immediately.
    pub async fn chat_with_session_init(
        &self,
        session_id: i64,
        task_id: i64,
    ) -> Result<PmResponse, AgentError> {
        self.run_loop(session_id, task_id, true).await
    }

    // -----------------------------------------------------------------------
    // Internal loop
    // -----------------------------------------------------------------------

    async fn run_loop(
        &self,
        session_id: i64,
        task_id: i64,
        is_init: bool,
    ) -> Result<PmResponse, AgentError> {
        let list_tool = Tool::from_type::<ListPendingReviewTasksParams>()
            .name("list_pending_review_tasks")
            .description(
                "List all tasks currently in 'review' status for this project. \
                 Call this at the start of every session to triage pending work.",
            )
            .build();

        let create_epic_tool = Tool::from_type::<CreateEpicParams>()
            .name("create_epic")
            .description(
                "Create a new epic representing a user initiative. \
                 Returns the epic_id to use when creating tasks.",
            )
            .build();

        let create_task_tool = Tool::from_type::<CreateTaskParams>()
            .name("create_task")
            .description(
                "Create a task and assign it to a focused agent. \
                 The focused agent will receive source_prompt as its primary input.",
            )
            .build();

        let update_status_tool = Tool::from_type::<PmUpdateTaskStatusParams>()
            .name("update_task_status")
            .description(
                "Update the status of a task you are managing. \
                 Valid values: \"in_progress\", \"review\", \"done\", \"blocked\".",
            )
            .build();

        let set_project_name_tool = Tool::from_type::<SetProjectNameParams>()
            .name("set_project_name")
            .description(
                "Set a descriptive name for this project based on the user's domain. \
                 Call this once during project init, before creating the epic.",
            )
            .build();

        let agent_type_str = AgentType::ProjectManager.as_str().to_string();
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
                max_tokens: 4096,
                model: self.model.clone(),
                system: Some(if is_init { init_project_system_prompt() } else { system_prompt() }),
                temperature: Some(0.2),
                top_p: None,
                stop_sequences: None,
                tools: Some({
                    let mut tools = vec![
                        create_epic_tool.clone(),
                        create_task_tool.clone(),
                        update_status_tool.clone(),
                    ];
                    if is_init {
                        tools.insert(0, set_project_name_tool.clone());
                    } else {
                        tools.insert(0, list_tool.clone());
                    }
                    tools
                }),
                tool_choice: Some(ToolChoice::Auto),
                response_format: None,
            };

            log::info!("[PM] Calling LLM with model={}", self.model);
            let response = self.llm_client.complete(request).await?;
            log::info!("[PM] LLM response received, stop_reason={:?}", response.stop_reason);

            let assistant_text = response
                .content
                .iter()
                .filter_map(|b| match b {
                    llm_sdk::types::ContentBlock::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("");

            let text_row = |content: String| ChatMessage {
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

            if let Some(tool_calls) = response.tool_calls {
                log::info!("[PM] {} tool call(s)", tool_calls.len());
                for tool_call in tool_calls {
                    let tool_name = tool_call.name();
                    let call_id = tool_call.id().to_string();
                    log::info!("[PM] Tool: {}", tool_name);

                    match tool_name {
                        "set_project_name" => {
                            let params: SetProjectNameParams =
                                tool_call.parse_arguments().map_err(AgentError::Llm)?;

                            let result_text = match self
                                .storage
                                .rename_project(self.project_id, &params.name)
                                .await
                            {
                                Ok(()) => format!("Project renamed to \"{}\".", params.name),
                                Err(e) => format!("Failed to rename project: {}", e),
                            };

                            let mut turn = Vec::new();
                            if !assistant_text.is_empty() {
                                turn.push(text_row(assistant_text.clone()));
                            }
                            turn.push(ChatMessage {
                                id: None,
                                session_id,
                                role: "assistant".to_string(),
                                agent_type: Some(agent_type_str.clone()),
                                content: serde_json::to_string(tool_call.arguments())?,
                                tool_call_id: Some(call_id.clone()),
                                tool_name: Some("set_project_name".to_string()),
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
                                tool_name: Some("set_project_name".to_string()),
                                turn_id: None,
                                created_at: 0,
                            });
                            self.storage.create_turn(turn).await?;
                        }

                        "list_pending_review_tasks" => {
                            let tasks = self
                                .task_storage
                                .list_pending_review_tasks(self.project_id)
                                .await
                                .unwrap_or_default();

                            let result_text = if tasks.is_empty() {
                                "No tasks pending review.".to_string()
                            } else {
                                let lines: Vec<String> = tasks
                                    .iter()
                                    .map(|t| {
                                        format!(
                                            "- Task #{}: [{}] {} (agent: {})",
                                            t.id.unwrap_or(0),
                                            t.status.as_str(),
                                            t.title,
                                            t.assigned_to_agent,
                                        )
                                    })
                                    .collect();
                                lines.join("\n")
                            };

                            let mut turn = Vec::new();
                            if !assistant_text.is_empty() {
                                turn.push(text_row(assistant_text.clone()));
                            }
                            turn.push(ChatMessage {
                                id: None,
                                session_id,
                                role: "assistant".to_string(),
                                agent_type: Some(agent_type_str.clone()),
                                content: serde_json::to_string(tool_call.arguments())?,
                                tool_call_id: Some(call_id.clone()),
                                tool_name: Some("list_pending_review_tasks".to_string()),
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
                                tool_name: Some("list_pending_review_tasks".to_string()),
                                turn_id: None,
                                created_at: 0,
                            });
                            self.storage.create_turn(turn).await?;
                        }

                        "create_epic" => {
                            let params: CreateEpicParams =
                                tool_call.parse_arguments().map_err(AgentError::Llm)?;

                            let now = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs() as i64;

                            let result_text = match self
                                .task_storage
                                .create_epic(Epic {
                                    id: None,
                                    project_id: self.project_id,
                                    title: params.title.clone(),
                                    description: params.description.clone(),
                                    source_prompt: params.description.clone(),
                                    status: EpicStatus::Open,
                                    created_by_agent: agent_type_str.clone(),
                                    created_by_task_id: Some(task_id),
                                    created_at: now,
                                    updated_at: now,
                                })
                                .await
                            {
                                Ok(id) => format!(
                                    "Epic created: id={}, title=\"{}\"",
                                    id, params.title
                                ),
                                Err(e) => format!("Failed to create epic: {}", e),
                            };

                            let mut turn = Vec::new();
                            if !assistant_text.is_empty() {
                                turn.push(text_row(assistant_text.clone()));
                            }
                            turn.push(ChatMessage {
                                id: None,
                                session_id,
                                role: "assistant".to_string(),
                                agent_type: Some(agent_type_str.clone()),
                                content: serde_json::to_string(tool_call.arguments())?,
                                tool_call_id: Some(call_id.clone()),
                                tool_name: Some("create_epic".to_string()),
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
                                tool_name: Some("create_epic".to_string()),
                                turn_id: None,
                                created_at: 0,
                            });
                            self.storage.create_turn(turn).await?;
                        }

                        "create_task" => {
                            let params: CreateTaskParams =
                                tool_call.parse_arguments().map_err(AgentError::Llm)?;

                            let now = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs() as i64;

                            let result_text = match self
                                .task_storage
                                .create_task(Task {
                                    id: None,
                                    project_id: self.project_id,
                                    epic_id: params.epic_id,
                                    title: params.title.clone(),
                                    description: params.description.clone(),
                                    source_prompt: params.source_prompt.clone(),
                                    assigned_to_agent: params.assigned_to_agent.clone(),
                                    status: TaskStatus::Open,
                                    depends_on_task_id: params.depends_on_task_id,
                                    created_by_agent: agent_type_str.clone(),
                                    created_at: now,
                                    updated_at: now,
                                })
                                .await
                            {
                                Ok(id) => format!(
                                    "Task created: id={}, title=\"{}\", assigned_to={}",
                                    id, params.title, params.assigned_to_agent
                                ),
                                Err(e) => format!("Failed to create task: {}", e),
                            };

                            // PM's own task gains InProgress when it creates sub-tasks.
                            let _ = self
                                .task_storage
                                .update_task_status(task_id, TaskStatus::InProgress)
                                .await;

                            let mut turn = Vec::new();
                            if !assistant_text.is_empty() {
                                turn.push(text_row(assistant_text.clone()));
                            }
                            turn.push(ChatMessage {
                                id: None,
                                session_id,
                                role: "assistant".to_string(),
                                agent_type: Some(agent_type_str.clone()),
                                content: serde_json::to_string(tool_call.arguments())?,
                                tool_call_id: Some(call_id.clone()),
                                tool_name: Some("create_task".to_string()),
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
                                tool_name: Some("create_task".to_string()),
                                turn_id: None,
                                created_at: 0,
                            });
                            self.storage.create_turn(turn).await?;
                        }

                        "update_task_status" => {
                            let params: PmUpdateTaskStatusParams =
                                tool_call.parse_arguments().map_err(AgentError::Llm)?;

                            let new_status = TaskStatus::from_str(&params.status);
                            let result_text = match self
                                .task_storage
                                .update_task_status(params.task_id, new_status)
                                .await
                            {
                                Ok(()) => format!(
                                    "Task #{} status updated to {}.",
                                    params.task_id, params.status
                                ),
                                Err(e) => format!("Failed to update task status: {}", e),
                            };

                            let mut turn = Vec::new();
                            if !assistant_text.is_empty() {
                                turn.push(text_row(assistant_text.clone()));
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

                        unknown => {
                            log::error!("[PM] Unknown tool: {}", unknown);
                            return Err(AgentError::Other(format!(
                                "PM called unknown tool: {}",
                                unknown
                            )));
                        }
                    }
                }

                // After handling all tool calls, loop to get the LLM's text summary.
                continue;
            }

            // No tool call — plain text response, we're done.
            if !assistant_text.is_empty() {
                self.storage
                    .create_turn(vec![text_row(assistant_text.clone())])
                    .await?;
                return Ok(PmResponse::Text(assistant_text));
            }

            nudges += 1;
            log::warn!("[PM] No response, nudge {}/{}", nudges, MAX_NUDGES);
            if nudges >= MAX_NUDGES {
                return Err(AgentError::Other(
                    "PM agent did not produce a response after multiple nudges.".to_string(),
                ));
            }
            self.storage
                .create_message(ChatMessage {
                    id: None,
                    session_id,
                    role: "user".to_string(),
                    agent_type: None,
                    content: "Please summarize what you have done and what the user should do next."
                        .to_string(),
                    tool_call_id: None,
                    tool_name: None,
                    turn_id: None,
                    created_at: 0,
                })
                .await?;
        }
    }
}
