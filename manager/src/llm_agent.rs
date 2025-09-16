use crate::database::Database;
use crate::llm_client::{create_llm_client, LlmCompletionRequest, LlmMessage};
use crate::models::{LlmAgentSession, LlmAgentToolCall, LlmProviderConfig, ToolRequest};
use crate::tools::ToolExecutor;
use crate::websocket::WebSocketBroadcaster;
use anyhow::Result;
use async_stream::try_stream;
use futures_util::StreamExt;
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

    /// Get the tool executor for a specific session's project
    async fn get_tool_executor_for_session(&self, session_id: &str) -> Result<ToolExecutor> {
        // Get session to find work_id
        let session = self.db.get_llm_agent_session(session_id)?;

        // Get work to find project_id
        let work = self.db.get_work_by_id(&session.work_id)?;

        if let Some(project_id) = work.project_id {
            // Get project to find project path
            let project = self.db.get_project_by_id(&project_id)?;
            Ok(ToolExecutor::new(PathBuf::from(project.path)))
        } else {
            // Fallback to the default tool executor
            Ok(ToolExecutor::new(self.tool_executor.base_path().clone()))
        }
    }

    /// Process tool calls from LLM response
    async fn process_tool_calls(&self, session_id: &str, response: &str) -> Result<()> {
        self.process_tool_calls_with_depth(session_id, response, 0)
            .await
    }

    /// Process tool calls from LLM response with recursion depth tracking
    fn process_tool_calls_with_depth<'a>(
        &'a self,
        session_id: &'a str,
        response: &'a str,
        depth: u32,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            const MAX_RECURSION_DEPTH: u32 = 5; // Prevent infinite loops

            if depth >= MAX_RECURSION_DEPTH {
                tracing::warn!(
                    session_id = %session_id,
                    current_depth = %depth,
                    max_depth = %MAX_RECURSION_DEPTH,
                    "Tool call recursion depth limit reached, stopping processing"
                );
                return Ok(());
            }
            tracing::info!(
                session_id = %session_id,
                current_depth = %depth,
                "Processing tool calls from LLM response"
            );

            // Extract JSON tool calls from response with retry mechanism
            let tool_calls = self
                .extract_tool_calls_with_retry(session_id, response, depth)
                .await?;
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
                let tool_request: ToolRequest = match serde_json::from_value(tool_call_json.clone())
                {
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

                // Get project-specific tool executor
                let project_tool_executor = self.get_tool_executor_for_session(session_id).await?;
                let tool_response = project_tool_executor.execute(tool_request).await;

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

                // Add tool response to conversation (with size limiting)
                let response_json_string = serde_json::to_string(&response_value)?;
                let truncated_response = if response_json_string.len() > 50000 {
                    // 50KB limit
                    // Truncate large responses to prevent LLM context overflow
                    tracing::warn!(
                        session_id = %session_id,
                        tool_call_id = %tool_call_id,
                        original_size = %response_json_string.len(),
                        "Tool response too large, truncating for LLM follow-up"
                    );

                    format!(
                    "{{\"truncated\": true, \"original_size\": {}, \"summary\": \"Response truncated due to size limit. First 1000 chars: {}...\"}}",
                    response_json_string.len(),
                    response_json_string.chars().take(1000).collect::<String>()
                )
                } else {
                    response_json_string
                };

                self.db
                    .create_llm_agent_message(session_id, "tool", truncated_response)?;
                tracing::debug!(
                    session_id = %session_id,
                    tool_call_id = %tool_call_id,
                    "Tool response added to conversation"
                );

                // If there are tool results, follow up with LLM
                tracing::info!(
                    session_id = %session_id,
                    tool_call_id = %tool_call_id,
                    current_depth = %depth,
                    "Following up with LLM after tool execution"
                );
                self.follow_up_with_llm_with_depth(session_id, depth + 1)
                    .await?;
            }

            tracing::info!(
                session_id = %session_id,
                "Completed processing all tool calls"
            );

            Ok(())
        })
    }

    /// Follow up with LLM after tool execution
    #[allow(dead_code)]
    async fn follow_up_with_llm(&self, session_id: &str) -> Result<String> {
        self.follow_up_with_llm_with_depth(session_id, 0).await
    }

    /// Follow up with LLM after tool execution with recursion depth tracking
    fn follow_up_with_llm_with_depth<'a>(
        &'a self,
        session_id: &'a str,
        depth: u32,
    ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>> {
        Box::pin(async move {
            const MAX_RECURSION_DEPTH: u32 = 5; // Prevent infinite loops

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
            let mut messages: Vec<_> = history
                .into_iter()
                .map(|msg| LlmMessage {
                    role: msg.role,
                    content: msg.content,
                })
                .collect();

            // Add tool system prompt for follow-up (same as initial request)
            let tool_system_prompt = self.create_tool_system_prompt();
            messages.push(LlmMessage {
                role: "system".to_string(),
                content: tool_system_prompt,
            });

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
            self.db.create_llm_agent_message(
                session_id,
                "assistant",
                assistant_response.clone(),
            )?;
            tracing::debug!(
                session_id = %session_id,
                response_length = %assistant_response.len(),
                "Follow-up assistant response stored in database"
            );

            // Check if the follow-up response contains tool calls
            if self.contains_tool_calls(&assistant_response) {
                tracing::info!(
                    session_id = %session_id,
                    "LLM follow-up response contains tool calls, processing them recursively"
                );
                self.process_tool_calls_with_depth(session_id, &assistant_response, depth + 1)
                    .await?;
            } else {
                tracing::debug!(
                    session_id = %session_id,
                    "LLM follow-up response does not contain tool calls"
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

    /// Create system prompt for tool usage
    fn create_tool_system_prompt(&self) -> String {
        use crate::models::{ListFilesRequest, ReadFileRequest, ToolRequest};
        use ts_rs::TS;

        // Generate TypeScript types for tools
        let tool_request_ts = ToolRequest::export_to_string()
            .unwrap_or_else(|_| "// Failed to generate ToolRequest type".to_string());
        let list_files_request_ts = ListFilesRequest::export_to_string()
            .unwrap_or_else(|_| "// Failed to generate ListFilesRequest type".to_string());
        let read_file_request_ts = ReadFileRequest::export_to_string()
            .unwrap_or_else(|_| "// Failed to generate ReadFileRequest type".to_string());

        format!(
            r#"You are an AI assistant with access to file system tools. You can use the following tools:

## Available Tools

The tools are defined using TypeScript types. When calling a tool, use the exact format shown below:

### Type Definitions

```typescript
// Individual tool request types
{list_files_request_ts}

{read_file_request_ts}

// Union type for all tool requests
{tool_request_ts}
```

### Tool Usage

When you need to use a tool, respond with ONLY the JSON request for that tool. Do not include any other text. The tool will be executed and you will receive the results, after which you can continue your response.

Examples:
- List files: {{"type": "list_files", "path": ".", "recursive": false}}
- Read file: {{"type": "read_file", "path": "src/main.rs", "max_size": 10000}}

### Guidelines

1. When you receive tool results (messages with role "tool"), analyze them and provide a helpful natural language response based on what you learned.
2. Always provide a complete answer to the user's original question using the information gathered from the tools.
3. Always analyze the project structure and read relevant files before providing code solutions.
4. Be concise and focus on the user's specific needs.

The tool request MUST exactly match the TypeScript interface defined above."#,
            list_files_request_ts = list_files_request_ts,
            read_file_request_ts = read_file_request_ts,
            tool_request_ts = tool_request_ts
        )
    }

    /// Check if response contains tool calls
    fn contains_tool_calls(&self, response: &str) -> bool {
        tracing::debug!(
            response_length = %response.len(),
            response_preview = %if response.len() > 200 {
                format!("{}...", &response[..200])
            } else {
                response.to_string()
            },
            "Checking if response contains tool calls"
        );

        // Look for JSON objects that might be tool calls - more flexible matching
        let contains_tool_keywords =
            response.contains("list_files") || response.contains("read_file");
        let contains_json_structure = response.contains("type") && response.contains("{");

        let result = contains_tool_keywords && contains_json_structure;

        tracing::info!(
            contains_tool_calls = %result,
            contains_tool_keywords = %contains_tool_keywords,
            contains_json_structure = %contains_json_structure,
            "Tool call detection result"
        );

        result
    }

    /// Extract tool calls from response with JSON error retry
    fn extract_tool_calls_with_retry<'a>(
        &'a self,
        session_id: &'a str,
        response: &'a str,
        depth: u32,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<serde_json::Value>>> + Send + 'a>> {
        Box::pin(async move {
            const MAX_JSON_RETRY_DEPTH: u32 = 3; // Maximum retries for JSON parsing errors

            let tool_calls = self.extract_tool_calls(response)?;

            // If we successfully extracted tool calls, return them
            if !tool_calls.is_empty() {
                return Ok(tool_calls);
            }

            // If we have no tool calls but the response contains tool keywords and JSON structure,
            // it might be malformed JSON that we should ask the LLM to fix
            if self.contains_tool_calls(response) && depth < MAX_JSON_RETRY_DEPTH {
                tracing::warn!(
                    session_id = %session_id,
                    retry_depth = %depth,
                    max_depth = %MAX_JSON_RETRY_DEPTH,
                    "No tool calls extracted but tool call detected - attempting JSON correction retry"
                );

                return self.retry_json_parsing(session_id, response, depth).await;
            }

            // No tool calls found and no retry needed
            Ok(tool_calls)
        })
    }

    /// Ask LLM to fix malformed JSON and retry parsing
    fn retry_json_parsing<'a>(
        &'a self,
        session_id: &'a str,
        malformed_response: &'a str,
        depth: u32,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<serde_json::Value>>> + Send + 'a>> {
        Box::pin(async move {
            use crate::models::ToolRequest;

            // Try to identify specific JSON parsing errors
            let mut error_details = Vec::new();

            // Test common JSON patterns to find specific errors
            if let Err(e) = serde_json::from_str::<ToolRequest>(malformed_response.trim()) {
                error_details.push(format!("Full response parse error: {}", e));
            }

            // Look for JSON-like structures and test them
            for line in malformed_response.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with('{') && trimmed.contains("type") {
                    if let Err(e) = serde_json::from_str::<ToolRequest>(trimmed) {
                        error_details.push(format!("Line '{}' parse error: {}", trimmed, e));
                    }
                }
            }

            let error_message = if error_details.is_empty() {
                "Could not identify specific JSON parsing errors".to_string()
            } else {
                error_details.join("; ")
            };

            tracing::info!(
                session_id = %session_id,
                retry_depth = %depth,
                error_details = %error_message,
                "Asking LLM to fix malformed JSON"
            );

            // Create a message asking the LLM to fix the JSON
            let fix_request = format!(
                r#"The previous response contained malformed JSON that could not be parsed. Please fix the JSON and provide a valid tool call.

Original response:
{}

Parsing errors:
{}

Please provide a corrected JSON tool call that follows the exact TypeScript interface format. The JSON must be valid and properly formatted with all required commas and quotation marks."#,
                malformed_response, error_message
            );

            // Add the error correction request to the session
            self.db
                .create_llm_agent_message(session_id, "user", fix_request)?;

            // Get corrected response from LLM using the same mechanism as follow-up
            let corrected_response = self
                .follow_up_with_llm_with_depth(session_id, depth)
                .await?;

            // Try to extract tool calls from the corrected response
            self.extract_tool_calls_with_retry(session_id, &corrected_response, depth + 1)
                .await
        })
    }

    /// Extract tool calls from response (internal method)
    fn extract_tool_calls(&self, response: &str) -> Result<Vec<serde_json::Value>> {
        use crate::models::ToolRequest;
        let mut tool_calls = Vec::new();
        let mut json_parsing_errors = Vec::new();

        tracing::debug!(
            response_length = %response.len(),
            "Extracting tool calls from response"
        );

        // Try parsing the entire response as a ToolRequest first
        match serde_json::from_str::<ToolRequest>(response.trim()) {
            Ok(tool_request) => {
                let json_value = serde_json::to_value(tool_request)?;
                tracing::info!("Successfully parsed full response as ToolRequest");
                tool_calls.push(json_value);
                return Ok(tool_calls);
            }
            Err(e) => {
                tracing::debug!(
                    error = %e,
                    "Failed to parse full response as ToolRequest, trying line-by-line"
                );
                json_parsing_errors.push((response.trim().to_string(), e.to_string()));
            }
        }

        // If that didn't work, try line-by-line extraction
        for (line_num, line) in response.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with('{') && trimmed.ends_with('}') {
                tracing::debug!(
                    line_number = %line_num,
                    line_content = %trimmed,
                    "Attempting to parse line as ToolRequest"
                );

                match serde_json::from_str::<ToolRequest>(trimmed) {
                    Ok(tool_request) => {
                        let json_value = serde_json::to_value(tool_request)?;
                        tracing::info!(
                            line_number = %line_num,
                            "Successfully parsed line as ToolRequest"
                        );
                        tool_calls.push(json_value);
                    }
                    Err(e) => {
                        tracing::debug!(
                            line_number = %line_num,
                            error = %e,
                            "Failed to parse line as ToolRequest"
                        );
                        json_parsing_errors.push((trimmed.to_string(), e.to_string()));
                    }
                }
            }
        }

        // Try to extract JSON blocks that span multiple lines
        if tool_calls.is_empty() {
            let mut brace_count = 0;
            let mut json_start = None;
            let chars: Vec<char> = response.chars().collect();

            for (i, &ch) in chars.iter().enumerate() {
                match ch {
                    '{' => {
                        if brace_count == 0 {
                            json_start = Some(i);
                        }
                        brace_count += 1;
                    }
                    '}' => {
                        brace_count -= 1;
                        if brace_count == 0 {
                            if let Some(start) = json_start {
                                let json_str: String = chars[start..=i].iter().collect();
                                tracing::debug!(
                                    json_candidate = %json_str,
                                    "Attempting to parse multi-line JSON block as ToolRequest"
                                );

                                match serde_json::from_str::<ToolRequest>(&json_str) {
                                    Ok(tool_request) => {
                                        let json_value = serde_json::to_value(tool_request)?;
                                        tracing::info!(
                                            "Successfully parsed multi-line JSON block as ToolRequest"
                                        );
                                        tool_calls.push(json_value);
                                    }
                                    Err(e) => {
                                        tracing::debug!(
                                            error = %e,
                                            "Failed to parse multi-line JSON block as ToolRequest"
                                        );
                                        json_parsing_errors.push((json_str, e.to_string()));
                                    }
                                }
                            }
                            json_start = None;
                        }
                    }
                    _ => {}
                }
            }
        }

        // If we have no tool calls but we have parsing errors, store them for potential retry
        if tool_calls.is_empty() && !json_parsing_errors.is_empty() {
            tracing::warn!(
                error_count = %json_parsing_errors.len(),
                "Failed to extract any tool calls due to JSON parsing errors"
            );

            for (json_str, error) in &json_parsing_errors {
                tracing::error!(
                    malformed_json = %json_str,
                    parse_error = %error,
                    "JSON parsing error detected"
                );
            }
        }

        tracing::info!(
            extracted_tool_calls = %tool_calls.len(),
            parsing_errors = %json_parsing_errors.len(),
            "Completed tool call extraction"
        );

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

#[cfg(test)]
mod tests {
    use crate::models::{ListFilesRequest, ReadFileRequest, ToolRequest};
    use ts_rs::TS;

    #[test]
    fn test_typescript_generation_for_list_files_request() {
        let ts_type = ListFilesRequest::export_to_string()
            .expect("Failed to generate TypeScript for ListFilesRequest");

        // Verify the generated TypeScript contains the expected structure
        assert!(ts_type.contains("export interface ListFilesRequest"));
        assert!(ts_type.contains("path: string"));
        assert!(ts_type.contains("recursive?: boolean"));
        assert!(ts_type.contains("include_hidden?: boolean"));

        // Ensure optional fields are marked correctly
        assert!(ts_type.contains("recursive?"));
        assert!(ts_type.contains("include_hidden?"));

        println!("Generated ListFilesRequest TypeScript:\n{}", ts_type);
    }

    #[test]
    fn test_typescript_generation_for_read_file_request() {
        let ts_type = ReadFileRequest::export_to_string()
            .expect("Failed to generate TypeScript for ReadFileRequest");

        // Verify the generated TypeScript contains the expected structure
        assert!(ts_type.contains("export interface ReadFileRequest"));
        assert!(ts_type.contains("path: string"));
        // Note: u64 maps to bigint in TypeScript, which is correct for large numbers
        assert!(ts_type.contains("max_size?"));
        // Should contain either bigint or number, depending on ts-rs version
        assert!(ts_type.contains("max_size?: bigint") || ts_type.contains("max_size?: number"));

        println!("Generated ReadFileRequest TypeScript:\n{}", ts_type);
    }

    #[test]
    fn test_typescript_generation_for_tool_request_union() {
        let ts_type =
            ToolRequest::export_to_string().expect("Failed to generate TypeScript for ToolRequest");

        // Verify the generated TypeScript contains the expected union structure
        assert!(ts_type.contains("ToolRequest"));
        assert!(ts_type.contains("list_files"));
        assert!(ts_type.contains("read_file"));
        assert!(ts_type.contains("ListFilesRequest"));
        assert!(ts_type.contains("ReadFileRequest"));

        println!("Generated ToolRequest TypeScript:\n{}", ts_type);
    }

    #[test]
    fn test_system_prompt_contains_generated_typescript() {
        // Test the system prompt generation directly without creating full LlmAgent
        use crate::models::{ListFilesRequest, ReadFileRequest, ToolRequest};
        use ts_rs::TS;

        // Generate TypeScript types for tools
        let tool_request_ts = ToolRequest::export_to_string()
            .unwrap_or_else(|_| "// Failed to generate ToolRequest type".to_string());
        let list_files_request_ts = ListFilesRequest::export_to_string()
            .unwrap_or_else(|_| "// Failed to generate ListFilesRequest type".to_string());
        let read_file_request_ts = ReadFileRequest::export_to_string()
            .unwrap_or_else(|_| "// Failed to generate ReadFileRequest type".to_string());

        let system_prompt = format!(
            r#"You are an AI assistant with access to file system tools. You can use the following tools:

## Available Tools

The tools are defined using TypeScript types. When calling a tool, use the exact format shown below:

### Type Definitions

```typescript
// Individual tool request types
{list_files_request_ts}

{read_file_request_ts}

// Union type for all tool requests
{tool_request_ts}
```

### Tool Usage

When you need to use a tool, respond with ONLY the JSON request for that tool. Do not include any other text. The tool will be executed and you will receive the results, after which you can continue your response.

Examples:
- List files: {{"type": "list_files", "path": ".", "recursive": false}}
- Read file: {{"type": "read_file", "path": "src/main.rs", "max_size": 10000}}

### Guidelines

1. When you receive tool results (messages with role "tool"), analyze them and provide a helpful natural language response based on what you learned.
2. Always provide a complete answer to the user's original question using the information gathered from the tools.
3. Always analyze the project structure and read relevant files before providing code solutions.
4. Be concise and focus on the user's specific needs.

The tool request MUST exactly match the TypeScript interface defined above."#,
            list_files_request_ts = list_files_request_ts,
            read_file_request_ts = read_file_request_ts,
            tool_request_ts = tool_request_ts
        );

        // Verify the system prompt contains TypeScript type definitions
        assert!(system_prompt.contains("TypeScript types"));
        assert!(system_prompt.contains("```typescript"));
        assert!(system_prompt.contains("ListFilesRequest"));
        assert!(system_prompt.contains("ReadFileRequest"));
        assert!(system_prompt.contains("ToolRequest"));

        // Verify it contains usage guidelines
        assert!(system_prompt.contains("When you need to use a tool"));
        assert!(system_prompt.contains("ONLY the JSON request"));
        assert!(system_prompt.contains("MUST exactly match the TypeScript interface"));

        println!("Generated system prompt:\n{}", system_prompt);
    }

    #[test]
    fn test_tool_call_extraction_logic() {
        use crate::models::ToolRequest;

        // Test direct parsing of valid tool requests
        let response1 = r#"{"type": "list_files", "path": ".", "recursive": true}"#;
        let parsed1 = serde_json::from_str::<ToolRequest>(response1);
        assert!(parsed1.is_ok());

        let response2 = r#"{"type": "read_file", "path": "src/main.rs", "max_size": 10000}"#;
        let parsed2 = serde_json::from_str::<ToolRequest>(response2);
        assert!(parsed2.is_ok());

        // Test invalid requests
        let invalid1 = r#"{"type": "invalid_tool", "path": "."}"#;
        let parsed_invalid1 = serde_json::from_str::<ToolRequest>(invalid1);
        assert!(parsed_invalid1.is_err());

        let invalid2 = r#"{"type": "list_files"}"#; // missing required path
        let parsed_invalid2 = serde_json::from_str::<ToolRequest>(invalid2);
        assert!(parsed_invalid2.is_err());
    }

    #[test]
    fn test_tool_request_serialization_roundtrip() {
        use crate::models::{ListFilesRequest, ReadFileRequest, ToolRequest};

        // Test ListFilesRequest
        let list_request = ToolRequest::ListFiles(ListFilesRequest {
            path: "src".to_string(),
            recursive: Some(true),
            include_hidden: Some(false),
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

    #[test]
    fn test_malformed_json_from_llm() {
        use crate::models::ToolRequest;

        // Test the exact malformed JSON you saw in the output
        let malformed_json = r#"{"type": "read_file "path": "/home/noc/Projects/rust-ui-test/README.md","max_size": 5000}"#;

        println!("Testing malformed JSON: {}", malformed_json);

        let result = serde_json::from_str::<ToolRequest>(malformed_json);

        match result {
            Ok(_) => {
                println!("Unexpectedly parsed successfully!");
                panic!("This JSON should have failed to parse");
            }
            Err(e) => {
                println!("JSON parsing error (as expected): {}", e);
                println!("Error kind: {:?}", e.classify());

                // This is actually a data error, not syntax - the JSON is valid but the enum variant is wrong
                assert!(e.is_data());

                // The error message should mention the unknown variant
                assert!(e.to_string().contains("unknown variant"));
                assert!(e.to_string().contains("read_file "));
            }
        }

        // Test a true syntax error - missing comma between fields
        let syntax_error_json = r#"{"type": "read_file", "path": "/test" "max_size": 1000}"#;
        println!("Testing syntax error JSON: {}", syntax_error_json);

        let result2 = serde_json::from_str::<ToolRequest>(syntax_error_json);

        match result2 {
            Ok(_) => panic!("This JSON should have failed to parse"),
            Err(e) => {
                println!("Syntax error (as expected): {}", e);
                println!("Error kind: {:?}", e.classify());
                assert!(e.is_syntax());
            }
        }

        // Test the missing comma at the beginning - the actual issue you observed
        let missing_comma_json = r#"{"type": "read_file" "path": "/home/noc/Projects/rust-ui-test/README.md","max_size": 5000}"#;
        println!("Testing missing comma JSON: {}", missing_comma_json);

        let result3 = serde_json::from_str::<ToolRequest>(missing_comma_json);

        match result3 {
            Ok(_) => panic!("This JSON should have failed to parse"),
            Err(e) => {
                println!("Missing comma error: {}", e);
                println!("Error kind: {:?}", e.classify());
                // This should be a syntax error
                assert!(e.is_syntax());
            }
        }
    }

    #[test]
    fn test_json_error_detection_and_messaging() {
        use crate::models::ToolRequest;

        // Test various malformed JSON scenarios and verify error messages
        let test_cases = vec![
            (
                r#"{"type": "read_file" "path": "/test.txt"}"#,
                "expected `,` or `}` at line 1 column 22",
                true, // is_syntax
            ),
            (
                r#"{"type": "invalid_tool", "path": "/test.txt"}"#,
                "unknown variant `invalid_tool`",
                false, // is_data, not syntax
            ),
            (
                r#"{"type": "list_files"}"#,
                "missing field `path`",
                false, // is_data, not syntax
            ),
            (
                r#"{"type": "read_file", "path": "/test.txt", "max_size": "not_a_number"}"#,
                "invalid type",
                false, // is_data, not syntax
            ),
        ];

        for (json_str, expected_error_fragment, should_be_syntax) in test_cases {
            println!("Testing JSON: {}", json_str);

            let result = serde_json::from_str::<ToolRequest>(json_str);

            match result {
                Ok(_) => panic!("Expected parsing to fail for: {}", json_str),
                Err(e) => {
                    let error_msg = e.to_string();
                    println!("Error: {}", error_msg);

                    // Check if the error message contains expected fragment
                    assert!(
                        error_msg
                            .to_lowercase()
                            .contains(&expected_error_fragment.to_lowercase()),
                        "Error '{}' should contain '{}'",
                        error_msg,
                        expected_error_fragment
                    );

                    // Check error classification
                    if should_be_syntax {
                        assert!(e.is_syntax(), "Error should be syntax error: {}", error_msg);
                    } else {
                        assert!(e.is_data(), "Error should be data error: {}", error_msg);
                    }
                }
            }
        }
    }

    #[test]
    fn test_json_retry_message_generation() {
        // Test that we can generate helpful error messages for the LLM
        let malformed_json = r#"{"type": "read_file" "path": "/home/noc/Projects/rust-ui-test/README.md","max_size": 5000}"#;

        // This simulates what our retry_json_parsing method would do
        let mut error_details = Vec::new();

        if let Err(e) = serde_json::from_str::<crate::models::ToolRequest>(malformed_json.trim()) {
            error_details.push(format!("Full response parse error: {}", e));
        }

        let error_message = error_details.join("; ");

        println!("Generated error message for LLM: {}", error_message);

        // Verify the error message contains useful information
        assert!(error_message.contains("expected `,` or `}`"));
        assert!(error_message.contains("line 1 column"));

        // Test that the fix request message is helpful
        let fix_request = format!(
            r#"The previous response contained malformed JSON that could not be parsed. Please fix the JSON and provide a valid tool call.

Original response:
{}

Parsing errors:
{}

Please provide a corrected JSON tool call that follows the exact TypeScript interface format. The JSON must be valid and properly formatted with all required commas and quotation marks."#,
            malformed_json, error_message
        );

        println!("Generated fix request:\n{}", fix_request);

        // Verify the fix request contains all necessary components
        assert!(fix_request.contains("malformed JSON"));
        assert!(fix_request.contains("Original response:"));
        assert!(fix_request.contains("Parsing errors:"));
        assert!(fix_request.contains("TypeScript interface format"));
        assert!(fix_request.contains(malformed_json));
        assert!(fix_request.contains(&error_message));
    }
}
