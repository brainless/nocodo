use crate::config::AppConfig;
use crate::database::Database;
use crate::llm_client::{create_llm_client, CompletionRequest, ContentBlock, Message, Role};
use crate::models::{LlmAgentSession, LlmProviderConfig};
use crate::websocket::WebSocketBroadcaster;

use anyhow::Result;
use manager_tools::bash::BashExecutor;
use manager_tools::bash::BashPermissions;
use manager_tools::ToolExecutor;
use std::boxed::Box;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

/// LLM Agent that handles direct communication with LLMs and tool execution
/// 
/// ## Tool Execution Status
/// 
/// Tool execution is currently **disabled** due to SDK integration complexity:
/// 
/// 1. **SDK Tool Integration**: The new trait-based SDK architecture requires
///    proper conversion between our tool definitions and each provider's
///    tool calling format (OpenAI function calling, Anthropic tools, etc.)
/// 
/// 2. **Provider Compatibility**: Different providers have different tool
///    calling mechanisms that need to be handled in the SDK layer
/// 
/// 3. **Future Re-enablement**: All tool execution code is preserved with
///    `#[allow(dead_code)]` attributes and will be re-enabled once SDK
///    tool integration is complete
/// 
/// The agent can still handle conversations and store messages, but cannot
/// execute tools until the SDK tool integration is implemented.
#[allow(dead_code)] // Tool execution disabled - fields will be used when re-enabled
pub struct LlmAgent {
    db: Arc<Database>,
    ws: Arc<WebSocketBroadcaster>,
    tool_executor: ToolExecutor,
    bash_executor: BashExecutor,
    config: Arc<AppConfig>,
}

