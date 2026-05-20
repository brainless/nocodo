use std::sync::Arc;

use llm_sdk::{
    client::LlmClient,
    tools::{Tool, ToolChoice},
    types::{CompletionRequest, Message, Role},
};

use super::modes::{project_naming, requirements_gathering};
use super::tools::{CompleteRequirementsParams, RecordProjectNoteParams, SetProjectNameParams};
use crate::{
    config::AgentConfig,
    error::AgentError,
    storage::{
        AgentStorage, AgentType, CommentStorage, ProjectNoteStorage, ProjectNoteTopic,
        QuestionKind, StructuredQuestion, TaskStorage,
    },
    task_policy,
    user_input_tool::{InputType, RequestUserInputParams},
};

// ---------------------------------------------------------------------------
// Public result type
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum PoSessionResult {
    Text(String),
    Questions {
        message: String,
        questions: Vec<StructuredQuestion>,
    },
    /// All questions answered and notes saved; `closing_message` is shown to the user.
    /// Backend should follow up with a `project_naming` mode call.
    RequirementsComplete {
        closing_message: String,
    },
    /// Project has been named via `set_project_name`; backend should trigger PM handoff.
    Named,
    Silent,
}

// ---------------------------------------------------------------------------
// Agent
// ---------------------------------------------------------------------------

pub struct ProductOwnerAgent {
    llm_client: Arc<dyn LlmClient>,
    storage: Arc<dyn AgentStorage>,
    task_storage: Arc<dyn TaskStorage>,
    _comment_storage: Arc<dyn CommentStorage>,
    note_storage: Arc<dyn ProjectNoteStorage>,
    model: String,
    project_id: i64,
}

impl ProductOwnerAgent {
    pub fn new(
        storage: Arc<dyn AgentStorage>,
        task_storage: Arc<dyn TaskStorage>,
        comment_storage: Arc<dyn CommentStorage>,
        note_storage: Arc<dyn ProjectNoteStorage>,
        config: AgentConfig,
        project_id: i64,
    ) -> Result<Self, AgentError> {
        let llm_client = crate::make_llm_client(&config)?;
        Ok(Self {
            llm_client,
            storage,
            task_storage,
            _comment_storage: comment_storage,
            note_storage,
            model: config.model,
            project_id,
        })
    }

    /// Run the PO agent.
    ///
    /// `is_naming = false` — requirements gathering mode: ask questions, record notes,
    /// signal completion via `complete_requirements`.
    ///
    /// `is_naming = true` — project naming mode: derive a project name from the
    /// conversation history and call `set_project_name`. No user interaction.
    pub async fn respond_in_session(
        &self,
        session_id: i64,
        messages: Vec<(String, String)>,
        is_naming: bool,
    ) -> Result<PoSessionResult, AgentError> {
        if is_naming {
            self.run_project_naming(messages).await
        } else {
            self.run_requirements_gathering(session_id, messages).await
        }
    }

    // -----------------------------------------------------------------------
    // Requirements gathering mode
    // -----------------------------------------------------------------------

