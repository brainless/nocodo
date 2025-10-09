use crate::config::AppConfig;
use crate::database::Database;
use crate::llm_client::{create_llm_client, LlmCompletionRequest, LlmMessage};
use crate::models::{LlmAgentSession, LlmAgentToolCall, LlmProviderConfig};
use crate::tools::ToolExecutor;
use crate::websocket::WebSocketBroadcaster;
use anyhow::Result;
use std::boxed::Box;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

/// LLM Agent that handles direct communication with LLMs and tool execution
pub struct LlmAgent {
    db: Arc<Database>,
    ws: Arc<WebSocketBroadcaster>,
    tool_executor: ToolExecutor,
    config: Arc<AppConfig>,
}

impl LlmAgent {
    pub fn new(
        db: Arc<Database>,
        ws: Arc<WebSocketBroadcaster>,
        project_path: PathBuf,
        config: Arc<AppConfig>,
    ) -> Self {
        Self {
            db,
            ws,
            tool_executor: ToolExecutor::new(project_path),
            config,
        }
    }

    /// Create a new LLM agent session
    pub async fn create_session(
        &self,
        work_id: i64,
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

        let mut session = LlmAgentSession::new(work_id, provider, model);

        // Store session in database
        let session_id = self.db.create_llm_agent_session(&session)?;
        session.id = session_id;
        tracing::debug!(
            session_id = %session.id,
            work_id = %session.work_id,
            "LLM agent session created and stored in database"
        );

        // Create system message if provided
        if let Some(system_prompt) = system_prompt {
            self.db
                .create_llm_agent_message(session.id, "system", system_prompt)?;
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
    pub async fn process_message(&self, session_id: i64, user_message: String) -> Result<String> {
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
                content: Some(msg.content.clone()),
                tool_calls: None,
                function_call: None,
                tool_call_id: None,
            });
        }

        tracing::debug!(
            session_id = %session_id,
            total_messages = %messages.len(),
            "Built conversation for LLM request"
        );

        // Log the full conversation being sent to LLM (truncated for large messages)
        for (i, msg) in messages.iter().enumerate() {
            let content_preview = if let Some(content) = &msg.content {
                if content.len() > 200 {
                    format!("{}...", &content[..200])
                } else {
                    content.clone()
                }
            } else {
                "<no content>".to_string()
            };
            let content_length = msg.content.as_ref().map(|c| c.len()).unwrap_or(0);
            tracing::info!(
                session_id = %session_id,
                message_index = %i,
                message_role = %msg.role,
                message_content = %content_preview,
                message_length = %content_length,
                "Sending message to LLM"
            );
        }

        // Create tool definitions for native tool calling
        let tools = Some(self.create_native_tool_definitions());

        let request = LlmCompletionRequest {
            model: session.model.clone(),
            messages,
            max_tokens: Some(4000),
            temperature: Some(0.3),
            stream: Some(false),
            tools,
            tool_choice: Some(crate::llm_client::ToolChoice::Auto("auto".to_string())), // Explicitly allow tool usage
            functions: None,
            function_call: None,
        };

        tracing::info!(
            session_id = %session_id,
            provider = %session.provider,
            model = %session.model,
            "Sending request to LLM provider"
        );

        // Get the complete response (non-streaming)
        let response = llm_client.complete(request).await?;

        let raw_assistant_response = response
            .choices
            .first()
            .and_then(|choice| choice.message.as_ref())
            .and_then(|message| message.content.clone())
            .unwrap_or_default();

        // Clean up the assistant response by removing unwanted prefixes
        let assistant_response = self.clean_assistant_response(&raw_assistant_response);

        let accumulated_tool_calls = llm_client.extract_tool_calls_from_response(&response);

        // Debug logging for tool call extraction
        tracing::info!(
            session_id = %session_id,
            extracted_tool_calls_count = %accumulated_tool_calls.len(),
            response_choices_count = %response.choices.len(),
            "Extracted tool calls from LLM response"
        );