impl LlmAgent {
    pub fn new(
        db: Arc<Database>,
        ws: Arc<WebSocketBroadcaster>,
        project_path: PathBuf,
        config: Arc<AppConfig>,
    ) -> Self {
        // Initialize bash permissions with default safe rules
        let bash_permissions = BashPermissions::default();

        // Initialize bash executor with 30 second default timeout
        let bash_executor =
            BashExecutor::new(bash_permissions, 30).expect("Failed to initialize bash executor");

        let tool_executor =
            ToolExecutor::new(project_path).with_bash_executor(Box::new(bash_executor.clone()));

        Self {
            db,
            ws,
            tool_executor,
            bash_executor,
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
        // Omit temperature for zAI/GLM to avoid floating point precision issues
        let temperature = if session.provider.to_lowercase() == "zai" {
            None
        } else {
            Some(0.7)
        };

        let config = LlmProviderConfig {
            provider: session.provider.clone(),
            model: session.model.clone(),
            api_key: self.get_api_key(&session.provider)?,
            base_url: self.get_base_url(&session.provider),
            max_tokens: Some(4000),
            temperature,
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

        // Build conversation for LLM using SDK types
        let mut messages = Vec::new();
        for msg in &history {
            // Parse assistant messages to extract tool calls from stored JSON format
            let (content, _tool_calls) = if msg.role == "assistant" {
                if let Ok(assistant_data) = serde_json::from_str::<serde_json::Value>(&msg.content)
                {
                    // Extract text content
                    let text = assistant_data
                        .get("text")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    // Extract tool calls if present (for now, just extract text content)
                    // TODO: Convert SDK tool calls to proper format when needed
                    let _tool_calls = if let Some(tool_calls_array) =
                        assistant_data.get("tool_calls").and_then(|v| v.as_array())
                    {
                        // Store tool calls but don't process them for now
                        tracing::debug!(
                            session_id = %session_id,
                            tool_calls_count = %tool_calls_array.len(),
                            "Found tool calls in assistant message"
                        );
                        Some(tool_calls_array.len())
                    } else {
                        None
                    };

                    (text, _tool_calls)
                } else {
                    // Not JSON format, use content as-is
                    (msg.content.clone(), None)
                }
            } else {
                // Non-assistant messages use content as-is
                (msg.content.clone(), None)
            };

            // Convert to SDK Message format
            let role = match msg.role.as_str() {
                "user" => Role::User,
                "assistant" => Role::Assistant,
                "system" => Role::System,
                _ => continue, // Skip unknown roles
            };

            messages.push(Message {
                role,
                content: vec![ContentBlock::Text { text: content }],
            });
        }

        tracing::debug!(
            session_id = %session_id,
            total_messages = %messages.len(),
            "Built conversation for LLM request"
        );

        // Log the full conversation being sent to LLM (truncated for large messages)
        for (i, msg) in messages.iter().enumerate() {
            let text_content: String = msg
                .content
                .iter()
                .filter_map(|block| match block {
                    crate::llm_client::ContentBlock::Text { text } => Some(text.clone()),
                    _ => None,
                })
                .collect();

            let content_preview = if text_content.len() > 200 {
                format!("{}...", &text_content[..200])
            } else {
                text_content.clone()
            };

            let content_length = text_content.len();
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
        // TODO: Implement SDK tool integration
        let _tools = Vec::<nocodo_llm_sdk::tools::Tool>::new(); // No tools for now

        // Determine temperature based on provider
        // GLM API has issues with floating point precision, so omit it to use API default
        let temperature = if session.provider.to_lowercase() == "zai" {
            None
        } else {
            Some(0.3)
        };

        // Get system message from messages if present
        let system_message = messages
            .iter()
            .position(|msg| matches!(msg.role, Role::System))
            .and_then(|pos| {
                if let Some(Message {
                    role: Role::System,
                    content,
                }) = messages.get(pos)
                {
                    content.first().and_then(|block| match block {
                        ContentBlock::Text { text } => Some(text.clone()),
                        _ => None,
                    })
                } else {
                    None
                }
            });

        // Filter out system message from regular messages (SDK handles it separately)
        let filtered_messages: Vec<Message> = messages
            .into_iter()
            .filter(|msg| !matches!(msg.role, Role::System))
            .collect();

        let request = CompletionRequest {
            messages: filtered_messages,
            max_tokens: 4000,
            model: session.model.clone(),
            system: system_message,
            temperature,
            top_p: None,
            stop_sequences: None,
        };

        tracing::info!(
            session_id = %session_id,
            provider = %session.provider,
            model = %session.model,
            "Sending request to LLM provider"
        );

        // Get the complete response (non-streaming)
        let response = llm_client.complete(request).await?;

        // Extract text content from SDK response
        let raw_assistant_response = response
            .content
            .iter()
            .filter_map(|block| match block {
                ContentBlock::Text { text } => Some(text),
                _ => None,
            })
            .cloned()
            .collect::<Vec<String>>()
            .join("\n");

        // Clean up assistant response by removing unwanted prefixes
        let assistant_response = self.clean_assistant_response(&raw_assistant_response);

        // TODO: Extract tool calls from SDK response when needed
        // Tool calls disabled for now
        let accumulated_tool_calls = Vec::<()>::new(); // Empty placeholder

        // Debug logging for tool call extraction
        tracing::info!(
            session_id = %session_id,
            extracted_tool_calls_count = %accumulated_tool_calls.len(),
            content_blocks_count = %response.content.len(),
            "Extracted tool calls from LLM response"
        );

        // Log details of response structure for debugging
        for (block_idx, block) in response.content.iter().enumerate() {
            match block {
                ContentBlock::Text { text } => {
                    tracing::info!(
                        session_id = %session_id,
                        block_index = %block_idx,
                        block_type = "text",
                        content_length = %text.len(),
                        content_preview = %if text.len() > 100 {
                            format!("{}...", &text[..100])
                        } else {
                            text.clone()
                        },
                        "Response content block details"
                    );
                }
                ContentBlock::Image {
                    content_type,
                    source,
                } => {
                    tracing::info!(
                        session_id = %session_id,
                        block_index = %block_idx,
                        block_type = "image",
                        content_type = %content_type,
                        media_type = %source.media_type,
                        "Response content block details (image)"
                    );
                }
            }
        }

        tracing::info!(
            session_id = %session_id,
            response_role = ?response.role,
            input_tokens = %response.usage.input_tokens,
            output_tokens = %response.usage.output_tokens,
            stop_reason = ?response.stop_reason,
            "Response metadata"
        );

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

        // Tool calls disabled - store plain assistant response
        let enhanced_assistant_response = assistant_response.clone();

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

        // Tool calls are disabled for now until SDK tool integration is implemented
        if false {
            // Always false since tool calls are disabled
            tracing::warn!(
                session_id = %session_id,
                tool_calls_count = %accumulated_tool_calls.len(),
                "Tool calls found but SDK tool integration not yet implemented"
            );
        } else {
            tracing::info!(
                session_id = %session_id,
                response_length = %assistant_response.len(),
                response_preview = %if assistant_response.len() > 100 {
                    format!("{}...", &assistant_response[..100])
                } else {
                    assistant_response.clone()
                },
                "No tool calls found in initial response - stored plain text response"
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
    #[allow(dead_code)] // Tool execution disabled - will be used when re-enabled
    async fn get_tool_executor_for_session(&self, session_id: i64) -> Result<ToolExecutor> {
        // Get session to find work_id
        let session = self.db.get_llm_agent_session(session_id)?;

        // Get work to find working directory
        let work = self.db.get_work_by_id(session.work_id)?;

        // Use working_directory if set, otherwise fall back to project.path or default
        let mut executor = if let Some(working_directory) = work.working_directory {
            ToolExecutor::new(PathBuf::from(working_directory))
        } else if let Some(project_id) = work.project_id {
            // Fallback to project path for backward compatibility with old Work items
            let project = self.db.get_project_by_id(project_id)?;
            ToolExecutor::new(PathBuf::from(project.path))
        } else {
            // Fallback to default tool executor
            ToolExecutor::new(self.tool_executor.base_path().clone())
        };

        // Attach bash executor if available
        executor = executor.with_bash_executor(Box::new(self.bash_executor.clone()));

        Ok(executor)
    }

    /// Process native tool calls from LLM response
    #[allow(dead_code)] // Tool execution disabled - will be used when re-enabled
    async fn process_native_tool_calls(
        &self,
        session_id: i64,
        _tool_calls: &[nocodo_llm_sdk::tools::ToolCall],
        _depth: u32,
    ) -> Result<()> {
        // Tool calls are disabled for now until SDK tool integration is implemented
        tracing::warn!(
            session_id = %session_id,
            "Tool calls disabled - SDK tool integration not yet implemented"
        );
        Ok(())
    }

    /// Follow up with LLM after tool execution with recursion depth tracking
    #[allow(dead_code)] // Tool execution disabled - will be used when re-enabled
    fn follow_up_with_llm_with_depth<'a>(
        &'a self,
        session_id: i64,
        depth: u32,
    ) -> Pin<Box<dyn Future<Output = Result<String>> + Send + 'a>> {
        Box::pin(async move {
            const MAX_RECURSION_DEPTH: u32 = 5; // Prevent infinite loops

            tracing::error!(  // Use error to ensure it's visible
                session_id = %session_id,
                depth = %depth,
                max_depth = %MAX_RECURSION_DEPTH,
                "Follow-up recursion depth limit reached - requesting final response from LLM"
            );

            // Make one final call to LLM to get a summary/conclusion based on gathered information
            // Use tool_choice: None to prevent more tool calls
            let history = self.db.get_llm_agent_messages(session_id)?;

            // Convert database messages to SDK Message format for final response
            let mut messages = Vec::new();
            for msg in &history {
                if msg.role == "system" {
                    continue; // Skip system messages in follow-up
                }

                let role = match msg.role.as_str() {
                    "user" => nocodo_llm_sdk::types::Role::User,
                    "assistant" => nocodo_llm_sdk::types::Role::Assistant,
                    _ => continue,
                };

                messages.push(nocodo_llm_sdk::types::Message {
                    role,
                    content: vec![nocodo_llm_sdk::types::ContentBlock::Text {
                        text: msg.content.clone(),
                    }],
                });
            }

            let session = self.db.get_llm_agent_session(session_id)?;
            let config = LlmProviderConfig {
                provider: session.provider.clone(),
                api_key: self.get_api_key(&session.provider)?,
                base_url: self.get_base_url(&session.provider),
                model: session.model.clone(),
                max_tokens: None,
                temperature: None,
            };
            let llm_client = create_llm_client(config)?;

            let final_request = nocodo_llm_sdk::types::CompletionRequest {
                messages,
                max_tokens: 2000,
                model: session.model.clone(),
                system: None, // System messages already in conversation
                temperature: Some(0.7),
                top_p: None,
                stop_sequences: None,
            };

            match llm_client.complete(final_request).await {
                Ok(response) => {
                    let content = response
                        .content
                        .iter()
                        .filter_map(|block| match block {
                            nocodo_llm_sdk::types::ContentBlock::Text { text } => {
                                Some(text.clone())
                            }
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("\n");

                    Ok(content)
                }
                Err(e) => {
                    tracing::error!(
                        session_id = %session_id,
                        error = %e,
                        "Failed to get final response from LLM"
                    );
                    Err(anyhow::anyhow!(
                        "Failed to get final response from LLM: {}",
                        e
                    ))
                }
            }
        })
    }

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
                "zai" => &api_keys.zai_api_key,
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
            "zai" => "ZAI_API_KEY",
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
            "zai" => {
                // Check if using GLM Coding Plan subscription
                let use_coding_plan = self
                    .config
                    .api_keys
                    .as_ref()
                    .and_then(|keys| keys.zai_coding_plan)
                    .unwrap_or(false);

                let base_url = if use_coding_plan {
                    "https://api.z.ai/api/coding".to_string()
                } else {
                    "https://api.z.ai/api".to_string()
                };

                tracing::info!(
                    provider = provider,
                    coding_plan = use_coding_plan,
                    base_url = %base_url,
                    "Selected zAI base URL"
                );

                Some(base_url)
            }
            _ => None,
        }
    }

    /// Reconstruct conversation history for follow-up LLM calls with proper tool call handling
    #[allow(dead_code)] // Tool execution disabled - will be used when re-enabled
    fn reconstruct_conversation_for_followup(
        &self,
        history: &[crate::models::LlmAgentMessage],
        session_id: i64,
    ) -> Result<Vec<Message>> {
        let mut messages = Vec::new();
        let mut tool_call_map = std::collections::HashMap::new();

        tracing::info!(
            session_id = %session_id,
            message_count = %history.len(),
            "CLAUDE_DEBUG: Starting conversation reconstruction for follow-up"
        );

        // First pass: collect tool calls from assistant messages
        for msg in history {
            if msg.role == "assistant" {
                if let Ok(assistant_data) = serde_json::from_str::<serde_json::Value>(&msg.content)
                {
                    if let Some(tool_calls_array) =
                        assistant_data.get("tool_calls").and_then(|v| v.as_array())
                    {
                        for tool_call in tool_calls_array {
                            if let Some(id) = tool_call.get("id").and_then(|v| v.as_str()) {
                                tool_call_map.insert(id.to_string(), tool_call.clone());
                            }
                        }
                    }
                }
            }
        }

        // Second pass: reconstruct messages with proper tool call information
        for msg in history {
            match msg.role.as_str() {
                "assistant" => {
                    // Parse assistant message to extract text and tool calls
                    if let Ok(assistant_data) =
                        serde_json::from_str::<serde_json::Value>(&msg.content)
                    {
                        let text = assistant_data
                            .get("text")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();

                        // Tool calls disabled for now - SDK tool integration not yet implemented
                        let _tool_calls: Option<Vec<nocodo_llm_sdk::tools::ToolCall>> = None;

                        // Convert to SDK Message format
                        let role = Role::Assistant;
                        let content = if text.is_empty() { "" } else { &text };
                        messages.push(Message {
                            role,
                            content: vec![ContentBlock::Text {
                                text: content.to_string(),
                            }],
                        });
                    } else {
                        // Fallback for non-JSON content - convert to SDK Message
                        let role = Role::Assistant;
                        messages.push(Message {
                            role,
                            content: vec![ContentBlock::Text {
                                text: msg.content.clone(),
                            }],
                        });
                    }
                }
                // Skip tool messages for now - SDK handles tool results differently
                "tool" => {
                    tracing::debug!(
                        session_id = %session_id,
                        tool_content = %msg.content,
                        "Skipping tool message in conversation reconstruction (SDK handles tool results differently)"
                    );
                    continue;
                }
                _ => {
                    // System and user messages - convert to SDK Message
                    let role = match msg.role.as_str() {
                        "user" => Role::User,
                        "system" => Role::System,
                        _ => Role::User, // Default to user for unknown roles
                    };
                    messages.push(Message {
                        role,
                        content: vec![ContentBlock::Text {
                            text: msg.content.clone(),
                        }],
                    });
                }
            }
        }

        tracing::debug!(
            session_id = %session_id,
            total_messages = %messages.len(),
            tool_calls_found = %tool_call_map.len(),
            "Reconstructed conversation for follow-up with proper tool call handling"
        );

        Ok(messages)
    }

    /// Create native tool definitions for supported providers
    #[allow(dead_code)] // Tool execution disabled - will be used when re-enabled
    fn create_native_tool_definitions(&self, provider: &str) -> Vec<serde_json::Value> {
        use crate::models::{
            ApplyPatchRequest, BashRequest, GrepRequest, ListFilesRequest, ReadFileRequest,
            WriteFileRequest,
        };
        use crate::schema_provider::get_schema_provider;
        use schemars::schema_for;

        // Support progressive testing via environment variable:
        // ENABLE_TOOLS=none - No tools (tests basic chat)
        // ENABLE_TOOLS=list_files - Only list_files
        // ENABLE_TOOLS=list_read - list_files + read_file
        // ENABLE_TOOLS=all (default) - All tools
        let enable_tools = std::env::var("ENABLE_TOOLS").unwrap_or_else(|_| "all".to_string());

        tracing::info!(
            enable_tools = %enable_tools,
            provider = %provider,
            "Creating native tool definitions with ENABLE_TOOLS={} for provider={}",
            enable_tools, provider
        );

        // Get the schema provider for this LLM provider
        let _schema_provider = get_schema_provider(provider);

        // Helper closure to generate tool definition with provider-specific schema
        // Since Tool fields are private, use serde_json::Value placeholder for now
        let make_tool = |name: &str, _schema: schemars::schema::RootSchema| -> serde_json::Value {
            // Return tool definition as JSON value - SDK tool integration disabled
            serde_json::json!({
                "type": "function",
                "function": {
                    "name": name,
                    "description": format!("Tool: {}", name),
                    "parameters": {}
                }
            })
        };

        match enable_tools.as_str() {
            "none" => {
                tracing::info!("ENABLE_TOOLS=none: Returning NO tools for progressive testing");
                vec![]
            }
            "list_files" => {
                tracing::info!("ENABLE_TOOLS=list_files: Returning ONLY list_files tool");
                vec![make_tool("list_files", schema_for!(ListFilesRequest))]
            }
            "list_read" => {
                tracing::info!("ENABLE_TOOLS=list_read: Returning list_files + read_file tools");
                vec![
                    make_tool("list_files", schema_for!(ListFilesRequest)),
                    make_tool("read_file", schema_for!(ReadFileRequest)),
                ]
            }
            _ => {
                // "all" or any other value - return all tools
                tracing::info!(
                    "ENABLE_TOOLS={}: Returning ALL tools (default)",
                    enable_tools
                );
                vec![
                    make_tool("list_files", schema_for!(ListFilesRequest)),
                    make_tool("read_file", schema_for!(ReadFileRequest)),
                    make_tool("write_file", schema_for!(WriteFileRequest)),
                    make_tool("grep", schema_for!(GrepRequest)),
                    make_tool("apply_patch", schema_for!(ApplyPatchRequest)),
                    make_tool("bash", schema_for!(BashRequest)),
                ]
            }
        }
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
