use std::path::Path;
use std::sync::Arc;

use llm_sdk::{
    client::LlmClient,
    tools::{Tool, ToolChoice},
    types::{CompletionRequest, Message, Role},
};

use super::modes::{
    gap_clarification, loop_mode, persona_interview, project_naming, requirements_gathering,
};
use super::tools::{
    CompleteGapClarificationParams, CompletePersonaInterviewParams, CompleteRequirementsParams,
    FinalizeSessionParams, RecordProjectNoteParams, SetProjectNameParams, TriggerCodegenParams,
};
use crate::{
    config::AgentConfig,
    error::AgentError,
    spec_fact::{self, RecordSpecFactParams},
    storage::{
        spec_schemas::{PersonaNote, SpecNoteContent},
        AgentStorage, AgentType, CommentStorage, ProjectNoteStorage, ProjectNoteTopic,
        QuestionKind, StructuredQuestion, TaskStorage,
    },
    task_policy,
    user_input_tool::{InputType, RequestUserInputParams},
};

// ---------------------------------------------------------------------------
// Public result type
// ---------------------------------------------------------------------------

/// Which mode PO should run in. Selected deterministically by the backend,
/// never by the LLM via tool call.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PoMode {
    RequirementsGathering,
    ProjectNaming,
    PersonaInterview,
    GapClarification,
}

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
    /// Persona interview is complete; backend creates planning session and fires PM.
    PersonaInterviewComplete {
        closing_message: String,
    },
    /// Gap clarification is complete; backend re-runs Praxis Writer with updated notes.
    GapClarificationComplete {
        closing_message: String,
    },
    /// (Loop mode, Phase 3.5.) PO called `trigger_codegen` — the backend should
    /// run the checker (`cargo check`) and fire `run_loop` again with the raw
    /// output appended as context. `message` is any user-facing text PO sent
    /// alongside the tool call (e.g. "let me check that").
    LoopTriggerCodegen {
        message: Option<String>,
        action_description: String,
    },
    /// (Loop mode, Phase 3.5.) PO called `finalize_session` — the checker came
    /// back clean and the user confirmed there's nothing else to add.
    LoopFinalized {
        closing_message: String,
    },
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

    /// Construct a `ProductOwnerAgent` around a caller-supplied `LlmClient`,
    /// bypassing `AgentConfig`/`make_llm_client`. Used by tests to inject a
    /// scripted/mock client — see `agents/src/product_owner/agent.rs` tests
    /// module for the pattern.
    pub fn new_with_client(
        llm_client: Arc<dyn LlmClient>,
        storage: Arc<dyn AgentStorage>,
        task_storage: Arc<dyn TaskStorage>,
        comment_storage: Arc<dyn CommentStorage>,
        note_storage: Arc<dyn ProjectNoteStorage>,
        model: String,
        project_id: i64,
    ) -> Self {
        Self {
            llm_client,
            storage,
            task_storage,
            _comment_storage: comment_storage,
            note_storage,
            model,
            project_id,
        }
    }

    /// Run the PO agent in the selected mode.
    ///
    /// - `RequirementsGathering` — ask questions, record notes, signal completion.
    /// - `ProjectNaming` — derive a project name from conversation history.
    /// - `PersonaInterview` — deep-dive interview on each user persona.
    pub async fn respond_in_session(
        &self,
        session_id: i64,
        messages: Vec<(String, String)>,
        mode: PoMode,
    ) -> Result<PoSessionResult, AgentError> {
        match mode {
            PoMode::ProjectNaming => self.run_project_naming(messages).await,
            PoMode::PersonaInterview => self.run_persona_interview(session_id, messages).await,
            PoMode::GapClarification => self.run_gap_clarification(session_id, messages).await,
            PoMode::RequirementsGathering => {
                self.run_requirements_gathering(session_id, messages).await
            }
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
                 for the development team. Use replaces_note_id when you have a project_note id, \
                 or replaces_note when you only have exact old note text, to supersede an \
                 earlier note when the user clarifies or changes direction.",
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
                        let params: RecordProjectNoteParams = match tool_call
                            .parse_arguments()
                            .map_err(AgentError::Llm)
                        {
                            Ok(p) => p,
                            Err(e) => {
                                log::warn!("[PO] record_project_note parse error: {}", e);
                                tool_result_messages
                                    .push(Message::tool(tool_call.id(), format!("Error: {}", e)));
                                continue;
                            }
                        };
                        let topic = ProjectNoteTopic::from_str(&params.topic);

                        // When content_type is set, validate and wrap the structured note
                        let note_content = if let Some(ref ct) = params.content_type {
                            match ct.as_str() {
                                "persona" => {
                                    match serde_json::from_str::<PersonaNote>(&params.note) {
                                        Ok(persona) => {
                                            let wrapped = SpecNoteContent::Persona(persona);
                                            match serde_json::to_string(&wrapped) {
                                                Ok(json) => json,
                                                Err(e) => {
                                                    log::warn!(
                                                        "[PO] persona note serialization error: {}",
                                                        e
                                                    );
                                                    params.note.clone()
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            log::warn!(
                                                "[PO] persona note parse error (storing raw): {}",
                                                e
                                            );
                                            params.note.clone()
                                        }
                                    }
                                }
                                _ => {
                                    log::debug!("[PO] unknown content_type '{}', storing raw", ct);
                                    params.note.clone()
                                }
                            }
                        } else {
                            params.note.clone()
                        };

                        if let Err(e) = self
                            .note_storage
                            .add_note(
                                self.project_id,
                                topic,
                                note_content,
                                Some(session_id),
                                params.replaces_note_id,
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
        log::info!("[PO:project_naming] stop_reason={:?}", response.stop_reason);

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

    // -----------------------------------------------------------------------
    // Persona interview mode
    // -----------------------------------------------------------------------

    async fn run_persona_interview(
        &self,
        session_id: i64,
        messages: Vec<(String, String)>,
    ) -> Result<PoSessionResult, AgentError> {
        let ask_tool = Tool::from_type::<RequestUserInputParams>()
            .name("request_user_input")
            .description(
                "Ask the user a structured question with predefined choices. \
                 Use this for goals and pain_points questions — the UI renders \
                 checkboxes. You may call this tool multiple times in one turn \
                 when the questions are independent.",
            )
            .build();

        let note_tool = Tool::from_type::<RecordProjectNoteParams>()
            .name("record_project_note")
            .description(
                "Record a persona as a structured note. Use topic: \"context\" and \
                 content_type: \"persona\". The note field must be valid JSON matching \
                 the PersonaNote schema: id, name, description, goals, pain_points, \
                 provenance_message_id (nullable int FK to user message), and \
                 incomplete_reason (nullable string). Call this immediately after \
                 identifying each persona — partial data with incomplete_reason is \
                 better than no data at all. Use replaces_note_id to supersede a prior \
                 partial persona note with updated data when a project_note id is available.",
            )
            .build();

        let complete_tool = Tool::from_type::<CompletePersonaInterviewParams>()
            .name("complete_persona_interview")
            .description(
                "Signal that all personas have been documented. Provide a short, warm \
                 closing_message summarising what was covered. Call this when the user \
                 confirms there are no more user types to interview.",
            )
            .build();

        let tools = vec![ask_tool, note_tool, complete_tool];

        let mut llm_messages = build_llm_messages(&messages);

        const MAX_ITERATIONS: usize = 8;
        for iteration in 0..MAX_ITERATIONS {
            let request = CompletionRequest {
                messages: llm_messages.clone(),
                max_tokens: 1024,
                model: self.model.clone(),
                system: Some(persona_interview::system_prompt()),
                temperature: Some(0.3),
                top_p: None,
                stop_sequences: None,
                tools: Some(tools.clone()),
                tool_choice: Some(ToolChoice::Auto),
                response_format: None,
            };

            log::info!(
                "[PO:persona_interview] iteration={} model={} msg_count={}",
                iteration,
                self.model,
                request.messages.len()
            );
            let response = self.llm_client.complete(request).await?;
            log::info!(
                "[PO:persona_interview] iteration={} stop_reason={:?}",
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
                    "[PO:persona_interview] tool id={} name={}",
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
                        let params: RecordProjectNoteParams = match tool_call
                            .parse_arguments()
                            .map_err(AgentError::Llm)
                        {
                            Ok(p) => p,
                            Err(e) => {
                                log::warn!(
                                    "[PO:persona_interview] record_project_note parse error: {}",
                                    e
                                );
                                tool_result_messages
                                    .push(Message::tool(tool_call.id(), format!("Error: {}", e)));
                                continue;
                            }
                        };
                        let topic = ProjectNoteTopic::from_str(&params.topic);

                        let note_content = if let Some(ref ct) = params.content_type {
                            if ct == "persona" {
                                match serde_json::from_str::<PersonaNote>(&params.note) {
                                    Ok(persona) => {
                                        match serde_json::to_string(&SpecNoteContent::Persona(
                                            persona,
                                        )) {
                                            Ok(json) => json,
                                            Err(e) => {
                                                log::warn!(
                                                    "[PO:persona_interview] serialization error: {}",
                                                    e
                                                );
                                                params.note.clone()
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        log::warn!(
                                            "[PO:persona_interview] persona parse error (storing raw): {}",
                                            e
                                        );
                                        params.note.clone()
                                    }
                                }
                            } else {
                                params.note.clone()
                            }
                        } else {
                            params.note.clone()
                        };

                        if let Err(e) = self
                            .note_storage
                            .add_note(
                                self.project_id,
                                topic,
                                note_content,
                                Some(session_id),
                                params.replaces_note_id,
                                params.replaces_note,
                            )
                            .await
                        {
                            log::warn!(
                                "[PO:persona_interview] record_project_note storage error: {}",
                                e
                            );
                        }
                        "Note recorded".to_string()
                    }
                    "complete_persona_interview" => {
                        let params: CompletePersonaInterviewParams =
                            tool_call.parse_arguments().map_err(AgentError::Llm)?;
                        completion = Some(params.closing_message);
                        "Persona interview complete".to_string()
                    }
                    other => {
                        log::warn!("[PO:persona_interview] unknown tool: {}", other);
                        "Unknown tool".to_string()
                    }
                };

                tool_result_messages.push(Message::tool(tool_call.id(), tool_result));
            }

            log::info!(
                "[PO:persona_interview] iteration={} complete={} questions={}",
                iteration,
                completion.is_some(),
                structured_questions.len()
            );

            if let Some(closing_message) = completion {
                return Ok(PoSessionResult::PersonaInterviewComplete { closing_message });
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
                "[PO:persona_interview] only notes recorded, looping (msg_count={})",
                llm_messages.len()
            );
        }

        log::warn!(
            "[PO:persona_interview] hit MAX_ITERATIONS={} without user-facing result",
            MAX_ITERATIONS
        );
        Ok(PoSessionResult::Silent)
    }

    // -----------------------------------------------------------------------
    // Gap clarification mode
    // -----------------------------------------------------------------------

    async fn run_gap_clarification(
        &self,
        session_id: i64,
        messages: Vec<(String, String)>,
    ) -> Result<PoSessionResult, AgentError> {
        let ask_tool = Tool::from_type::<RequestUserInputParams>()
            .name("request_user_input")
            .description(
                "Ask the user a structured question about missing persona data. \
                 Use this for goals and pain_points questions — the UI renders \
                 checkboxes. You may call this tool multiple times in one turn \
                 when the questions are independent.",
            )
            .build();

        let note_tool = Tool::from_type::<RecordProjectNoteParams>()
            .name("record_project_note")
            .description(
                "Record an updated persona as a structured note. Use topic: \"context\" \
                 and content_type: \"persona\". The note field must be valid JSON matching \
                 the PersonaNote schema. Use replaces_note_id to supersede the old persona \
                 note with the updated data. Set incomplete_reason to null when the \
                 persona is now complete.",
            )
            .build();

        let complete_tool = Tool::from_type::<CompleteGapClarificationParams>()
            .name("complete_gap_clarification")
            .description(
                "Signal that all gaps have been addressed (filled or user declined). \
                 Provide a short closing_message summarising what was clarified.",
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
                system: Some(gap_clarification::system_prompt()),
                temperature: Some(0.3),
                top_p: None,
                stop_sequences: None,
                tools: Some(tools.clone()),
                tool_choice: Some(ToolChoice::Auto),
                response_format: None,
            };

            log::info!(
                "[PO:gap_clarification] iteration={} model={} msg_count={}",
                iteration,
                self.model,
                request.messages.len()
            );
            let response = self.llm_client.complete(request).await?;
            log::info!(
                "[PO:gap_clarification] iteration={} stop_reason={:?}",
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
                    "[PO:gap_clarification] tool id={} name={}",
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
                        let params: RecordProjectNoteParams = match tool_call
                            .parse_arguments()
                            .map_err(AgentError::Llm)
                        {
                            Ok(p) => p,
                            Err(e) => {
                                log::warn!(
                                    "[PO:gap_clarification] record_project_note parse error: {}",
                                    e
                                );
                                tool_result_messages
                                    .push(Message::tool(tool_call.id(), format!("Error: {}", e)));
                                continue;
                            }
                        };
                        let topic = ProjectNoteTopic::from_str(&params.topic);

                        let note_content = if let Some(ref ct) = params.content_type {
                            if ct == "persona" {
                                match serde_json::from_str::<PersonaNote>(&params.note) {
                                    Ok(persona) => {
                                        match serde_json::to_string(&SpecNoteContent::Persona(
                                            persona,
                                        )) {
                                            Ok(json) => json,
                                            Err(e) => {
                                                log::warn!(
                                                    "[PO:gap_clarification] serialization error: {}",
                                                    e
                                                );
                                                params.note.clone()
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        log::warn!(
                                            "[PO:gap_clarification] persona parse error (storing raw): {}",
                                            e
                                        );
                                        params.note.clone()
                                    }
                                }
                            } else {
                                params.note.clone()
                            }
                        } else {
                            params.note.clone()
                        };

                        if let Err(e) = self
                            .note_storage
                            .add_note(
                                self.project_id,
                                topic,
                                note_content,
                                Some(session_id),
                                params.replaces_note_id,
                                params.replaces_note,
                            )
                            .await
                        {
                            log::warn!(
                                "[PO:gap_clarification] record_project_note storage error: {}",
                                e
                            );
                        }
                        "Note recorded".to_string()
                    }
                    "complete_gap_clarification" => {
                        let params: CompleteGapClarificationParams =
                            tool_call.parse_arguments().map_err(AgentError::Llm)?;
                        completion = Some(params.closing_message);
                        "Gap clarification complete".to_string()
                    }
                    other => {
                        log::warn!("[PO:gap_clarification] unknown tool: {}", other);
                        "Unknown tool".to_string()
                    }
                };

                tool_result_messages.push(Message::tool(tool_call.id(), tool_result));
            }

            log::info!(
                "[PO:gap_clarification] iteration={} complete={} questions={}",
                iteration,
                completion.is_some(),
                structured_questions.len()
            );

            if let Some(closing_message) = completion {
                return Ok(PoSessionResult::GapClarificationComplete { closing_message });
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
                "[PO:gap_clarification] only notes recorded, looping (msg_count={})",
                llm_messages.len()
            );
        }

        log::warn!(
            "[PO:gap_clarification] hit MAX_ITERATIONS={} without user-facing result",
            MAX_ITERATIONS
        );
        Ok(PoSessionResult::Silent)
    }

    // -----------------------------------------------------------------------
    // Loop mode (Praxis v1 Phase 3.5) — unified PO<->user loop
    // -----------------------------------------------------------------------

    /// Run one turn of the unified PO loop (MILESTONE_PRAXIS_v1.md §8).
    ///
    /// Unlike the legacy modes above, this takes `project_root` directly — it
    /// writes spec facts into the managed project via `record_spec_fact` — and
    /// an optional `checker_context`: raw `cargo check` stderr from a prior
    /// `trigger_codegen` call, appended to the conversation so PO can turn
    /// compiler errors into follow-up questions for the user (no `Gap` struct
    /// parsing — see MILESTONE_PRAXIS_v1.md §7.3 point 1).
    pub async fn run_loop(
        &self,
        session_id: i64,
        project_root: &Path,
        messages: Vec<(String, String)>,
        checker_context: Option<String>,
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

        let record_tool = Tool::from_type::<RecordSpecFactParams>()
            .name("record_spec_fact")
            .description(
                "Write a fact into the project as real nocodo_praxis Rust code. \
                 Phase 3.5 only supports fact_type: \"persona\" — data must match \
                 the PersonaNote shape: id, name, description, goals, pain_points, \
                 provenance_message_id (nullable int), incomplete_reason (nullable \
                 string). The backend serializes it deterministically — you never \
                 write Rust yourself. Call this as soon as you have an id, name, \
                 and description for a persona; partial data with incomplete_reason \
                 set is fine.",
            )
            .build();

        let codegen_tool = Tool::from_type::<TriggerCodegenParams>()
            .name("trigger_codegen")
            .description(
                "Ask the backend to run the checker (cargo check) now on what's \
                 been recorded so far. The result comes back as your next turn's \
                 context — raw compiler output for you to interpret into a \
                 plain-language follow-up question. Call this after recording one \
                 or a few related facts, not after every single one.",
            )
            .build();

        let finalize_tool = Tool::from_type::<FinalizeSessionParams>()
            .name("finalize_session")
            .description(
                "Call this only when the most recent checker run came back clean \
                 (no compiler errors) and the user confirms there's nothing else \
                 to add. Ends the loop.",
            )
            .build();

        let tools = vec![ask_tool, record_tool, codegen_tool, finalize_tool];

        let mut llm_messages = build_llm_messages(&messages);
        if let Some(ctx) = checker_context {
            llm_messages.push(Message {
                role: Role::User,
                content: vec![llm_sdk::types::ContentBlock::Text {
                    text: format!(
                        "[System: checker output from your last trigger_codegen call]\n\n{}",
                        ctx
                    ),
                }],
                tool_call_id: None,
                tool_name: None,
            });
        }

        const MAX_ITERATIONS: usize = 6;
        for iteration in 0..MAX_ITERATIONS {
            let request = CompletionRequest {
                messages: llm_messages.clone(),
                max_tokens: 1024,
                model: self.model.clone(),
                system: Some(loop_mode::system_prompt()),
                temperature: Some(0.3),
                top_p: None,
                stop_sequences: None,
                tools: Some(tools.clone()),
                tool_choice: Some(ToolChoice::Auto),
                response_format: None,
            };

            log::info!(
                "[PO:loop] iteration={} model={} msg_count={}",
                iteration,
                self.model,
                request.messages.len()
            );
            let response = self.llm_client.complete(request).await?;
            log::info!(
                "[PO:loop] iteration={} stop_reason={:?}",
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
            let mut codegen: Option<String> = None;
            let mut finalized: Option<String> = None;
            let mut tool_result_messages: Vec<Message> = Vec::new();
            let mut assistant_tool_msgs: Vec<Message> = Vec::new();

            for tool_call in &tool_calls {
                log::info!(
                    "[PO:loop] tool id={} name={}",
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
                    "record_spec_fact" => {
                        let params: RecordSpecFactParams = match tool_call
                            .parse_arguments()
                            .map_err(AgentError::Llm)
                        {
                            Ok(p) => p,
                            Err(e) => {
                                log::warn!("[PO:loop] record_spec_fact parse error: {}", e);
                                tool_result_messages
                                    .push(Message::tool(tool_call.id(), format!("Error: {}", e)));
                                continue;
                            }
                        };
                        match spec_fact::record_spec_fact(
                            project_root,
                            &session_id.to_string(),
                            params,
                        ) {
                            Ok(result) => format!(
                                "Recorded fact '{}' -> {}",
                                result.identifier, result.file_path
                            ),
                            Err(e) => {
                                log::warn!("[PO:loop] record_spec_fact error: {}", e);
                                format!("Error recording fact: {}", e)
                            }
                        }
                    }
                    "trigger_codegen" => {
                        let params: TriggerCodegenParams =
                            tool_call.parse_arguments().map_err(AgentError::Llm)?;
                        codegen = Some(params.action_description);
                        "Codegen/check triggered".to_string()
                    }
                    "finalize_session" => {
                        let params: FinalizeSessionParams =
                            tool_call.parse_arguments().map_err(AgentError::Llm)?;
                        finalized = Some(params.closing_message);
                        "Session finalized".to_string()
                    }
                    other => {
                        log::warn!("[PO:loop] unknown tool: {}", other);
                        "Unknown tool".to_string()
                    }
                };

                tool_result_messages.push(Message::tool(tool_call.id(), tool_result));
            }

            log::info!(
                "[PO:loop] iteration={} finalized={} codegen={} questions={}",
                iteration,
                finalized.is_some(),
                codegen.is_some(),
                structured_questions.len()
            );

            if let Some(closing_message) = finalized {
                return Ok(PoSessionResult::LoopFinalized { closing_message });
            }

            if let Some(action_description) = codegen {
                let message = if text.trim().is_empty() {
                    None
                } else {
                    Some(text)
                };
                return Ok(PoSessionResult::LoopTriggerCodegen {
                    message,
                    action_description,
                });
            }

            if !structured_questions.is_empty() {
                return Ok(PoSessionResult::Questions {
                    message: text,
                    questions: structured_questions,
                });
            }

            // Only record_spec_fact calls: feed tool results back and loop.
            llm_messages.extend(assistant_tool_msgs);
            llm_messages.extend(tool_result_messages);
            log::info!(
                "[PO:loop] only facts recorded, looping (msg_count={})",
                llm_messages.len()
            );
        }

        log::warn!(
            "[PO:loop] hit MAX_ITERATIONS={} without user-facing result",
            MAX_ITERATIONS
        );
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

// ---------------------------------------------------------------------------
// Tests — loop mode (Praxis v1 Phase 3.5)
//
// There's no pre-existing mock-LLM test pattern in this crate to follow (PO,
// like every other agent here, was previously only exercised via the running
// backend). `ScriptedLlmClient` below is a minimal `LlmClient` impl that
// replays a fixed sequence of `CompletionResponse`s — enough to drive
// `run_loop` through one full record_spec_fact -> trigger_codegen iteration,
// a checker-context -> follow-up-question turn, and a finalize_session turn,
// without a network call or real model.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod loop_tests {
    use super::*;
    use llm_sdk::{
        error::LlmError,
        tools::ToolCall,
        types::{ContentBlock, CompletionResponse, Usage},
    };
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Mutex as StdMutex;

    /// Replays a fixed queue of canned responses, one per `complete()` call.
    /// Panics if the queue runs dry — that means the test under-scripted the
    /// number of LLM turns `run_loop` actually needs.
    struct ScriptedLlmClient {
        responses: StdMutex<std::collections::VecDeque<CompletionResponse>>,
    }

    impl ScriptedLlmClient {
        fn new(responses: Vec<CompletionResponse>) -> Self {
            Self {
                responses: StdMutex::new(responses.into()),
            }
        }
    }

    #[async_trait::async_trait]
    impl LlmClient for ScriptedLlmClient {
        async fn complete(
            &self,
            _request: CompletionRequest,
        ) -> Result<CompletionResponse, LlmError> {
            self.responses
                .lock()
                .unwrap()
                .pop_front()
                .ok_or_else(|| LlmError::Api {
                    status: 0,
                    message: "ScriptedLlmClient: no more scripted responses".to_string(),
                })
        }

        fn provider_name(&self) -> &str {
            "scripted-test"
        }

        fn model_name(&self) -> &str {
            "scripted-test-model"
        }
    }

    fn usage() -> Usage {
        Usage {
            input_tokens: 0,
            output_tokens: 0,
        }
    }

    fn tool_call_response(calls: Vec<(&str, &str, serde_json::Value)>) -> CompletionResponse {
        CompletionResponse {
            content: vec![],
            role: Role::Assistant,
            usage: usage(),
            stop_reason: Some("tool_use".to_string()),
            tool_calls: Some(
                calls
                    .into_iter()
                    .map(|(id, name, args)| ToolCall::new(id.to_string(), name.to_string(), args))
                    .collect(),
            ),
        }
    }

    /// A fresh scratch project root per test, so parallel tests don't collide
    /// writing `backend/src/praxis/specs/personas.rs`. Mirrors the pattern in
    /// `agents/src/spec_fact/mod.rs` tests.
    fn scratch_project_root() -> std::path::PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "nocodo-agents-po-loop-test-{}-{}",
            std::process::id(),
            n
        ));
        std::fs::create_dir_all(&dir).expect("create scratch dir");
        dir
    }

    /// Build a `ProductOwnerAgent` around a scripted client. The storage
    /// trait objects point at a fresh throwaway sqlite file — `run_loop`'s
    /// tools (`record_spec_fact`, `request_user_input`, `trigger_codegen`,
    /// `finalize_session`) never touch `AgentStorage`/`TaskStorage`/
    /// `CommentStorage`/`ProjectNoteStorage`, so the file doesn't need any
    /// schema — opening a connection to it is enough to satisfy the
    /// constructor.
    fn test_po(client: Arc<dyn LlmClient>) -> ProductOwnerAgent {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let db_path = std::env::temp_dir()
            .join(format!(
                "nocodo-agents-po-loop-test-db-{}-{}.sqlite",
                std::process::id(),
                n
            ))
            .to_string_lossy()
            .to_string();

        let storage: Arc<dyn AgentStorage> =
            Arc::new(crate::storage::sqlite::SqliteAgentStorage::open(&db_path).unwrap());
        let task_storage: Arc<dyn TaskStorage> =
            Arc::new(crate::storage::sqlite::SqliteTaskStorage::open(&db_path).unwrap());
        let comment_storage: Arc<dyn CommentStorage> =
            Arc::new(crate::storage::sqlite::SqliteCommentStorage::open(&db_path).unwrap());
        let note_storage: Arc<dyn ProjectNoteStorage> =
            Arc::new(crate::storage::sqlite::SqliteProjectNoteStorage::open(&db_path).unwrap());

        ProductOwnerAgent::new_with_client(
            client,
            storage,
            task_storage,
            comment_storage,
            note_storage,
            "scripted-test-model".to_string(),
            1,
        )
    }

    fn persona_args() -> serde_json::Value {
        serde_json::json!({
            "fact_type": "persona",
            "data": {
                "id": "admin",
                "name": "Team Admin",
                "description": "Manages the team and assigns tasks",
                "goals": ["Assign tasks to team members"],
                "pain_points": ["No visibility into who's doing what"],
                "provenance_message_id": null,
                "incomplete_reason": null,
            }
        })
    }

    /// One record_spec_fact call, then a trigger_codegen call in a second
    /// turn — proves the wiring end-to-end for one loop iteration: PO writes
    /// a persona to disk via the real (non-LLM) serializer, then signals the
    /// backend to run the checker.
    #[tokio::test]
    async fn run_loop_records_persona_then_triggers_codegen() {
        let root = scratch_project_root();
        let client = Arc::new(ScriptedLlmClient::new(vec![
            tool_call_response(vec![("call-1", "record_spec_fact", persona_args())]),
            tool_call_response(vec![(
                "call-2",
                "trigger_codegen",
                serde_json::json!({ "action_description": "recorded the admin persona" }),
            )]),
        ]));
        let po = test_po(client);

        let result = po
            .run_loop(
                42,
                &root,
                vec![(
                    "user".to_string(),
                    "Admins manage the team and assign tasks.".to_string(),
                )],
                None,
            )
            .await
            .expect("run_loop should succeed");

        match result {
            PoSessionResult::LoopTriggerCodegen {
                action_description, ..
            } => {
                assert_eq!(action_description, "recorded the admin persona");
            }
            other => panic!("expected LoopTriggerCodegen, got {:?}", other),
        }

        // record_spec_fact actually wrote real nocodo_praxis Rust source —
        // this is the load-bearing part of the test, not the tool plumbing.
        let written =
            std::fs::read_to_string(root.join("backend/src/praxis/specs/personas.rs")).unwrap();
        assert!(written.contains("pub const PERSONA_ADMIN: PersonaId = PersonaId(\"admin\");"));
        assert!(written.contains("CONV_42_ADMIN"));
    }

    /// When the backend hands back `cargo check` stderr as `checker_context`,
    /// PO should read it and turn it into a follow-up question rather than
    /// echoing raw compiler output — proving the gap-context wiring works for
    /// one iteration (full loop-to-completion is Phase 7, not this test).
    #[tokio::test]
    async fn run_loop_with_checker_context_produces_followup_question() {
        let root = scratch_project_root();
        let client = Arc::new(ScriptedLlmClient::new(vec![tool_call_response(vec![(
            "call-1",
            "request_user_input",
            serde_json::json!({
                "question": "You mentioned an admin role — can you tell me a bit about them?",
                "input_type": "single_choice",
                "options": ["They manage the team", "Something else"],
            }),
        )])]));
        let po = test_po(client);

        let checker_output =
            "error[E0425]: cannot find value `ROLE_ADMIN` in this scope".to_string();
        let result = po
            .run_loop(
                42,
                &root,
                vec![("user".to_string(), "yes, go ahead and check".to_string())],
                Some(checker_output),
            )
            .await
            .expect("run_loop should succeed");

        match result {
            PoSessionResult::Questions { questions, .. } => {
                assert_eq!(questions.len(), 1);
                assert!(questions[0].question.contains("admin"));
            }
            other => panic!("expected Questions, got {:?}", other),
        }
    }

    /// `finalize_session` should end the loop immediately with the closing
    /// message, without requiring further LLM turns.
    #[tokio::test]
    async fn run_loop_finalize_session_ends_loop() {
        let root = scratch_project_root();
        let client = Arc::new(ScriptedLlmClient::new(vec![tool_call_response(vec![(
            "call-1",
            "finalize_session",
            serde_json::json!({ "closing_message": "All set — thanks for walking me through it!" }),
        )])]));
        let po = test_po(client);

        let result = po
            .run_loop(
                42,
                &root,
                vec![("user".to_string(), "That's everything.".to_string())],
                None,
            )
            .await
            .expect("run_loop should succeed");

        match result {
            PoSessionResult::LoopFinalized { closing_message } => {
                assert_eq!(
                    closing_message,
                    "All set — thanks for walking me through it!"
                );
            }
            other => panic!("expected LoopFinalized, got {:?}", other),
        }
    }

    // -----------------------------------------------------------------------
    // Phase 7 end-to-end test (MILESTONE_PRAXIS_v1.md §10 Phase 7, §2.1)
    //
    // Walks the todo-app scenario from §2.1 through the full loop: PO records
    // 2 personas, 3 permissions, 2 roles, 4 states (with the 2 transitions
    // §2.1 exercises: todo->in_progress via `assign_task`,
    // in_progress->done via `complete_task`), and 1 entity, all via the real
    // (non-LLM) `record_spec_fact` serializers — then asserts the static
    // checker (`crate::static_checker::run_static_checks`) finds zero gaps
    // once the two transition guards and their model-type dependencies exist,
    // and that a real `cargo check` of the assembled project succeeds.
    //
    // One deliberate substitution: `RustEngineerAgent::run_state_transition`
    // (the mode that would normally generate the two guard structs below) is
    // NOT invoked here. Unlike `ProductOwnerAgent` — which has
    // `new_with_client` specifically so tests can inject a `ScriptedLlmClient`
    // — `RustEngineerAgent` holds a concrete `LlamaCppClient`, not an
    // `Arc<dyn LlmClient>`, so it isn't mockable without a source change to
    // that agent. Per this test's task brief: "the scripted client should
    // return a hand-written, syntactically valid `TransitionGuard` impl
    // string standing in for what a real RustEngineer LLM call would
    // produce" — ASSIGN_TASK_GUARD/COMPLETE_TASK_GUARD below are exactly
    // that stand-in, written directly to disk via
    // `crate::code_writer::write_transition` (the same deterministic
    // post-processing sink `run_state_transition` itself writes through).
    // This is a wiring test, not a model-quality test: the thing being
    // proven is that spec facts + guard files + model structs compose into
    // zero static-checker gaps and a compiling crate, not that an LLM can
    // write a good guard.
    //
    // Similarly, no `diesel_model_struct` mode exists in this codebase (see
    // Phase 7 report) and the static checker never gates on model structs
    // existing (see `static_checker::run_static_checks` — it only reads
    // `backend/src/praxis/{specs,transitions}/*.rs`), so MODELS_RS below is
    // a hand-written stand-in for what that (not-yet-built) mode would
    // produce, needed only so the real `cargo check` at the end of this test
    // has `User`/`Task` types to resolve.

    const ASSIGN_TASK_GUARD: &str = r#"use crate::models::{Task, User};
use nocodo_praxis::auth::{HasRole, RoleId};
use nocodo_praxis::entity::EntityId;
use nocodo_praxis::statemachine::{HasState, StateId, TransitionError, TransitionGuard};

pub struct AssignTask;

impl TransitionGuard<(User, Task, User), Task> for AssignTask {
    const ENTITY: EntityId = EntityId("task");
    const FROM: StateId = StateId("todo");
    const TO: StateId = StateId("in_progress");

    fn guard((actor, task, assignee): (User, Task, User)) -> Result<Task, TransitionError> {
        if !actor.has_role(RoleId("admin")) {
            return Err(TransitionError::Forbidden {
                required: RoleId("admin"),
                found: Some(actor.role()),
            });
        }
        if !task.is_in_state(StateId("todo")) {
            return Err(TransitionError::InvalidState {
                entity: EntityId("task"),
                expected: StateId("todo"),
                found: Some(task.state()),
            });
        }
        Ok(Task {
            assignee: Some(assignee.id),
            status: StateId("in_progress"),
            ..task
        })
    }
}
"#;

    const COMPLETE_TASK_GUARD: &str = r#"use crate::models::{Task, User};
use nocodo_praxis::entity::EntityId;
use nocodo_praxis::statemachine::{HasState, StateId, TransitionError, TransitionGuard};

pub struct CompleteTask;

impl TransitionGuard<(User, Task), Task> for CompleteTask {
    const ENTITY: EntityId = EntityId("task");
    const FROM: StateId = StateId("in_progress");
    const TO: StateId = StateId("done");

    fn guard((actor, task): (User, Task)) -> Result<Task, TransitionError> {
        if !task.is_in_state(StateId("in_progress")) {
            return Err(TransitionError::InvalidState {
                entity: EntityId("task"),
                expected: StateId("in_progress"),
                found: Some(task.state()),
            });
        }
        if task.assignee != Some(actor.id) {
            return Err(TransitionError::InvariantViolation {
                invariant: "only the assignee can complete their own task",
            });
        }
        Ok(Task {
            status: StateId("done"),
            ..task
        })
    }
}
"#;

    const MODELS_RS: &str = r#"use nocodo_praxis::auth::{HasRole, RoleId};
use nocodo_praxis::statemachine::{HasState, StateId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct User {
    pub id: i64,
    pub role: RoleId,
}

impl HasRole for User {
    fn role(&self) -> RoleId {
        self.role
    }
}

#[derive(Debug, Clone)]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub assignee: Option<i64>,
    pub status: StateId,
}

impl HasState for Task {
    fn state(&self) -> StateId {
        self.status
    }
}
"#;

    fn persona_fact_args(
        id: &str,
        name: &str,
        description: &str,
        goals: &[&str],
        pain_points: &[&str],
    ) -> serde_json::Value {
        serde_json::json!({
            "fact_type": "persona",
            "data": {
                "id": id,
                "name": name,
                "description": description,
                "goals": goals,
                "pain_points": pain_points,
                "provenance_message_id": null,
                "incomplete_reason": null,
            }
        })
    }

    fn permission_fact_args(id: &str, description: &str) -> serde_json::Value {
        serde_json::json!({
            "fact_type": "permission",
            "data": { "id": id, "description": description }
        })
    }

    fn role_fact_args(
        id: &str,
        description: &str,
        permission_ids: &[&str],
        persona_ids: &[&str],
    ) -> serde_json::Value {
        serde_json::json!({
            "fact_type": "role",
            "data": {
                "id": id,
                "description": description,
                "permission_ids": permission_ids,
                "persona_ids": persona_ids,
                "semantics": "flat",
            }
        })
    }

    fn state_fact_args(
        id: &str,
        description: &str,
        transitions: Vec<(&str, Vec<&str>, &str)>,
    ) -> serde_json::Value {
        let transitions_json: Vec<serde_json::Value> = transitions
            .into_iter()
            .map(|(to, role_ids, condition)| {
                serde_json::json!({ "to": to, "role_ids": role_ids, "condition": condition })
            })
            .collect();
        serde_json::json!({
            "fact_type": "state",
            "data": {
                "id": id,
                "description": description,
                "transitions": transitions_json,
            }
        })
    }

    fn entity_fact_args() -> serde_json::Value {
        serde_json::json!({
            "fact_type": "entity",
            "data": {
                "id": "task",
                "description": "A unit of work that can be assigned to a team member and tracked to completion",
                "fields": [
                    { "name": "title", "description": "Short name for the task", "invariant": "max 200 characters" },
                    { "name": "assignee", "description": "Which team member is responsible", "invariant": "optional, references a user id" },
                ],
                "state_ids": ["todo", "in_progress", "done", "cancelled"],
            }
        })
    }

    /// Recursively copy a directory tree, skipping any `target/` directory
    /// (build artifacts — not needed and potentially huge).
    fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) {
        std::fs::create_dir_all(dst).expect("create dest dir");
        for entry in std::fs::read_dir(src).expect("read src dir") {
            let entry = entry.expect("dir entry");
            let file_type = entry.file_type().expect("file type");
            let dest_path = dst.join(entry.file_name());
            if file_type.is_dir() {
                if entry.file_name() == "target" {
                    continue;
                }
                copy_dir_recursive(&entry.path(), &dest_path);
            } else if file_type.is_file() {
                std::fs::copy(entry.path(), &dest_path).expect("copy file");
            }
        }
    }

    /// Assemble a real, standalone Cargo crate under `root/backend` that a
    /// `cargo check` subprocess can actually build: a vendored copy of
    /// `nocodo_praxis` (copied into `root/nocodo_praxis` so the path
    /// dependency's workspace-root search doesn't walk up into *this* repo's
    /// workspace — `nocodo_praxis` itself declares no `[workspace]`, so
    /// without vendoring, cargo would treat the temp crate as an orphan
    /// member of the real nocodo workspace and fail) plus a `main.rs` that
    /// declares the `models` and `praxis` modules `record_spec_fact` /
    /// `write_transition` / this test have already written under
    /// `backend/src/`.
    fn assemble_cargo_project(root: &std::path::Path) {
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = manifest_dir
            .parent()
            .expect("agents/ crate has a parent directory (workspace root)");
        let praxis_src = workspace_root.join("nocodo_praxis");
        copy_dir_recursive(&praxis_src, &root.join("nocodo_praxis"));

        std::fs::write(
            root.join("backend/Cargo.toml"),
            r#"[package]
name = "todo-app-backend"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "todo-app-backend"
path = "src/main.rs"

[dependencies]
nocodo_praxis = { path = "../nocodo_praxis" }
"#,
        )
        .expect("write backend Cargo.toml");

        std::fs::write(
            root.join("backend/src/main.rs"),
            "mod models;\nmod praxis;\n\nfn main() {}\n",
        )
        .expect("write backend main.rs");
    }

    #[tokio::test]
    async fn full_todo_app_loop_reaches_zero_gaps_and_compiles() {
        let root = scratch_project_root();

        let client = Arc::new(ScriptedLlmClient::new(vec![
            // Turn 1: both personas + all three permissions.
            tool_call_response(vec![
                (
                    "t1-1",
                    "record_spec_fact",
                    persona_fact_args(
                        "admin",
                        "Team Admin",
                        "Manages the team and assigns tasks",
                        &["Assign work to the team"],
                        &["No visibility into task status"],
                    ),
                ),
                (
                    "t1-2",
                    "record_spec_fact",
                    persona_fact_args(
                        "member",
                        "Team Member",
                        "Does the assigned work",
                        &["Complete assigned tasks"],
                        &["Unclear what to work on next"],
                    ),
                ),
                (
                    "t1-3",
                    "record_spec_fact",
                    permission_fact_args("create_task", "Create a new task"),
                ),
                (
                    "t1-4",
                    "record_spec_fact",
                    permission_fact_args("assign_task", "Assign a task to a team member"),
                ),
                (
                    "t1-5",
                    "record_spec_fact",
                    permission_fact_args("complete_task", "Mark an assigned task as done"),
                ),
            ]),
            // Turn 2: both roles.
            tool_call_response(vec![
                (
                    "t2-1",
                    "record_spec_fact",
                    role_fact_args(
                        "admin",
                        "Team admin",
                        &["create_task", "assign_task"],
                        &["admin"],
                    ),
                ),
                (
                    "t2-2",
                    "record_spec_fact",
                    role_fact_args("member", "Team member", &["complete_task"], &["member"]),
                ),
            ]),
            // Turn 3: all four states (two with transitions, two terminal).
            tool_call_response(vec![
                (
                    "t3-1",
                    "record_spec_fact",
                    state_fact_args(
                        "todo",
                        "Task created, not yet started",
                        vec![("in_progress", vec!["admin"], "always")],
                    ),
                ),
                (
                    "t3-2",
                    "record_spec_fact",
                    state_fact_args(
                        "in_progress",
                        "Task is being worked on",
                        vec![("done", vec!["member"], "only_if_assigned_to_self")],
                    ),
                ),
                (
                    "t3-3",
                    "record_spec_fact",
                    state_fact_args("done", "Task is complete", vec![]),
                ),
                (
                    "t3-4",
                    "record_spec_fact",
                    state_fact_args(
                        "cancelled",
                        "Task was cancelled before completion",
                        vec![],
                    ),
                ),
            ]),
            // Turn 4: the task entity.
            tool_call_response(vec![("t4-1", "record_spec_fact", entity_fact_args())]),
            // Turn 5: trigger_codegen for the transition guards.
            tool_call_response(vec![(
                "t5-1",
                "trigger_codegen",
                serde_json::json!({
                    "action_description": "generate the assign_task and complete_task transition guards for the task entity"
                }),
            )]),
        ]));

        let po = test_po(client);
        let result = po
            .run_loop(
                99,
                &root,
                vec![(
                    "user".to_string(),
                    "I want to build a todo app for my team.".to_string(),
                )],
                None,
            )
            .await
            .expect("run_loop should succeed");

        match result {
            PoSessionResult::LoopTriggerCodegen {
                action_description, ..
            } => {
                assert!(action_description.contains("transition guard"));
            }
            other => panic!(
                "expected LoopTriggerCodegen after recording the full todo app spec, got {:?}",
                other
            ),
        }

        // RustEngineer `state_transition` stand-in (see this test's leading
        // doc comment for why) — writes through the same deterministic sink
        // `run_state_transition` itself uses.
        crate::code_writer::write_transition(&root, "assign_task", ASSIGN_TASK_GUARD)
            .expect("write assign_task guard");
        crate::code_writer::write_transition(&root, "complete_task", COMPLETE_TASK_GUARD)
            .expect("write complete_task guard");

        // Hand-written model-struct stand-in (no `diesel_model_struct` mode
        // exists yet — see this test's leading doc comment).
        std::fs::create_dir_all(root.join("backend/src/models"))
            .expect("create backend/src/models");
        std::fs::write(root.join("backend/src/models/mod.rs"), MODELS_RS)
            .expect("write backend/src/models/mod.rs");

        // The static checker is the actual Phase 7 exit criterion: zero gaps
        // across C1 (unresolved), C2 (referential integrity), C3 (guard
        // coverage), C4 (spec completeness).
        let gaps = crate::static_checker::run_static_checks(&root);
        assert!(
            gaps.is_empty(),
            "expected zero static checker gaps, got: {gaps:?}"
        );

        // Real compile check (not a mock) against the assembled project —
        // MILESTONE_PRAXIS_v1.md §11 risk 5: the static checker doesn't run
        // rustc, so this is the independent proof that generated code
        // actually compiles.
        assemble_cargo_project(&root);
        let outcome =
            crate::checker::run_cargo_check(&root).expect("cargo check should run without error");
        assert!(
            outcome.success,
            "cargo check failed against the assembled todo-app project:\n{}",
            outcome.output
        );
    }

    /// A plain-text response (no tool calls) should surface as `Text`, same
    /// as the legacy modes — sanity check that loop mode doesn't require a
    /// tool call on every turn.
    #[tokio::test]
    async fn run_loop_plain_text_response_is_passed_through() {
        let root = scratch_project_root();
        let client = Arc::new(ScriptedLlmClient::new(vec![CompletionResponse {
            content: vec![ContentBlock::Text {
                text: "Tell me more about who uses this system.".to_string(),
            }],
            role: Role::Assistant,
            usage: usage(),
            stop_reason: Some("end_turn".to_string()),
            tool_calls: None,
        }]));
        let po = test_po(client);

        let result = po
            .run_loop(
                42,
                &root,
                vec![("user".to_string(), "I want to build a todo app.".to_string())],
                None,
            )
            .await
            .expect("run_loop should succeed");

        match result {
            PoSessionResult::Text(t) => assert!(t.contains("who uses this system")),
            other => panic!("expected Text, got {:?}", other),
        }
    }
}
