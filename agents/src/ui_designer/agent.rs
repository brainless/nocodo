use std::sync::Arc;

use llm_sdk::{
    client::LlmClient,
    tools::{Tool, ToolChoice},
    types::{CompletionRequest, Message, Role},
};

use super::{prompts::system_prompt, tools::UpdateTaskStatusParams, FormLayout};
use crate::{
    error::AgentError,
    storage::{AgentStorage, AgentType, ChatMessage, TaskStatus, TaskStorage, UiFormStorage},
};

const MAX_NUDGES: u32 = 3;

#[derive(Debug)]
pub enum UiDesignerResponse {
    FormGenerated(FormLayout),
    Stopped(String),
}

pub struct UiDesignerAgent {
    llm_client: Arc<dyn LlmClient>,
    storage: Arc<dyn AgentStorage>,
    form_storage: Arc<dyn UiFormStorage>,
    task_storage: Arc<dyn TaskStorage>,
    model: String,
    project_id: i64,
}

impl UiDesignerAgent {
    pub fn new(
        llm_client: Arc<dyn LlmClient>,
        storage: Arc<dyn AgentStorage>,
        form_storage: Arc<dyn UiFormStorage>,
        task_storage: Arc<dyn TaskStorage>,
        model: impl Into<String>,
        project_id: i64,
    ) -> Self {
        Self {
            llm_client,
            storage,
            form_storage,
            task_storage,
            model: model.into(),
            project_id,
        }
    }

    pub async fn run_for_task(
        &self,
        session_id: i64,
        task_id: i64,
    ) -> Result<UiDesignerResponse, AgentError> {
        self.run_loop(session_id, task_id).await
    }

    async fn run_loop(
        &self,
        session_id: i64,
        task_id: i64,
    ) -> Result<UiDesignerResponse, AgentError> {
        let write_form_tool = Tool::from_type::<FormLayout>()
            .name("write_form_layout")
            .description(
                "Write the complete form layout for this entity. Call exactly once with all rows and fields.",
            )
            .build();

        let update_task_status_tool = Tool::from_type::<UpdateTaskStatusParams>()
            .name("update_task_status")
            .description(
                "Update the status of the current task. Use \"in_progress\" when starting, \
                 \"done\" when the form is saved, \"blocked\" if the input is unusable.",
            )
            .build();

        let agent_type_str = AgentType::UiDesigner.as_str().to_string();
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
                system: Some(system_prompt().to_string()),
                temperature: Some(0.2),
                top_p: None,
                stop_sequences: None,
                tools: Some(vec![write_form_tool.clone(), update_task_status_tool.clone()]),
                tool_choice: Some(ToolChoice::Auto),
                response_format: None,
            };

            let response = self.llm_client.complete(request).await?;

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
                        "write_form_layout" => {
                            let form: FormLayout =
                                tool_call.parse_arguments().map_err(AgentError::Llm)?;
                            let layout_json = serde_json::to_string(&form)?;

                            self.form_storage
                                .save_form_layout(self.project_id, &form.entity, &layout_json)
                                .await?;

                            let result_text =
                                format!("Form layout saved for entity '{}'.", form.entity);

                            let mut turn = Vec::new();
                            if !assistant_text.is_empty() {
                                turn.push(text_row(session_id, assistant_text.clone()));
                            }
                            turn.push(ChatMessage {
                                id: None,
                                session_id,
                                role: "assistant".to_string(),
                                agent_type: Some(agent_type_str.clone()),
                                content: layout_json,
                                tool_call_id: Some(call_id.clone()),
                                tool_name: Some("write_form_layout".to_string()),
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
                                tool_name: Some("write_form_layout".to_string()),
                                turn_id: None,
                                created_at: 0,
                            });
                            self.storage.create_turn(turn).await?;

                            let _ = self
                                .task_storage
                                .update_task_status(task_id, TaskStatus::Done)
                                .await;

                            return Ok(UiDesignerResponse::FormGenerated(form));
                        }

                        "update_task_status" => {
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
                            // Continue loop — model will follow up.
                        }

                        unknown => {
                            return Err(AgentError::Other(format!(
                                "ui_designer called unknown tool: {}",
                                unknown
                            )));
                        }
                    }
                }
                continue;
            }

            // No tool call — text-only response.
            if !assistant_text.is_empty() {
                self.storage
                    .create_turn(vec![text_row(session_id, assistant_text.clone())])
                    .await?;
                // If it replied with text but no write_form_layout, nudge it.
            }

            nudges += 1;
            if nudges >= MAX_NUDGES {
                return Err(AgentError::Other(
                    "ui_designer did not call write_form_layout after multiple nudges".to_string(),
                ));
            }

            self.storage
                .create_message(ChatMessage {
                    id: None,
                    session_id,
                    role: "user".to_string(),
                    agent_type: None,
                    content: "Please call write_form_layout now with the complete form definition."
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
