use std::sync::Arc;

use llm_sdk::{
    client::LlmClient,
    tools::{Tool, ToolChoice},
    types::{CompletionRequest, Message, Role},
};

use shared_types::SchemaDef;

use super::{
    prompts::system_prompt,
    tools::{AskUserParams, StopAgentParams},
};
use crate::{
    error::AgentError,
    storage::{AgentStorage, AgentType, ChatMessage, SchemaStorage},
};
const MAX_NUDGES: u32 = 3;

// ---------------------------------------------------------------------------
// Public result type returned to callers after each user turn
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum AgentResponse {
    /// Model replied with text (possibly after calling generate_schema internally).
    Text(String),
    /// Model called generate_schema; schema is now stored. Text is the model's message.
    SchemaGenerated {
        text: String,
        schema: SchemaDef,
        schema_row_id: i64,
    },
    /// Model called stop_agent because the request is outside its domain.
    Stopped(String),
    /// Model called ask_user because it needs clarifying information.
    Question(String),
}

// ---------------------------------------------------------------------------
// Agent
// ---------------------------------------------------------------------------

pub struct SchemaDesignerAgent {
    llm_client: Arc<dyn LlmClient>,
    storage: Arc<dyn AgentStorage>,
    schema_storage: Arc<dyn SchemaStorage>,
    model: String,
    project_id: i64,
}

impl SchemaDesignerAgent {
    pub fn new(
        llm_client: Arc<dyn LlmClient>,
        storage: Arc<dyn AgentStorage>,
        schema_storage: Arc<dyn SchemaStorage>,
        model: impl Into<String>,
        project_id: i64,
    ) -> Self {
        Self {
            llm_client,
            storage,
            schema_storage,
            model: model.into(),
            project_id,
        }
    }

    /// Process one user message and return the agent's response.
    /// Session is created automatically on first call and resumed on subsequent calls.
    /// Set `preview_mode=true` to return schema without persisting to DB.
    pub async fn chat(
        &self,
        user_text: &str,
        preview_mode: bool,
    ) -> Result<AgentResponse, AgentError> {
        log::info!(
            "[Agent] Starting chat for project={}, preview={}",
            self.project_id,
            preview_mode
        );
        log::debug!("[Agent] User message: {}", user_text);

        // Always resume the single session for this project + agent type.
        let session = self
            .storage
            .get_or_create_session(self.project_id, AgentType::SchemaDesigner.as_str())
            .await?;
        let session_id = session
            .id
            .expect("session must have id after get_or_create");
        log::info!("[Agent] Using session_id={}", session_id);

        // Persist the incoming user message.
        self.storage
            .create_message(ChatMessage {
                id: None,
                session_id,
                role: "user".to_string(),
                agent_type: None,
                content: user_text.to_string(),
                tool_call_id: None,
                tool_name: None,
                turn_id: None,
                created_at: 0,
            })
            .await?;

        self.run_loop(session_id, preview_mode).await
    }

    /// Continue an existing chat session.
    /// The caller is responsible for persisting the incoming user message first.
    pub async fn chat_with_session(
        &self,
        session_id: i64,
        preview_mode: bool,
    ) -> Result<AgentResponse, AgentError> {
        self.run_loop(session_id, preview_mode).await
    }

    // -----------------------------------------------------------------------
    // Internal: drive the LLM loop until a final response is produced.
    // -----------------------------------------------------------------------

