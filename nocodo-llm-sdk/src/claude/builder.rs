use crate::{
    claude::{
        client::ClaudeClient,
        tools::ClaudeToolFormat,
        types::{ClaudeContentBlock, ClaudeMessage, ClaudeMessageRequest, ClaudeRole, ClaudeTool},
    },
    error::LlmError,
    tools::{ProviderToolFormat, Tool, ToolChoice, ToolResult},
};

/// Builder for creating Claude message requests
pub struct MessageBuilder<'a> {
    client: &'a ClaudeClient,
    model: Option<String>,
    max_tokens: Option<u32>,
    messages: Vec<ClaudeMessage>,
    system: Option<String>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    stop_sequences: Option<Vec<String>>,
    tools: Option<Vec<ClaudeTool>>,
    tool_choice: Option<serde_json::Value>,
}

impl<'a> MessageBuilder<'a> {
    /// Create a new message builder
    pub fn new(client: &'a ClaudeClient) -> Self {
        Self {
            client,
            model: None,
            max_tokens: None,
            messages: Vec::new(),
            system: None,
            temperature: None,
            top_p: None,
            stop_sequences: None,
            tools: None,
            tool_choice: None,
        }
    }

    /// Set the model to use
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the maximum number of tokens to generate
    pub fn max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Add a message to the conversation
    ///
    /// Note: Only "user" and "assistant" roles are valid for messages.
    /// System messages should be set using the `system()` method instead.
    /// Invalid roles will be treated as "user" by default.
    pub fn message(mut self, role: impl Into<String>, content: impl Into<String>) -> Self {
        let role_str = role.into();
        let role = match role_str.as_str() {
            "user" => ClaudeRole::User,
            "assistant" => ClaudeRole::Assistant,
            _ => {
                // Log warning and default to User role instead of panicking
                tracing::warn!("Invalid role '{}', defaulting to 'user'", role_str);
                ClaudeRole::User
            }
        };

        let content = vec![ClaudeContentBlock::Text {
            text: content.into(),
        }];

        self.messages.push(ClaudeMessage { role, content });
        self
    }

    /// Add a user message
    pub fn user_message(self, content: impl Into<String>) -> Self {
        self.message("user", content)
    }

    /// Add an assistant message
    pub fn assistant_message(self, content: impl Into<String>) -> Self {
        self.message("assistant", content)
    }

    /// Set the system prompt
    pub fn system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }

    /// Set the temperature for randomness
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set the top-p sampling parameter
    pub fn top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }

    /// Set custom stop sequences
    pub fn stop_sequences(mut self, stop_sequences: Vec<String>) -> Self {
        self.stop_sequences = Some(stop_sequences);
        self
    }

    /// Add a tool to the request
    pub fn tool(mut self, tool: Tool) -> Self {
        let tools = self.tools.get_or_insert_with(Vec::new);
        tools.push(ClaudeToolFormat::to_provider_tool(&tool));
        self
    }

    /// Add multiple tools to the request
    pub fn tools(mut self, tools: Vec<Tool>) -> Self {
        for tool in tools {
            self = self.tool(tool);
        }
        self
    }

    /// Set tool choice strategy
    pub fn tool_choice(mut self, choice: ToolChoice) -> Self {
        self.tool_choice = Some(ClaudeToolFormat::to_provider_tool_choice(&choice));
        self
    }

    /// Add a tool result to continue the conversation
    pub fn tool_result(mut self, result: ToolResult) -> Self {
        // For Claude, tool results are added as user messages with tool_result content
        // This is a simplified approach - in practice, Claude expects tool results in a specific format
        let content = format!(
            "Tool result for {}: {}",
            result.tool_call_id(),
            result.content()
        );
        self.messages.push(ClaudeMessage::user(content));
        self
    }

    /// Send the request and get the response
    pub async fn send(self) -> Result<crate::claude::types::ClaudeMessageResponse, LlmError> {
        let request = ClaudeMessageRequest {
            model: self
                .model
                .ok_or_else(|| LlmError::invalid_request("Model must be specified"))?,
            max_tokens: self
                .max_tokens
                .ok_or_else(|| LlmError::invalid_request("max_tokens must be specified"))?,
            messages: self.messages,
            system: self.system,
            temperature: self.temperature,
            top_p: self.top_p,
            stop_sequences: self.stop_sequences,
            tools: self.tools,
            tool_choice: self.tool_choice,
        };

        self.client.create_message(request).await
    }
}

impl ClaudeClient {
    /// Start building a message request
    pub fn message_builder(&self) -> MessageBuilder<'_> {
        MessageBuilder::new(self)
    }
}
