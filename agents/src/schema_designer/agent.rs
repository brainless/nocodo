use std::sync::Arc;

use llm_sdk::{
    client::LlmClient,
    tools::{Tool, ToolChoice},
    types::{CompletionRequest, Message, Role},
};

use super::{
    prompts::system_prompt,
    tools::{GenerateSchemaParams, StopAgentParams},
};
use crate::{
    error::AgentError,
    storage::{AgentStorage, ChatMessage, SchemaStorage, ToolCallRecord},
};

const AGENT_TYPE: &str = "schema_designer";
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
        schema: GenerateSchemaParams,
        schema_row_id: i64,
    },
    /// Model called stop_agent because the request is outside its domain.
    Stopped(String),
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
    pub async fn chat(&self, user_text: &str, preview_mode: bool) -> Result<AgentResponse, AgentError> {
        log::info!("[Agent] Starting chat for project={}, preview={}", self.project_id, preview_mode);
        log::debug!("[Agent] User message: {}", user_text);
        
        // Always resume the single session for this project + agent type.
        let session = self
            .storage
            .get_or_create_session(self.project_id, AGENT_TYPE)
            .await?;
        let session_id = session.id.expect("session must have id after get_or_create");
        log::info!("[Agent] Using session_id={}", session_id);

        // Persist the incoming user message.
        let user_msg_id = self.storage
            .create_message(ChatMessage {
                id: None,
                session_id,
                role: "user".to_string(),
                content: user_text.to_string(),
                tool_call_id: None,
                created_at: 0,
            })
            .await?;

        self.run_loop(session_id, user_msg_id, preview_mode).await
    }

    /// Continue an existing chat session with a new user message.
    /// Returns (session_id, user_message_id) for the API to track.
    pub async fn chat_with_session(
        &self,
        session_id: i64,
        user_text: &str,
        preview_mode: bool,
    ) -> Result<(i64, AgentResponse), AgentError> {
        // Persist the incoming user message.
        let user_msg_id = self.storage
            .create_message(ChatMessage {
                id: None,
                session_id,
                role: "user".to_string(),
                content: user_text.to_string(),
                tool_call_id: None,
                created_at: 0,
            })
            .await?;

        let response = self.run_loop(session_id, user_msg_id, preview_mode).await?;
        Ok((user_msg_id, response))
    }

    // -----------------------------------------------------------------------
    // Internal: drive the LLM loop until a final response is produced.
    // -----------------------------------------------------------------------

    async fn run_loop(
        &self,
        session_id: i64,
        _user_msg_id: i64,
        preview_mode: bool,
    ) -> Result<AgentResponse, AgentError> {
        let generate_schema_tool = Tool::from_type::<GenerateSchemaParams>()
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
                tools: Some(vec![generate_schema_tool.clone(), stop_agent_tool.clone()]),
                tool_choice: Some(ToolChoice::Auto),
                response_format: None,
            };

            log::debug!("[Agent] Sending {} messages to LLM", request.messages.len());
            log::info!("[Agent] Calling LLM with model={}", self.model);
            
            let response = self.llm_client.complete(request).await?;
            
            log::info!("[Agent] LLM response received, stop_reason={:?}", response.stop_reason);

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

            // Persist assistant text if non-empty.
            let assistant_msg_id = if !assistant_text.is_empty() {
                let id = self
                    .storage
                    .create_message(ChatMessage {
                        id: None,
                        session_id,
                        role: "assistant".to_string(),
                        content: assistant_text.clone(),
                        tool_call_id: None,
                        created_at: 0,
                    })
                    .await?;
                Some(id)
            } else {
                None
            };

            // Handle tool calls.
            if let Some(tool_calls) = response.tool_calls {
                log::info!("[Agent] LLM made {} tool call(s)", tool_calls.len());
                for tool_call in tool_calls {
                    let tool_name = tool_call.name();
                    let call_id = tool_call.id();
                    log::info!("[Agent] Tool call: name={}, call_id={}", tool_name, call_id);
                    match tool_name {
                        "generate_schema" => {
                            log::info!("[Agent] Processing generate_schema tool call");
                            let params: GenerateSchemaParams =
                                tool_call.parse_arguments().map_err(AgentError::Llm)?;
                            log::debug!("[Agent] Schema params parsed: {} tables", params.tables.len());

                            let schema_json = serde_json::to_string(&params)?;

                            // Persist tool call record (linked to the assistant message, or a
                            // synthetic one if the model sent no text).
                            let msg_id = match assistant_msg_id {
                                Some(id) => id,
                                None => {
                                    self.storage
                                        .create_message(ChatMessage {
                                            id: None,
                                            session_id,
                                            role: "assistant".to_string(),
                                            content: String::new(),
                                            tool_call_id: None,
                                            created_at: 0,
                                        })
                                        .await?
                                }
                            };

                            let tc_id = self
                                .storage
                                .create_tool_call(ToolCallRecord {
                                    id: None,
                                    message_id: msg_id,
                                    call_id: tool_call.id().to_string(),
                                    tool_name: "generate_schema".to_string(),
                                    arguments: schema_json.clone(),
                                    result: None,
                                    created_at: 0,
                                })
                                .await?;

                            // Save schema to DB only if not in preview mode
                            let (schema_row_id, result_text) = if preview_mode {
                                log::info!("[Agent] Schema generated in preview mode - not saving to DB");
                                (0, "Schema generated (preview mode - not saved).".to_string())
                            } else {
                                log::info!("[Agent] Saving schema to database...");
                                let row_id = self
                                    .schema_storage
                                    .save_schema(self.project_id, session_id, &schema_json)
                                    .await?;
                                log::info!("[Agent] Schema saved with row_id={}", row_id);
                                (row_id, format!("Schema stored successfully as version {}.", row_id))
                            };

                            self.storage
                                .update_tool_call_result(tc_id, &result_text)
                                .await?;

                            // Persist the tool result message so the model sees it next turn.
                            self.storage
                                .create_message(ChatMessage {
                                    id: None,
                                    session_id,
                                    role: "tool".to_string(),
                                    content: result_text,
                                    tool_call_id: Some(tool_call.id().to_string()),
                                    created_at: 0,
                                })
                                .await?;

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

                            // Persist the stop call.
                            let msg_id = match assistant_msg_id {
                                Some(id) => id,
                                None => {
                                    self.storage
                                        .create_message(ChatMessage {
                                            id: None,
                                            session_id,
                                            role: "assistant".to_string(),
                                            content: params.reply.clone(),
                                            tool_call_id: None,
                                            created_at: 0,
                                        })
                                        .await?
                                }
                            };

                            self.storage
                                .create_tool_call(ToolCallRecord {
                                    id: None,
                                    message_id: msg_id,
                                    call_id: tool_call.id().to_string(),
                                    tool_name: "stop_agent".to_string(),
                                    arguments: serde_json::to_string(tool_call.arguments())?,
                                    result: Some("Agent stopped.".to_string()),
                                    created_at: 0,
                                })
                                .await?;

                            log::info!("[Agent] Returning Stopped response");
                            return Ok(AgentResponse::Stopped(params.reply));
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

            // No tool call: if the model produced text we can return it.
            if !assistant_text.is_empty() {
                log::info!("[Agent] Returning Text response ({} chars)", assistant_text.len());
                return Ok(AgentResponse::Text(assistant_text));
            }

            // No text and no tool call — nudge the model.
            nudges += 1;
            log::warn!("[Agent] No response from model, nudge {}/{}", nudges, MAX_NUDGES);
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
                    content: "Please respond with either a schema (call generate_schema) or an explanation (call stop_agent).".to_string(),
                    tool_call_id: None,
                    created_at: 0,
                })
                .await?;
        }
    }
}
