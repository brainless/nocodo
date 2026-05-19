use std::sync::Arc;

use llm_sdk::{
    client::LlmClient,
    tools::{Tool, ToolChoice},
    types::{CompletionRequest, Message, Role},
};

use super::prompts::po_user_session_system_prompt;
use super::tools::{HandOffToPmParams, RecordProjectNoteParams};
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
    HandedOff {
        final_message: String,
    },
    Silent,
}

// ---------------------------------------------------------------------------
// Agent
// ---------------------------------------------------------------------------

pub struct ProductOwnerAgent {
    llm_client: Arc<dyn LlmClient>,
    _storage: Arc<dyn AgentStorage>,
    task_storage: Arc<dyn TaskStorage>,
    _comment_storage: Arc<dyn CommentStorage>,
    note_storage: Arc<dyn ProjectNoteStorage>,
    model: String,
    project_id: i64,
}

impl ProductOwnerAgent {
    pub fn new(
        _storage: Arc<dyn AgentStorage>,
        task_storage: Arc<dyn TaskStorage>,
        _comment_storage: Arc<dyn CommentStorage>,
        note_storage: Arc<dyn ProjectNoteStorage>,
        config: AgentConfig,
        project_id: i64,
    ) -> Result<Self, AgentError> {
        let llm_client = crate::make_llm_client(&config)?;
        Ok(Self {
            llm_client,
            _storage,
            task_storage,
            _comment_storage,
            note_storage,
            model: config.model,
            project_id,
        })
    }

    pub async fn respond_in_session(
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
                 for the Project Manager. Use replaces_note to supersede an earlier note \
                 when the user clarifies or changes direction.",
            )
            .build();

        let handoff_tool = Tool::from_type::<HandOffToPmParams>()
            .name("hand_off_to_pm")
            .description(
                "Call this when you have gathered enough requirements and recorded the key \
                 project notes. Provide a friendly closing message for the user. The Project \
                 Manager will read the notes you have recorded to create the epic and tasks.",
            )
            .build();

        let llm_messages: Vec<Message> = messages
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
            .collect();

        let request = CompletionRequest {
            messages: llm_messages,
            max_tokens: 1024,
            model: self.model.clone(),
            system: Some(po_user_session_system_prompt()),
            temperature: Some(0.3),
            top_p: None,
            stop_sequences: None,
            tools: Some(vec![ask_tool, note_tool, handoff_tool]),
            tool_choice: Some(ToolChoice::Auto),
            response_format: None,
        };

        log::info!("[PO:user_session] Calling LLM with model={}", self.model);
        let response = self.llm_client.complete(request).await?;

        let text = response
            .content
            .iter()
            .filter_map(|b| match b {
                llm_sdk::types::ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");

        if let Some(tool_calls) = response.tool_calls {
            let mut structured_questions: Vec<StructuredQuestion> = Vec::new();
            let mut handoff: Option<String> = None;

            for tool_call in tool_calls {
                match tool_call.name() {
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
                    }
                    "record_project_note" => {
                        let params: RecordProjectNoteParams =
                            match tool_call.parse_arguments().map_err(AgentError::Llm) {
                                Ok(p) => p,
                                Err(e) => {
                                    log::warn!("[PO] record_project_note parse error: {}", e);
                                    continue;
                                }
                            };
                        let topic = ProjectNoteTopic::from_str(&params.topic);
                        if let Err(e) = self
                            .note_storage
                            .add_note(
                                self.project_id,
                                topic,
                                params.title,
                                params.note,
                                Some(session_id),
                                params.replaces_note,
                            )
                            .await
                        {
                            log::warn!("[PO] record_project_note storage error: {}", e);
                        }
                    }
                    "hand_off_to_pm" => {
                        let params: HandOffToPmParams =
                            tool_call.parse_arguments().map_err(AgentError::Llm)?;
                        handoff = Some(params.final_message);
                    }
                    _ => {}
                }
            }

            // Handoff is processed after notes so all notes are persisted first.
            if let Some(final_message) = handoff {
                return Ok(PoSessionResult::HandedOff { final_message });
            }

            if !structured_questions.is_empty() {
                return Ok(PoSessionResult::Questions {
                    message: text,
                    questions: structured_questions,
                });
            }
        }

        if text.trim().is_empty() {
            Ok(PoSessionResult::Silent)
        } else {
            Ok(PoSessionResult::Text(text))
        }
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