    async fn run_requirements_gathering(
        &self,
        session_id: i64,
        messages: Vec<(String, String)>,
    ) -> Result<PoSessionResult, AgentError> {
        let ask_tool = Tool::from_type::<RequestUserInputParams>()
            .name("request_user_input")
            .description(
                "Ask the user a structured question with predefined choices. \
                 Use this instead of listing options in prose — the UI will render \
                 radio buttons or checkboxes. You may call this tool multiple times \
                 in one turn when the questions are independent.",
            )
            .build();

        let note_tool = Tool::from_type::<RecordProjectNoteParams>()
            .name("record_project_note")
            .description(
                "Record a business-layer artifact (goal, constraint, decision, context, or \
                 assumption) discovered during intake. Call this as you learn key facts — \
                 you may call it multiple times. These notes become the requirements brief \
                 for the development team. Use replaces_note to supersede an earlier note \
                 when the user clarifies or changes direction.",
            )
            .build();

        let complete_tool = Tool::from_type::<CompleteRequirementsParams>()
            .name("complete_requirements")
            .description(
                "Signal that all questions are answered and project notes are saved. \
                 Provide a short, warm closing message for the user. \
                 Call this only when you have enough for a clear requirements brief.",
            )
            .build();

        let tools = vec![ask_tool, note_tool, complete_tool];

        let mut llm_messages = build_llm_messages(&messages);

        const MAX_ITERATIONS: usize = 6;
        for iteration in 0..MAX_ITERATIONS {
            let request = CompletionRequest {
                messages: llm_messages.clone(),
                max_tokens: 1024,
                model: self.model.clone(),
                system: Some(requirements_gathering::system_prompt()),
                temperature: Some(0.3),
                top_p: None,
                stop_sequences: None,
                tools: Some(tools.clone()),
                tool_choice: Some(ToolChoice::Auto),
                response_format: None,
            };

            log::info!(
                "[PO:requirements_gathering] iteration={} model={} msg_count={}",
                iteration,
                self.model,
                request.messages.len()
            );
            let response = self.llm_client.complete(request).await?;
            log::info!(
                "[PO:requirements_gathering] iteration={} stop_reason={:?}",
                iteration,
                response.stop_reason
            );

            let text = extract_text(&response.content);

            let Some(tool_calls) = response.tool_calls else {
                return Ok(if text.trim().is_empty() {
                    PoSessionResult::Silent
                } else {
                    PoSessionResult::Text(text)
                });
            };

            let mut structured_questions: Vec<StructuredQuestion> = Vec::new();
            let mut completion: Option<String> = None;
            let mut tool_result_messages: Vec<Message> = Vec::new();
            let mut assistant_tool_msgs: Vec<Message> = Vec::new();

            for tool_call in &tool_calls {
                log::info!(
                    "[PO:requirements_gathering] tool id={} name={}",
                    tool_call.id(),
                    tool_call.name()
                );

                assistant_tool_msgs.push(Message {
                    role: Role::Assistant,
                    content: vec![llm_sdk::types::ContentBlock::Text {
                        text: tool_call.raw_arguments().to_string(),
                    }],
                    tool_call_id: Some(tool_call.id().to_string()),
                    tool_name: Some(tool_call.name().to_string()),
                });

                let tool_result = match tool_call.name() {
                    "request_user_input" => {
                        let params: RequestUserInputParams =
                            tool_call.parse_arguments().map_err(AgentError::Llm)?;
                        let kind = match params.input_type {
                            InputType::SingleChoice => QuestionKind::SingleChoice {
                                options: params.options,
                            },
                            InputType::MultipleChoice => QuestionKind::MultipleChoice {
                                options: params.options,
                            },
                        };
                        structured_questions.push(StructuredQuestion {
                            question: params.question,
                            kind,
                        });
                        "Question queued for user".to_string()
                    }
                    "record_project_note" => {
                        let params: RecordProjectNoteParams =
                            match tool_call.parse_arguments().map_err(AgentError::Llm) {
                                Ok(p) => p,
                                Err(e) => {
                                    log::warn!(
                                        "[PO] record_project_note parse error: {}",
                                        e
                                    );
                                    tool_result_messages.push(Message::tool(
                                        tool_call.id(),
                                        format!("Error: {}", e),
                                    ));
                                    continue;
                                }
                            };
                        let topic = ProjectNoteTopic::from_str(&params.topic);
                        if let Err(e) = self
                            .note_storage
                            .add_note(
                                self.project_id,
                                topic,
                                params.note,
                                Some(session_id),
                                params.replaces_note,
                            )
                            .await
                        {
                            log::warn!("[PO] record_project_note storage error: {}", e);
                        }
                        "Note recorded".to_string()
                    }
                    "complete_requirements" => {
                        let params: CompleteRequirementsParams =
                            tool_call.parse_arguments().map_err(AgentError::Llm)?;
                        completion = Some(params.closing_message);
                        "Requirements marked complete".to_string()
                    }
                    other => {
                        log::warn!("[PO:requirements_gathering] unknown tool: {}", other);
                        "Unknown tool".to_string()
                    }
                };

                tool_result_messages.push(Message::tool(tool_call.id(), tool_result));
            }

            log::info!(
                "[PO:requirements_gathering] iteration={} complete={} questions={}",
                iteration,
                completion.is_some(),
                structured_questions.len()
            );

            if let Some(closing_message) = completion {
                return Ok(PoSessionResult::RequirementsComplete { closing_message });
            }

            if !structured_questions.is_empty() {
                return Ok(PoSessionResult::Questions {
                    message: text,
                    questions: structured_questions,
                });
            }

            // Only note-recording calls: feed tool results back and loop.
            llm_messages.extend(assistant_tool_msgs);
            llm_messages.extend(tool_result_messages);
            log::info!(
                "[PO:requirements_gathering] only notes recorded, looping (msg_count={})",
                llm_messages.len()
            );
        }

        log::warn!(
            "[PO:requirements_gathering] hit MAX_ITERATIONS={} without user-facing result",
            MAX_ITERATIONS
        );
        Ok(PoSessionResult::Silent)
    }