        // Log details of response structure for debugging
        for (choice_idx, choice) in response.choices.iter().enumerate() {
            if let Some(message) = &choice.message {
                let message_tool_calls_count =
                    message.tool_calls.as_ref().map(|tc| tc.len()).unwrap_or(0);
                tracing::info!(
                    session_id = %session_id,
                    choice_index = %choice_idx,
                    message_role = %message.role,
                    message_content_length = %message.content.as_ref().map(|c| c.len()).unwrap_or(0),
                    message_tool_calls_count = %message_tool_calls_count,
                    has_function_call = %message.function_call.is_some(),
                    finish_reason = ?choice.finish_reason,
                    "Response choice details"
                );

                // Log each tool call in the message for debugging
                if let Some(tool_calls) = &message.tool_calls {
                    for (tc_idx, tool_call) in tool_calls.iter().enumerate() {
                        tracing::info!(
                            session_id = %session_id,
                            choice_index = %choice_idx,
                            tool_call_index = %tc_idx,
                            tool_call_id = %tool_call.id,
                            tool_call_type = %tool_call.r#type,
                            function_name = %tool_call.function.name,
                            arguments_length = %tool_call.function.arguments.len(),
                            "Found tool call in message"
                        );
                    }
                }
            }

            // Also check choice-level tool calls (Anthropic format)
            let choice_tool_calls_count =
                choice.tool_calls.as_ref().map(|tc| tc.len()).unwrap_or(0);
            if choice_tool_calls_count > 0 {
                tracing::info!(
                    session_id = %session_id,
                    choice_index = %choice_idx,
                    choice_tool_calls_count = %choice_tool_calls_count,
                    "Found tool calls at choice level"
                );
            }
        }

        // Broadcast the complete response to WebSocket
        self.ws
            .broadcast_llm_agent_chunk(session_id, assistant_response.clone())
            .await;

        tracing::info!(
            session_id = %session_id,
            response_length = %assistant_response.len(),
            tool_calls_count = %accumulated_tool_calls.len(),
            "Received complete LLM response"
        );

        // Store assistant response with tool call information in structured JSON format
        // This allows proper conversation reconstruction and consistent UI display
        let enhanced_assistant_response = if !accumulated_tool_calls.is_empty() {
            // Store all providers in the same structured JSON format
            let assistant_data = serde_json::json!({
                "text": assistant_response,
                "tool_calls": accumulated_tool_calls
            });
            serde_json::to_string(&assistant_data).unwrap_or_else(|_| assistant_response.clone())
        } else {
            assistant_response.clone()
        };

        self.db.create_llm_agent_message(
            session_id,
            "assistant",
            enhanced_assistant_response.clone(),
        )?;
        tracing::debug!(
            session_id = %session_id,
            response_length = %enhanced_assistant_response.len(),
            tool_calls_count = %accumulated_tool_calls.len(),
            "Assistant response with tool call info stored in database"
        );

        // Process native tool calls
        if !accumulated_tool_calls.is_empty() {
            tracing::info!(
                session_id = %session_id,
                tool_calls_count = %accumulated_tool_calls.len(),
                "Processing native tool calls from response"
            );

            // Log details of each tool call for debugging
            for (i, tool_call) in accumulated_tool_calls.iter().enumerate() {
                tracing::info!(
                    session_id = %session_id,
                    tool_index = %i,
                    tool_call_id = %tool_call.id,
                    function_name = %tool_call.function.name,
                    arguments_length = %tool_call.function.arguments.len(),
                    "Tool call details"
                );
            }

            self.process_native_tool_calls(session_id, &accumulated_tool_calls)
                .await?;

            tracing::info!(
                session_id = %session_id,
                "Completed processing native tool calls"
            );
        } else {
            tracing::debug!(
                session_id = %session_id,
                "No tool calls found in response"
            );
        }

        tracing::info!(
            session_id = %session_id,
            work_id = %session.work_id,
            "Successfully processed user message with LLM agent"
        );

