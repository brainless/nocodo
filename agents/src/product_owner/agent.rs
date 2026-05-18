use std::sync::Arc;

use llm_sdk::{
    client::LlmClient,
    tools::{Tool, ToolChoice},
    types::{CompletionRequest, Message, Role},
};

use super::prompts::po_user_session_system_prompt;
use super::tools::HandOffToPmParams;
use crate::{
    config::AgentConfig,
    error::AgentError,
    storage::{
        AgentStorage, AgentType, CommentStorage, QuestionKind, StructuredQuestion, TaskStorage,
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
        summary: String,
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
    model: String,
}

impl ProductOwnerAgent {
    pub fn new(
        _storage: Arc<dyn AgentStorage>,
        task_storage: Arc<dyn TaskStorage>,
        _comment_storage: Arc<dyn CommentStorage>,
        config: AgentConfig,
    ) -> Result<Self, AgentError> {
        let llm_client = crate::make_llm_client(&config)?;
        Ok(Self {
            llm_client,
            _storage,
            task_storage,
            _comment_storage,
            model: config.model,
        })
    }

    pub async fn respond_in_session(
        &self,
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

        let handoff_tool = Tool::from_type::<HandOffToPmParams>()
            .name("hand_off_to_pm")
            .description(
                "Call this when you have gathered enough requirements to proceed. \
                 Provide a friendly closing message for the user and a structured \
                 requirements brief for the Project Manager who will create the epic \
                 and development tasks.",
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
            tools: Some(vec![ask_tool, handoff_tool]),
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
                    "hand_off_to_pm" => {
                        let params: HandOffToPmParams =
                            tool_call.parse_arguments().map_err(AgentError::Llm)?;
                        return Ok(PoSessionResult::HandedOff {
                            final_message: params.final_message,
                            summary: params.summary,
                        });
                    }
                    _ => {}
                }
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