    // -----------------------------------------------------------------------
    // Project naming mode
    // -----------------------------------------------------------------------

    async fn run_project_naming(
        &self,
        messages: Vec<(String, String)>,
    ) -> Result<PoSessionResult, AgentError> {
        let name_tool = Tool::from_type::<SetProjectNameParams>()
            .name("set_project_name")
            .description(
                "Set a concise, descriptive name for the project derived from the user's domain.",
            )
            .build();

        let llm_messages = build_llm_messages(&messages);

        let request = CompletionRequest {
            messages: llm_messages,
            max_tokens: 256,
            model: self.model.clone(),
            system: Some(project_naming::system_prompt()),
            temperature: Some(0.2),
            top_p: None,
            stop_sequences: None,
            tools: Some(vec![name_tool]),
            tool_choice: Some(ToolChoice::Auto),
            response_format: None,
        };

        log::info!("[PO:project_naming] calling LLM model={}", self.model);
        let response = self.llm_client.complete(request).await?;
        log::info!(
            "[PO:project_naming] stop_reason={:?}",
            response.stop_reason
        );

        let Some(tool_calls) = response.tool_calls else {
            log::warn!("[PO:project_naming] no tool call — model returned text only");
            return Ok(PoSessionResult::Silent);
        };

        for tool_call in &tool_calls {
            if tool_call.name() == "set_project_name" {
                let params: SetProjectNameParams =
                    tool_call.parse_arguments().map_err(AgentError::Llm)?;
                log::info!("[PO:project_naming] set_project_name={:?}", params.name);
                if let Err(e) = self
                    .storage
                    .rename_project(self.project_id, &params.name)
                    .await
                {
                    log::warn!("[PO:project_naming] rename_project error: {}", e);
                }
                return Ok(PoSessionResult::Named);
            }
        }

        log::warn!("[PO:project_naming] set_project_name not called");
        Ok(PoSessionResult::Silent)
    }

    pub async fn validate_tasks(&self, task_ids: Vec<i64>) -> Result<(), AgentError> {
        for task_id in task_ids {
            let task = self
                .task_storage
                .get_task(task_id)
                .await?
                .ok_or_else(|| AgentError::Other(format!("Task {} not found", task_id)))?;

            let agent_type = AgentType::from_str(&task.assigned_to_agent);
            let next_state = task_policy::initial_state_for(&agent_type);
            self.task_storage
                .update_task_status(task_id, next_state)
                .await?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn build_llm_messages(messages: &[(String, String)]) -> Vec<Message> {
    messages
        .iter()
        .map(|(role, content)| {
            let r = match role.as_str() {
                "assistant" => Role::Assistant,
                "tool" => Role::Tool,
                _ => Role::User,
            };
            Message {
                role: r,
                content: vec![llm_sdk::types::ContentBlock::Text {
                    text: content.clone(),
                }],
                tool_call_id: None,
                tool_name: None,
            }
        })
        .collect()
}

fn extract_text(content: &[llm_sdk::types::ContentBlock]) -> String {
    content
        .iter()
        .filter_map(|b| match b {
            llm_sdk::types::ContentBlock::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("")
}
