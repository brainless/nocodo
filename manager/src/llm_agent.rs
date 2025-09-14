use crate::database::Database;
use crate::llm_client::{create_llm_client, LlmCompletionRequest, LlmMessage};
use crate::models::{LlmAgentSession, LlmAgentToolCall, LlmProviderConfig, ToolRequest};
use crate::tools::ToolExecutor;
use crate::websocket::WebSocketBroadcaster;
use anyhow::Result;
use async_stream::try_stream;
use futures_util::StreamExt;
use std::path::PathBuf;
use std::sync::Arc;

/// LLM Agent that handles direct communication with LLMs and tool execution
pub struct LlmAgent {
    db: Arc<Database>,
    ws: Arc<WebSocketBroadcaster>,
    tool_executor: ToolExecutor,
}

impl LlmAgent {
    pub fn new(db: Arc<Database>, ws: Arc<WebSocketBroadcaster>, project_path: PathBuf) -> Self {
        Self {
            db,
            ws,
            tool_executor: ToolExecutor::new(project_path),
        }
    }

    /// Create a new LLM agent session
    pub async fn create_session(
        &self,
        work_id: String,
        provider: String,
        model: String,
        system_prompt: Option<String>,
    ) -> Result<LlmAgentSession> {
        tracing::info!(
            work_id = %work_id,
            provider = %provider,
            model = %model,
            has_system_prompt = %system_prompt.is_some(),
            "Creating LLM agent session"
        );

        let session = LlmAgentSession::new(work_id, provider, model);

        // Store session in database
        self.db.create_llm_agent_session(&session)?;
        tracing::debug!(
            session_id = %session.id,
            work_id = %session.work_id,
            "LLM agent session created and stored in database"
        );

        // Create system message if provided
        if let Some(system_prompt) = system_prompt {
            self.db
                .create_llm_agent_message(&session.id, "system", system_prompt)?;
            tracing::debug!(
                session_id = %session.id,
                "System prompt added to LLM agent session"
            );
        }

        tracing::info!(
            session_id = %session.id,
            work_id = %session.work_id,
            provider = %session.provider,
            model = %session.model,
            "LLM agent session successfully created"
        );

        Ok(session)
    }