    async fn run_loop(
        &self,
        session_id: i64,
        preview_mode: bool,
    ) -> Result<AgentResponse, AgentError> {
        let generate_schema_tool = Tool::from_type::<SchemaDef>()
            .name("generate_schema")
            .description(
                "Emit a complete, normalized SQLite schema for the user's requirements. \
                 Call this once per turn with the full schema. Each call creates a new \
                 versioned snapshot.",
            )
            .build();

        let stop_agent_tool = Tool::from_type::<StopAgentParams>()
            .name("stop_agent")
            .description(
                "Call this when the user's request is outside the schema-design domain \
                 (not expressible as a relational database schema). Provide a polite \
                 explanation in the reply field.",
            )
            .build();

        let ask_user_tool = Tool::from_type::<AskUserParams>()
            .name("ask_user")
            .description(
                "Call this when you need clarifying information from the user before you \
                 can design a proper schema. Provide your question in the question field. \
                 You may use plain text or Markdown formatting.",
            )
            .build();

        let mut nudges: u32 = 0;

        loop {
            // Reconstruct full message history from storage for every turn.
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
                    }
                })
                .collect();

            let request = CompletionRequest {
                messages: llm_messages,
                max_tokens: 4096,
                model: self.model.clone(),
                system: Some(system_prompt()),
                temperature: Some(0.2),
                top_p: None,
                stop_sequences: None,
                tools: Some(vec![generate_schema_tool.clone(), stop_agent_tool.clone(), ask_user_tool.clone()]),
                tool_choice: Some(ToolChoice::Auto),
                response_format: None,
            };

            log::debug!("[Agent] Sending {} messages to LLM", request.messages.len());
            log::info!("[Agent] Calling LLM with model={}", self.model);

            let response = self.llm_client.complete(request).await?;

            log::info!(
                "[Agent] LLM response received, stop_reason={:?}",
                response.stop_reason
            );

            // Extract text content from the response.
            let assistant_text = response
                .content
                .iter()
                .filter_map(|b| match b {
                    llm_sdk::types::ContentBlock::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join("");

            // Build the optional leading text row (present when the LLM sends both
            // text and a tool call in the same response).
            let agent_type_str = AgentType::SchemaDesigner.as_str().to_string();
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

            // Handle tool calls.
            if let Some(tool_calls) = response.tool_calls {
                log::info!("[Agent] LLM made {} tool call(s)", tool_calls.len());
                for tool_call in tool_calls {
                    let tool_name = tool_call.name();
                    let call_id = tool_call.id().to_string();
                    log::info!("[Agent] Tool call: name={}, call_id={}", tool_name, call_id);
                    match tool_name {
                        "generate_schema" => {
                            log::info!("[Agent] Processing generate_schema tool call");
                            let mut params: SchemaDef =
                                tool_call.parse_arguments().map_err(AgentError::Llm)?;
                            log::debug!(
                                "[Agent] Schema params parsed: {} tables",
                                params.tables.len()
                            );

                            // Ensure created_at / updated_at are always the last columns.
                            for table in &mut params.tables {
                                let (mut audit, rest): (Vec<_>, Vec<_>) = table
                                    .columns
                                    .drain(..)
                                    .partition(|c| c.name == "created_at" || c.name == "updated_at");
                                audit.sort_by_key(|c| if c.name == "updated_at" { 0u8 } else { 1u8 });
                                table.columns = rest;
                                table.columns.extend(audit);
                            }

                            let schema_json = serde_json::to_string(&params)?;

                            // Save schema before building turn rows so result_text is ready.
                            let (schema_row_id, result_text) = if preview_mode {
                                log::info!(
                                    "[Agent] Schema generated in preview mode - not saving to DB"
                                );
                                (
                                    0,
                                    format!(
                                        "Schema generated (preview mode - not saved):\n```json\n{}\n```",
                                        schema_json
                                    ),
                                )
                            } else {
                                log::info!("[Agent] Saving schema to database...");
                                let row_id = self
                                    .schema_storage
                                    .save_schema(self.project_id, session_id, &schema_json)
                                    .await?;
                                log::info!("[Agent] Schema saved with row_id={}", row_id);
                                (
                                    row_id,
                                    format!(
                                        "Schema saved as version {}.\n```json\n{}\n```",
                                        row_id, schema_json
                                    ),
                                )
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
                                content: schema_json,
                                tool_call_id: Some(call_id.clone()),
                                tool_name: Some("generate_schema".to_string()),
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
                                tool_name: Some("generate_schema".to_string()),
                                turn_id: None,
                                created_at: 0,
                            });
                            self.storage.create_turn(turn).await?;

                            log::info!("[Agent] Returning SchemaGenerated response");
                            return Ok(AgentResponse::SchemaGenerated {
                                text: assistant_text,
                                schema: params,
                                schema_row_id,
                            });
                        }

                        "stop_agent" => {
                            log::info!("[Agent] Processing stop_agent tool call");
                            let params: StopAgentParams =
                                tool_call.parse_arguments().map_err(AgentError::Llm)?;
                            log::info!("[Agent] Stop reason: {}", params.reply);

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
                                tool_name: Some("stop_agent".to_string()),
                                turn_id: None,
                                created_at: 0,
                            });
                            self.storage.create_turn(turn).await?;

                            log::info!("[Agent] Returning Stopped response");
                            return Ok(AgentResponse::Stopped(params.reply));
                        }

                        "ask_user" => {
                            log::info!("[Agent] Processing ask_user tool call");
                            let params: AskUserParams =
                                tool_call.parse_arguments().map_err(AgentError::Llm)?;
                            log::info!("[Agent] Question: {}", params.question);

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
                                tool_name: Some("ask_user".to_string()),
                                turn_id: None,
                                created_at: 0,
                            });
                            turn.push(ChatMessage {
                                id: None,
                                session_id,
                                role: "tool".to_string(),
                                agent_type: None,
                                content: "Question sent to user. Awaiting user response.".to_string(),
                                tool_call_id: Some(call_id),
                                tool_name: Some("ask_user".to_string()),
                                turn_id: None,
                                created_at: 0,
                            });
                            self.storage.create_turn(turn).await?;

                            log::info!("[Agent] Returning Question response");
                            return Ok(AgentResponse::Question(params.question));
                        }

                        unknown => {
                            log::error!("[Agent] Unknown tool called: {}", unknown);
                            return Err(AgentError::Other(format!(
                                "Model called unknown tool: {}",
                                unknown
                            )));
                        }
                    }
                }
            }

            // No tool call: persist text and return.
            if !assistant_text.is_empty() {
                log::info!(
                    "[Agent] Returning Text response ({} chars)",
                    assistant_text.len()
                );
                self.storage.create_turn(vec![text_row(assistant_text.clone())]).await?;
                return Ok(AgentResponse::Text(assistant_text));
            }

            // No text and no tool call — nudge the model.
            nudges += 1;
            log::warn!(
                "[Agent] No response from model, nudge {}/{}",
                nudges,
                MAX_NUDGES
            );
            if nudges >= MAX_NUDGES {
                log::error!("[Agent] Max nudges reached, giving up");
                return Err(AgentError::Other(
                    "Model did not produce a response after multiple nudges.".to_string(),
                ));
            }

            self.storage
                .create_message(ChatMessage {
                    id: None,
                    session_id,
                    role: "user".to_string(),
                    agent_type: None,
                    content: "Please respond with either a schema (call generate_schema) or an explanation (call stop_agent).".to_string(),
                    tool_call_id: None,
                    tool_name: None,
                    turn_id: None,
                    created_at: 0,
                })
                .await?;
        }
    }
}