        Ok(assistant_response)
    }

    /// Get the tool executor for a specific session's project
    async fn get_tool_executor_for_session(&self, session_id: i64) -> Result<ToolExecutor> {
        // Get session to find work_id
        let session = self.db.get_llm_agent_session(session_id)?;

        // Get work to find project_id
        let work = self.db.get_work_by_id(session.work_id)?;

        if let Some(project_id) = work.project_id {
            // Get project to find project path
            let project = self.db.get_project_by_id(project_id)?;
            Ok(ToolExecutor::new(PathBuf::from(project.path)))
        } else {
            // Fallback to the default tool executor
            Ok(ToolExecutor::new(self.tool_executor.base_path().clone()))
        }
    }

    /// Process native tool calls from LLM response
    async fn process_native_tool_calls(
        &self,
        session_id: i64,
        tool_calls: &[crate::llm_client::LlmToolCall],
    ) -> Result<()> {
        // Get session info for provider-specific handling
        let session = self.db.get_llm_agent_session(session_id)?;
        tracing::info!(
            session_id = %session_id,
            tool_calls_count = %tool_calls.len(),
            "Processing native tool calls"
        );

        for (index, tool_call) in tool_calls.iter().enumerate() {
            tracing::info!(
                session_id = %session_id,
                tool_index = %index,
                tool_call_id = %tool_call.id,
                function_name = %tool_call.function.name,
                "Processing native tool call"
            );

            // Parse the tool arguments based on function name for native function calling
            let tool_request: crate::models::ToolRequest = match tool_call.function.name.as_str() {
                "list_files" => {
                    match serde_json::from_str::<crate::models::ListFilesRequest>(
                        &tool_call.function.arguments,
                    ) {
                        Ok(request) => crate::models::ToolRequest::ListFiles(request),
                        Err(e) => {
                            tracing::error!(
                                session_id = %session_id,
                                tool_index = %index,
                                error = %e,
                                arguments = %tool_call.function.arguments,
                                "Failed to parse list_files arguments"
                            );
                            continue;
                        }
                    }
                }
                "read_file" => {
                    match serde_json::from_str::<crate::models::ReadFileRequest>(
                        &tool_call.function.arguments,
                    ) {
                        Ok(request) => crate::models::ToolRequest::ReadFile(request),
                        Err(e) => {
                            tracing::error!(
                                session_id = %session_id,
                                tool_index = %index,
                                error = %e,
                                arguments = %tool_call.function.arguments,
                                "Failed to parse read_file arguments"
                            );
                            continue;
                        }
                    }
                }
                "write_file" => {
                    match serde_json::from_str::<crate::models::WriteFileRequest>(
                        &tool_call.function.arguments,
                    ) {
                        Ok(request) => crate::models::ToolRequest::WriteFile(request),
                        Err(e) => {
                            tracing::error!(
                                session_id = %session_id,
                                tool_index = %index,
                                error = %e,
                                arguments = %tool_call.function.arguments,
                                "Failed to parse write_file arguments"
                            );
                            continue;
                        }
                    }
                }
                "grep" => {
                    match serde_json::from_str::<crate::models::GrepRequest>(
                        &tool_call.function.arguments,
                    ) {
                        Ok(request) => crate::models::ToolRequest::Grep(request),
                        Err(e) => {
                            tracing::error!(
                                session_id = %session_id,
                                tool_index = %index,
                                error = %e,
                                arguments = %tool_call.function.arguments,
                                "Failed to parse grep arguments"
                            );
                            continue;
                        }
                    }
                }
                unknown_function => {
                    tracing::error!(
                        session_id = %session_id,
                        tool_index = %index,
                        function_name = %unknown_function,
                        "Unknown function name in tool call"
                    );
                    continue;
                }
            };

            tracing::debug!(
                session_id = %session_id,
                tool_index = %index,
                tool_request = ?tool_request,
                "Successfully parsed native tool request from function call"
            );

            // Create tool call record
            let tool_name = match &tool_request {
                crate::models::ToolRequest::ListFiles(_) => "list_files",
                crate::models::ToolRequest::ReadFile(_) => "read_file",
                crate::models::ToolRequest::WriteFile(_) => "write_file",
                crate::models::ToolRequest::Grep(_) => "grep",
            };

            tracing::debug!(
                session_id = %session_id,
                tool_index = %index,
                tool_name = %tool_name,
                "Creating native tool call record"
            );

            let mut tool_call_record = LlmAgentToolCall::new(
                session_id,
                tool_name.to_string(),
                serde_json::to_value(&tool_request)?,
            );

            // Update tool call status to executing
            tool_call_record.status = "executing".to_string();
            let tool_call_id = self.db.create_llm_agent_tool_call(&tool_call_record)?;
            tracing::debug!(
                session_id = %session_id,
                tool_call_id = %tool_call_id,
                tool_name = %tool_name,
                "Native tool call record created with executing status"
            );

            // Broadcast tool call started
            self.ws
                .broadcast_tool_call_started(
                    session_id,
                    tool_call_id.to_string(),
                    tool_name.to_string(),
                )
                .await;

            // Execute tool
            tracing::info!(
                session_id = %session_id,
                tool_call_id = %tool_call_id,
                tool_name = %tool_name,
                "Executing native tool"
            );

            // Get project-specific tool executor
            let project_tool_executor = self.get_tool_executor_for_session(session_id).await?;
            let tool_response = project_tool_executor.execute(tool_request).await;

            // Update tool call with response
            let response_value = match tool_response {
                Ok(response) => {
                    tool_call_record.complete(serde_json::to_value(&response)?);
                    let response_json = serde_json::to_value(&response)?;
                    tracing::info!(
                        session_id = %session_id,
                        tool_call_id = %tool_call_id,
                        tool_name = %tool_name,
                        "Native tool execution completed successfully"
                    );

                    // Broadcast tool call completed
                    self.ws
                        .broadcast_tool_call_completed(
                            session_id,
                            tool_call_id.to_string(),
                            response_json.clone(),
                        )
                        .await;

                    response_json
                }
                Err(e) => {
                    tool_call_record.fail(e.to_string());
                    let error_value = serde_json::json!({
                        "error": e.to_string(),
                        "tool_name": tool_name
                    });
                    tracing::error!(
                        session_id = %session_id,
                        tool_call_id = %tool_call_id,
                        tool_name = %tool_name,
                        error = %e,
                        "Native tool execution failed"
                    );

                    // Broadcast tool call failed
                    self.ws
                        .broadcast_tool_call_failed(
                            session_id,
                            tool_call_id.to_string(),
                            e.to_string(),
                        )
                        .await;

                    error_value
                }
            };

            self.db.update_llm_agent_tool_call(&tool_call_record)?;
            tracing::debug!(
                session_id = %session_id,
                tool_call_id = %tool_call_id,
                "Native tool call record updated with execution result"
            );

            // Add tool response to conversation
            let tool_result_string = if session.provider == "anthropic" {
                // For Claude, we need to store tool results with the tool_use_id for proper formatting
                let tool_result_content = serde_json::json!({
                    "tool_use_id": tool_call.id,
                    "content": response_value
                });
                serde_json::to_string(&tool_result_content)?
            } else {
                // For other providers, store tool results as simple JSON
                serde_json::to_string(&response_value)?
            };
            let message_id =
                self.db
                    .create_llm_agent_message(session_id, "tool", tool_result_string)?;
            tracing::debug!(
                session_id = %session_id,
                tool_call_id = %tool_call_id,
                message_id = %message_id,
                "Native tool response added to conversation"
            );

            // Update tool call record with message_id
            tool_call_record.message_id = Some(message_id);
            self.db.update_llm_agent_tool_call(&tool_call_record)?;
            tracing::debug!(
                session_id = %session_id,
                tool_call_id = %tool_call_id,
                message_id = %message_id,
                "Tool call record updated with message_id"
            );
        }

        // After processing all tool calls, follow up with LLM
        tracing::warn!(  // Use warn to make it more visible
            session_id = %session_id,
            tool_calls_count = %tool_calls.len(),
            "FOLLOW_UP_DEBUG: About to call follow_up_with_llm_with_depth after processing {} tool calls",
            tool_calls.len()
        );

        // Add extra debug to see if follow-up is enabled/available
        tracing::warn!(
            session_id = %session_id,
            "FOLLOW_UP_DEBUG: Checking if follow-up calls are enabled and agent is available"
        );

        self.follow_up_with_llm_with_depth(session_id, 1).await?;
        tracing::warn!(  // Use warn to make it more visible
            session_id = %session_id,
            "FOLLOW_UP_DEBUG: follow_up_with_llm_with_depth completed successfully"
        );

        tracing::info!(
            session_id = %session_id,
            "Completed processing all native tool calls"
        );

        Ok(())
    }

    /// Follow up with LLM after tool execution
    #[allow(dead_code)]
    async fn follow_up_with_llm(&self, session_id: i64) -> Result<String> {
        self.follow_up_with_llm_with_depth(session_id, 0).await
    }

    /// Follow up with LLM after tool execution with recursion depth tracking
    fn follow_up_with_llm_with_depth<'a>(
        &'a self,
        session_id: i64,
        depth: u32,
    ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>> {
        Box::pin(async move {
            const MAX_RECURSION_DEPTH: u32 = 5; // Prevent infinite loops

            tracing::warn!(
                session_id = %session_id,
                current_depth = %depth,
                "FOLLOW_UP_DEBUG: ENTERED follow_up_with_llm_with_depth method - this should appear in both manual and E2E test"
            );

            if depth >= MAX_RECURSION_DEPTH {
                tracing::warn!(
                    session_id = %session_id,
                    current_depth = %depth,
                    max_depth = %MAX_RECURSION_DEPTH,
                    "Follow-up recursion depth limit reached, stopping processing"
                );
                return Ok("Maximum recursion depth reached.".to_string());
            }
            tracing::info!(
                session_id = %session_id,
                current_depth = %depth,
                "FOLLOW_UP_DEBUG: Starting follow-up with LLM after tool execution"
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
                temperature: Some(0.3), // Use same temperature as initial request for consistency
            };

            tracing::debug!(
                session_id = %session_id,
                provider = %config.provider,
                model = %config.model,
                "Creating LLM client for follow-up"
            );

            let llm_client = create_llm_client(config)?;

            // Build conversation for LLM (simplified to avoid conversation corruption)
            let messages: Vec<_> = history
                .into_iter()
                .map(|msg| LlmMessage {
                    role: msg.role,
                    content: Some(msg.content),
                    tool_calls: None,
                    function_call: None,
                    tool_call_id: None,
                })
                .collect();

            tracing::debug!(
                session_id = %session_id,
                total_messages = %messages.len(),
                "Built conversation for LLM follow-up request"
            );

            // Log the follow-up conversation being sent to LLM (truncated for large messages)
            for (i, msg) in messages.iter().enumerate() {
                let content_preview = if let Some(content) = &msg.content {
                    if content.len() > 500 {
                        format!("{}...", &content[..500])
                    } else {
                        content.clone()
                    }
                } else {
                    "<no content>".to_string()
                };
                let content_length = msg.content.as_ref().map(|c| c.len()).unwrap_or(0);
                tracing::warn!(  // Use warn to make it more visible
                    session_id = %session_id,
                    message_index = %i,
                    message_role = %msg.role,
                    message_content = %content_preview,
                    message_length = %content_length,
                    "FOLLOW_UP_DEBUG: Sending follow-up message to LLM"
                );
            }

            // CLAUDE_DEBUG: Log the exact conversation sent to Claude for debugging
            if session.provider == "anthropic" {
                tracing::info!(
                    session_id = %session_id,
                    "CLAUDE_DEBUG: Follow-up conversation history for Claude:"
                );
                for (i, msg) in messages.iter().enumerate() {
                    tracing::info!(
                        session_id = %session_id,
                        message_index = %i,
                        role = %msg.role,
                        content_length = %msg.content.as_ref().map(|c| c.len()).unwrap_or(0),
                        content_preview = %msg.content.as_ref().unwrap_or(&"<no content>".to_string()).chars().take(100).collect::<String>(),
                        "CLAUDE_DEBUG: Message in follow-up conversation"
                    );
                }
            }

            // Create tool definitions for native tool calling in follow-up
            let tools = Some(self.create_native_tool_definitions());

            let request = LlmCompletionRequest {
                model: session.model.clone(),
                messages,
                max_tokens: Some(4000),
                temperature: Some(0.3), // Use same temperature as initial request for consistency
                stream: Some(false),
                tools,
                tool_choice: Some(crate::llm_client::ToolChoice::Auto("auto".to_string())), // Explicitly allow tool usage
                functions: None,
                function_call: None,
            };

            tracing::info!(
                session_id = %session_id,
                provider = %session.provider,
                model = %session.model,
                "Sending follow-up request to LLM provider"
            );

            // Get the complete response (non-streaming)
            let response = llm_client.complete(request).await?;
            let raw_assistant_response = response
                .choices
                .first()
                .and_then(|choice| choice.message.as_ref())
                .and_then(|message| message.content.clone())
                .unwrap_or_default();

            // Clean up the assistant response by removing unwanted prefixes
            let assistant_response = self.clean_assistant_response(&raw_assistant_response);

            let follow_up_tool_calls = llm_client.extract_tool_calls_from_response(&response);

            // Broadcast the complete response to WebSocket
            self.ws
                .broadcast_llm_agent_chunk(session_id, assistant_response.clone())
                .await;

            tracing::info!(
                session_id = %session_id,
                response_length = %assistant_response.len(),
                follow_up_tool_calls_count = %follow_up_tool_calls.len(),
                "Received complete LLM follow-up response"
            );

            // Store assistant response with tool call information in structured JSON format
            // This allows proper conversation reconstruction and consistent UI display
            let enhanced_assistant_response = if !follow_up_tool_calls.is_empty() {
                // Store all providers in the same structured JSON format
                let assistant_data = serde_json::json!({
                    "text": assistant_response,
                    "tool_calls": follow_up_tool_calls
                });
                serde_json::to_string(&assistant_data)
                    .unwrap_or_else(|_| assistant_response.clone())
            } else {
                assistant_response.clone()
            };

            self.db.create_llm_agent_message(
                session_id,
                "assistant",
                enhanced_assistant_response.clone(),
            )?;
            tracing::debug!(
                session_id = %session_id,
                response_length = %enhanced_assistant_response.len(),
                follow_up_tool_calls_count = %follow_up_tool_calls.len(),
                "Follow-up assistant response with tool call info stored in database"
            );

            // Process native tool calls from follow-up response
            if !follow_up_tool_calls.is_empty() {
                tracing::info!(
                    session_id = %session_id,
                    tool_calls_count = %follow_up_tool_calls.len(),
                    "Processing native tool calls from follow-up response"
                );
                self.process_native_tool_calls(session_id, &follow_up_tool_calls)
                    .await?;
            } else {
                tracing::debug!(
                    session_id = %session_id,
                    "No tool calls found in follow-up response"
                );
            }

            tracing::info!(
                session_id = %session_id,
                work_id = %session.work_id,
                "Successfully completed LLM follow-up after tool execution"
            );

            Ok(assistant_response)
        })
    }

    /// Clean up assistant response by removing unwanted prefixes
    fn clean_assistant_response(&self, response: &str) -> String {
        let cleaned = response.trim();

        // Remove "Making tool calls:" prefix if present
        let without_prefix = if let Some(stripped) = cleaned.strip_prefix("Making tool calls:") {
            stripped.trim()
        } else {
            cleaned
        };

        // Remove any leading/trailing whitespace and return
        without_prefix.trim().to_string()
    }

    /// Get API key for provider
    fn get_api_key(&self, provider: &str) -> Result<String> {
        let provider_lower = provider.to_lowercase();

        // First try to get from config file
        if let Some(api_keys) = &self.config.api_keys {
            let config_key = match provider_lower.as_str() {
                "xai" => &api_keys.xai_api_key,
                "openai" => &api_keys.openai_api_key,
                "anthropic" | "claude" => &api_keys.anthropic_api_key,
                _ => &None,
            };

            if let Some(key) = config_key {
                if !key.is_empty() {
                    return Ok(key.clone());
                }
            }
        }

        // Fallback to environment variables
        let env_var = match provider_lower.as_str() {
            "grok" | "xai" => "XAI_API_KEY",
            "openai" => "OPENAI_API_KEY",
            "anthropic" | "claude" => "ANTHROPIC_API_KEY",
            _ => {
                return Err(anyhow::anyhow!(
                    "No API key configured for provider: {}",
                    provider
                ))
            }
        };

        std::env::var(env_var).map_err(|_| anyhow::anyhow!(
            "No API key configured for provider: {}. Please set it in ~/.config/nocodo/manager.toml [api_keys] section or {} environment variable",
            provider,
            env_var
        ))
    }

    /// Get base URL for provider
    fn get_base_url(&self, provider: &str) -> Option<String> {
        match provider.to_lowercase().as_str() {
            "grok" | "xai" => Some("https://api.x.ai".to_string()),
            "openai" => None, // Use default OpenAI URL
            "anthropic" | "claude" => Some("https://api.anthropic.com".to_string()),
            _ => None,
        }
    }

    /// Fail a session
    #[allow(dead_code)]
    pub async fn fail_session(&self, session_id: i64) -> Result<()> {
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

    /// Create native tool definitions for supported providers
    fn create_native_tool_definitions(&self) -> Vec<crate::llm_client::ToolDefinition> {
        use crate::models::{GrepRequest, ListFilesRequest, ReadFileRequest, WriteFileRequest};

        vec![
            crate::llm_client::ToolDefinition {
                r#type: "function".to_string(),
                function: crate::llm_client::FunctionDefinition {
                    name: "list_files".to_string(),
                    description: "List files and directories in a given path".to_string(),
                    parameters: serde_json::to_value(ListFilesRequest::example_schema())
                        .unwrap_or_default(),
                },
            },
            crate::llm_client::ToolDefinition {
                r#type: "function".to_string(),
                function: crate::llm_client::FunctionDefinition {
                    name: "read_file".to_string(),
                    description: "Read the contents of a file".to_string(),
                    parameters: serde_json::to_value(ReadFileRequest::example_schema())
                        .unwrap_or_default(),
                },
            },
            crate::llm_client::ToolDefinition {
                r#type: "function".to_string(),
                function: crate::llm_client::FunctionDefinition {
                    name: "write_file".to_string(),
                    description: "Write or modify a file".to_string(),
                    parameters: serde_json::to_value(WriteFileRequest::example_schema())
                        .unwrap_or_default(),
                },
            },
            crate::llm_client::ToolDefinition {
                r#type: "function".to_string(),
                function: crate::llm_client::FunctionDefinition {
                    name: "grep".to_string(),
                    description: "Search for patterns in files using grep".to_string(),
                    parameters: serde_json::to_value(GrepRequest::example_schema())
                        .unwrap_or_default(),
                },
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use crate::models::{ListFilesRequest, ReadFileRequest, ToolRequest};

    #[test]
    fn test_tool_request_serialization_roundtrip() {
        // Test ListFilesRequest
        let list_request = ToolRequest::ListFiles(ListFilesRequest {
            path: "src".to_string(),
            recursive: Some(true),
            include_hidden: Some(false),
            max_files: None,
        });

        let json_str = serde_json::to_string(&list_request).expect("Failed to serialize");
        let parsed = serde_json::from_str::<ToolRequest>(&json_str).expect("Failed to deserialize");

        match parsed {
            ToolRequest::ListFiles(req) => {
                assert_eq!(req.path, "src");
                assert_eq!(req.recursive, Some(true));
                assert_eq!(req.include_hidden, Some(false));
            }
            _ => panic!("Wrong tool request type"),
        }

        // Test ReadFileRequest
        let read_request = ToolRequest::ReadFile(ReadFileRequest {
            path: "README.md".to_string(),
            max_size: Some(5000),
        });

        let json_str = serde_json::to_string(&read_request).expect("Failed to serialize");
        let parsed = serde_json::from_str::<ToolRequest>(&json_str).expect("Failed to deserialize");

        match parsed {
            ToolRequest::ReadFile(req) => {
                assert_eq!(req.path, "README.md");
                assert_eq!(req.max_size, Some(5000));
            }
            _ => panic!("Wrong tool request type"),
        }
    }
}