    /// Process a user message with the LLM agent
    pub async fn process_message(&self, session_id: &str, user_message: String) -> Result<String> {
        tracing::info!(
            session_id = %session_id,
            user_message_length = %user_message.len(),
            "Processing user message in LLM agent session"
        );

        // Get session
        let session = self.db.get_llm_agent_session(session_id)?;
        tracing::debug!(
            session_id = %session_id,
            work_id = %session.work_id,
            provider = %session.provider,
            model = %session.model,
            session_status = %session.status,
            "Retrieved LLM agent session"
        );

        // Store user message
        self.db
            .create_llm_agent_message(session_id, "user", user_message.clone())?;
        tracing::debug!(
            session_id = %session_id,
            "User message stored in database"
        );

        // Get conversation history
        let history = self.db.get_llm_agent_messages(session_id)?;
        tracing::debug!(
            session_id = %session_id,
            message_count = %history.len(),
            "Retrieved conversation history"
        );

        // Create LLM client
        let config = LlmProviderConfig {
            provider: session.provider.clone(),
            model: session.model.clone(),
            api_key: self.get_api_key(&session.provider)?,
            base_url: self.get_base_url(&session.provider),
            max_tokens: Some(4000),
            temperature: Some(0.7),
        };

        tracing::debug!(
            session_id = %session_id,
            provider = %config.provider,
            model = %config.model,
            max_tokens = ?config.max_tokens,
            temperature = ?config.temperature,
            "Creating LLM client"
        );

        let llm_client = create_llm_client(config)?;

        // Build conversation for LLM
        let mut messages = Vec::new();
        for msg in &history {
            messages.push(LlmMessage {
                role: msg.role.clone(),
                content: msg.content.clone(),
            });
        }

        // Add tool system prompt
        let tool_system_prompt = self.create_tool_system_prompt();
        messages.push(LlmMessage {
            role: "system".to_string(),
            content: tool_system_prompt,
        });

        tracing::debug!(
            session_id = %session_id,
            total_messages = %messages.len(),
            "Built conversation for LLM request"
        );

        // Log the full conversation being sent to LLM (truncated for large messages)
        for (i, msg) in messages.iter().enumerate() {
            let content_preview = if msg.content.len() > 200 {
                format!("{}...", &msg.content[..200])
            } else {
                msg.content.clone()
            };
            tracing::info!(
                session_id = %session_id,
                message_index = %i,
                message_role = %msg.role,
                message_content = %content_preview,
                message_length = %msg.content.len(),
                "Sending message to LLM"
            );
        }

        let request = LlmCompletionRequest {
            model: session.model.clone(),
            messages,
            max_tokens: Some(4000),
            temperature: Some(0.7),
            stream: Some(true),
        };

        tracing::info!(
            session_id = %session_id,
            provider = %session.provider,
            model = %session.model,
            "Sending request to LLM provider"
        );

        // Stream the response
        let mut assistant_response = String::new();
        let mut chunk_count = 0;
        let mut stream = llm_client.stream_complete(request);

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            chunk_count += 1;

            if !chunk.is_finished {
                assistant_response.push_str(&chunk.content);

                // Broadcast chunk to WebSocket
                self.ws
                    .broadcast_llm_agent_chunk(session_id.to_string(), chunk.content.clone())
                    .await;

                tracing::trace!(
                    session_id = %session_id,
                    chunk_number = %chunk_count,
                    chunk_length = %chunk.content.len(),
                    "Received and broadcasted LLM response chunk"
                );
            }
        }

        tracing::info!(
            session_id = %session_id,
            total_chunks = %chunk_count,
            response_length = %assistant_response.len(),
            "Completed LLM response streaming"
        );

        // Store assistant response
        self.db
            .create_llm_agent_message(session_id, "assistant", assistant_response.clone())?;
        tracing::debug!(
            session_id = %session_id,
            response_length = %assistant_response.len(),
            "Assistant response stored in database"
        );

        // Check if the response contains tool calls (JSON)
        if self.contains_tool_calls(&assistant_response) {
            tracing::info!(
                session_id = %session_id,
                "LLM response contains tool calls, processing them"
            );
            self.process_tool_calls(session_id, &assistant_response)
                .await?;
        } else {
            tracing::debug!(
                session_id = %session_id,
                "LLM response does not contain tool calls"
            );
        }

        tracing::info!(
            session_id = %session_id,
            work_id = %session.work_id,
            "Successfully processed user message with LLM agent"
        );

        Ok(assistant_response)
    }

    /// Process tool calls from LLM response
    async fn process_tool_calls(&self, session_id: &str, response: &str) -> Result<()> {
        tracing::info!(
            session_id = %session_id,
            "Processing tool calls from LLM response"
        );

        // Extract JSON tool calls from response
        let tool_calls = self.extract_tool_calls(response)?;
        tracing::debug!(
            session_id = %session_id,
            tool_call_count = %tool_calls.len(),
            "Extracted tool calls from LLM response"
        );

        for (index, tool_call_json) in tool_calls.into_iter().enumerate() {
            tracing::info!(
                session_id = %session_id,
                tool_index = %index,
                tool_call_json = %tool_call_json,
                "Processing tool call"
            );

            // Parse tool request
            let tool_request: ToolRequest = match serde_json::from_value(tool_call_json.clone()) {
                Ok(request) => {
                    tracing::debug!(
                        session_id = %session_id,
                        tool_index = %index,
                        tool_request = ?request,
                        "Successfully parsed tool request"
                    );
                    request
                }
                Err(e) => {
                    tracing::error!(
                        session_id = %session_id,
                        tool_index = %index,
                        error = %e,
                        tool_call_json = %tool_call_json,
                        "Failed to parse tool request"
                    );
                    continue;
                }
            };

            // Create tool call record
            let tool_name = match &tool_request {
                ToolRequest::ListFiles(_) => "list_files",
                ToolRequest::ReadFile(_) => "read_file",
            };

            tracing::debug!(
                session_id = %session_id,
                tool_index = %index,
                tool_name = %tool_name,
                "Creating tool call record"
            );

            let mut tool_call = LlmAgentToolCall::new(
                session_id.to_string(),
                tool_name.to_string(),
                tool_call_json,
            );

            // Update tool call status to executing
            tool_call.status = "executing".to_string();
            let tool_call_id = self.db.create_llm_agent_tool_call(&tool_call)?;
            tracing::debug!(
                session_id = %session_id,
                tool_call_id = %tool_call_id,
                tool_name = %tool_name,
                "Tool call record created with executing status"
            );

            // Execute tool
            tracing::info!(
                session_id = %session_id,
                tool_call_id = %tool_call_id,
                tool_name = %tool_name,
                "Executing tool"
            );

            let tool_response = self.tool_executor.execute(tool_request).await;

            // Update tool call with response
            let response_value = match tool_response {
                Ok(response) => {
                    tool_call.complete(serde_json::to_value(response)?);
                    let response_json =
                        serde_json::to_value(tool_call.response.clone().unwrap_or_default())?;
                    tracing::info!(
                        session_id = %session_id,
                        tool_call_id = %tool_call_id,
                        tool_name = %tool_name,
                        "Tool execution completed successfully"
                    );
                    response_json
                }
                Err(e) => {
                    tool_call.fail(e.to_string());
                    let response_json =
                        serde_json::to_value(tool_call.response.clone().unwrap_or_default())?;
                    tracing::error!(
                        session_id = %session_id,
                        tool_call_id = %tool_call_id,
                        tool_name = %tool_name,
                        error = %e,
                        "Tool execution failed"
                    );
                    response_json
                }
            };

            self.db.update_llm_agent_tool_call(&tool_call)?;
            tracing::debug!(
                session_id = %session_id,
                tool_call_id = %tool_call_id,
                "Tool call record updated with execution result"
            );

            // Add tool response to conversation
            self.db.create_llm_agent_message(
                session_id,
                "tool",
                serde_json::to_string(&response_value)?,
            )?;
            tracing::debug!(
                session_id = %session_id,
                tool_call_id = %tool_call_id,
                "Tool response added to conversation"
            );

            // If there are tool results, follow up with LLM
            tracing::info!(
                session_id = %session_id,
                tool_call_id = %tool_call_id,
                "Following up with LLM after tool execution"
            );
            self.follow_up_with_llm(session_id).await?;
        }

        tracing::info!(
            session_id = %session_id,
            "Completed processing all tool calls"
        );

        Ok(())
    }

    /// Follow up with LLM after tool execution
    async fn follow_up_with_llm(&self, session_id: &str) -> Result<String> {
        tracing::info!(
            session_id = %session_id,
            "Following up with LLM after tool execution"
        );

        // Get updated conversation history
        let history = self.db.get_llm_agent_messages(session_id)?;
        tracing::debug!(
            session_id = %session_id,
            message_count = %history.len(),
            "Retrieved updated conversation history for follow-up"
        );

        // Get session
        let session = self.db.get_llm_agent_session(session_id)?;
        tracing::debug!(
            session_id = %session_id,
            work_id = %session.work_id,
            provider = %session.provider,
            model = %session.model,
            "Retrieved session for follow-up"
        );

        // Create LLM client
        let config = LlmProviderConfig {
            provider: session.provider.clone(),
            model: session.model.clone(),
            api_key: self.get_api_key(&session.provider)?,
            base_url: self.get_base_url(&session.provider),
            max_tokens: Some(4000),
            temperature: Some(0.7),
        };

        tracing::debug!(
            session_id = %session_id,
            provider = %config.provider,
            model = %config.model,
            "Creating LLM client for follow-up"
        );

        let llm_client = create_llm_client(config)?;

        // Build conversation for LLM
        let messages: Vec<_> = history
            .into_iter()
            .map(|msg| LlmMessage {
                role: msg.role,
                content: msg.content,
            })
            .collect();

        tracing::debug!(
            session_id = %session_id,
            total_messages = %messages.len(),
            "Built conversation for LLM follow-up request"
        );

        // Log the follow-up conversation being sent to LLM (truncated for large messages)
        for (i, msg) in messages.iter().enumerate() {
            let content_preview = if msg.content.len() > 200 {
                format!("{}...", &msg.content[..200])
            } else {
                msg.content.clone()
            };
            tracing::info!(
                session_id = %session_id,
                message_index = %i,
                message_role = %msg.role,
                message_content = %content_preview,
                message_length = %msg.content.len(),
                "Sending follow-up message to LLM"
            );
        }

        let request = LlmCompletionRequest {
            model: session.model.clone(),
            messages,
            max_tokens: Some(4000),
            temperature: Some(0.7),
            stream: Some(true),
        };

        tracing::info!(
            session_id = %session_id,
            provider = %session.provider,
            model = %session.model,
            "Sending follow-up request to LLM provider"
        );

        // Stream the response
        let mut assistant_response = String::new();
        let mut chunk_count = 0;
        let mut stream = llm_client.stream_complete(request);

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            chunk_count += 1;

            if !chunk.is_finished {
                assistant_response.push_str(&chunk.content);

                // Broadcast chunk to WebSocket
                self.ws
                    .broadcast_llm_agent_chunk(session_id.to_string(), chunk.content.clone())
                    .await;

                tracing::trace!(
                    session_id = %session_id,
                    chunk_number = %chunk_count,
                    chunk_length = %chunk.content.len(),
                    "Received and broadcasted LLM follow-up response chunk"
                );
            }
        }

        tracing::info!(
            session_id = %session_id,
            total_chunks = %chunk_count,
            response_length = %assistant_response.len(),
            "Completed LLM follow-up response streaming"
        );

        // Store assistant response
        self.db
            .create_llm_agent_message(session_id, "assistant", assistant_response.clone())?;
        tracing::debug!(
            session_id = %session_id,
            response_length = %assistant_response.len(),
            "Follow-up assistant response stored in database"
        );

        tracing::info!(
            session_id = %session_id,
            work_id = %session.work_id,
            "Successfully completed LLM follow-up after tool execution"
        );

        Ok(assistant_response)
    }

    /// Create system prompt for tool usage
    fn create_tool_system_prompt(&self) -> String {
        r#"You are an AI assistant with access to file system tools. You can use the following tools:

1. **list_files**: List files and directories in a project
   - Request format: {"type": "list_files", "path": "<directory_path>", "recursive": <boolean>, "include_hidden": <boolean>}
   - Example: {"type": "list_files", "path": ".", "recursive": false, "include_hidden": false}

2. **read_file**: Read the content of a file
   - Request format: {"type": "read_file", "path": "<file_path>", "max_size": <bytes>}
   - Example: {"type": "read_file", "path": "src/main.rs", "max_size": 10000}

When you need to use a tool, respond with ONLY the JSON request for that tool. Do not include any other text. The tool will be executed and you will receive the results, after which you can continue your response.

Always analyze the project structure and read relevant files before providing code solutions. Be concise and focus on the user's specific needs."#.to_string()
    }

    /// Check if response contains tool calls
    fn contains_tool_calls(&self, response: &str) -> bool {
        // Look for JSON objects that might be tool calls
        response.contains("\"type\":\"list_files\"") || response.contains("\"type\":\"read_file\"")
    }

    /// Extract tool calls from response
    fn extract_tool_calls(&self, response: &str) -> Result<Vec<serde_json::Value>> {
        let mut tool_calls = Vec::new();

        // Simple JSON extraction (in production, use a more robust parser)
        for line in response.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('{') && trimmed.ends_with('}') {
                if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(trimmed) {
                    if let Some(tool_type) = json_value.get("type").and_then(|v| v.as_str()) {
                        if tool_type == "list_files" || tool_type == "read_file" {
                            tool_calls.push(json_value);
                        }
                    }
                }
            }
        }

        Ok(tool_calls)
    }

    /// Get API key for provider
    fn get_api_key(&self, provider: &str) -> Result<String> {
        // In production, this should come from secure configuration
        match provider.to_lowercase().as_str() {
            "grok" => std::env::var("GROK_API_KEY").map_err(|e| anyhow::anyhow!(e)),
            "openai" => std::env::var("OPENAI_API_KEY").map_err(|e| anyhow::anyhow!(e)),
            "anthropic" | "claude" => {
                std::env::var("ANTHROPIC_API_KEY").map_err(|e| anyhow::anyhow!(e))
            }
            _ => Err(anyhow::anyhow!(
                "No API key configured for provider: {}",
                provider
            )),
        }
    }

    /// Get base URL for provider
    fn get_base_url(&self, provider: &str) -> Option<String> {
        match provider.to_lowercase().as_str() {
            "grok" => Some("https://api.x.ai".to_string()),
            "openai" => None, // Use default OpenAI URL
            "anthropic" | "claude" => Some("https://api.anthropic.com".to_string()),
            _ => None,
        }
    }

    /// Complete a session
    pub async fn complete_session(&self, session_id: &str) -> Result<()> {
        tracing::info!(
            session_id = %session_id,
            "Completing LLM agent session"
        );

        let mut session = self.db.get_llm_agent_session(session_id)?;
        let old_status = session.status.clone();
        session.complete();
        self.db.update_llm_agent_session(&session)?;

        tracing::info!(
            session_id = %session_id,
            work_id = %session.work_id,
            old_status = %old_status,
            new_status = %session.status,
            "LLM agent session completed successfully"
        );

        Ok(())
    }

    /// Fail a session
    #[allow(dead_code)]
    pub async fn fail_session(&self, session_id: &str) -> Result<()> {
        tracing::info!(
            session_id = %session_id,
            "Failing LLM agent session"
        );

        let mut session = self.db.get_llm_agent_session(session_id)?;
        let old_status = session.status.clone();
        session.fail();
        self.db.update_llm_agent_session(&session)?;

        tracing::warn!(
            session_id = %session_id,
            work_id = %session.work_id,
            old_status = %old_status,
            new_status = %session.status,
            "LLM agent session failed"
        );

        Ok(())
    }

    /// Get session status
    pub async fn get_session_status(&self, session_id: &str) -> Result<LlmAgentSession> {
        tracing::debug!(
            session_id = %session_id,
            "Getting LLM agent session status"
        );

        let session = self
            .db
            .get_llm_agent_session(session_id)
            .map_err(|e| anyhow::anyhow!(e))?;

        tracing::debug!(
            session_id = %session_id,
            work_id = %session.work_id,
            status = %session.status,
            provider = %session.provider,
            model = %session.model,
            "Retrieved LLM agent session status"
        );

        Ok(session)
    }

    /// Stream session progress
    #[allow(dead_code)]
    pub fn stream_session_progress(
        &self,
        session_id: String,
    ) -> impl futures_util::Stream<Item = Result<String>> + use<'_> {
        try_stream! {
            let session = self.db.get_llm_agent_session(&session_id)?;

            // Send initial status
            yield format!("Session status: {}", session.status);

            // Stream messages
            let messages = self.db.get_llm_agent_messages(&session_id)?;
            for message in messages {
                yield format!("[{}] {}", message.role, message.content);
            }
        }
    }
}
