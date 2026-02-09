use crate::{
    error::LlmError,
    llama_cpp::{
        client::LlamaCppClient,
        tools::LlamaCppToolFormat,
        types::{LlamaCppChatCompletionRequest, LlamaCppMessage, LlamaCppRole},
    },
    tools::{ProviderToolFormat, Tool},
};

/// Builder for creating llama.cpp chat completion requests
pub struct LlamaCppMessageBuilder<'a> {
    client: &'a LlamaCppClient,
    model: Option<String>,
    max_tokens: Option<u32>,
    messages: Vec<LlamaCppMessage>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    stop: Option<Vec<String>>,
    stream: Option<bool>,
    tools: Option<Vec<crate::openai::types::OpenAITool>>,
    parallel_tool_calls: Option<bool>,
}

impl<'a> LlamaCppMessageBuilder<'a> {
    /// Create a new message builder
    pub fn new(client: &'a LlamaCppClient) -> Self {
        Self {
            client,
            model: None,
            max_tokens: None,
            messages: Vec::new(),
            temperature: None,
            top_p: None,
            stop: None,
            stream: None,
            tools: None,
            parallel_tool_calls: None,
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
    pub fn message(mut self, role: impl Into<String>, content: impl Into<String>) -> Self {
        let role_str = role.into();
        let role = match role_str.as_str() {
            "system" => LlamaCppRole::System,
            "user" => LlamaCppRole::User,
            "assistant" => LlamaCppRole::Assistant,
            "tool" => LlamaCppRole::Tool,
            _ => {
                tracing::warn!("Invalid role '{}', defaulting to 'user'", role_str);
                LlamaCppRole::User
            }
        };

        self.messages.push(LlamaCppMessage::new(role, content));
        self
    }

    /// Add a system message
    pub fn system_message(self, content: impl Into<String>) -> Self {
        self.message("system", content)
    }

    /// Add a user message
    pub fn user_message(self, content: impl Into<String>) -> Self {
        self.message("user", content)
    }

    /// Add an assistant message
    pub fn assistant_message(self, content: impl Into<String>) -> Self {
        self.message("assistant", content)
    }

    /// Add a tool message
    pub fn tool_message(self, content: impl Into<String>) -> Self {
        self.message("tool", content)
    }

    /// Set temperature
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Set top-p sampling parameter
    pub fn top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }

    /// Set stop sequences
    pub fn stop_sequences(mut self, stop: Vec<String>) -> Self {
        self.stop = Some(stop);
        self
    }

    /// Enable or disable streaming
    pub fn stream(mut self, stream: bool) -> Self {
        self.stream = Some(stream);
        self
    }

    /// Add a tool to the request
    pub fn tool(mut self, tool: Tool) -> Self {
        let tools = self.tools.get_or_insert_with(Vec::new);
        tools.push(LlamaCppToolFormat::to_provider_tool(&tool));
        self
    }

    /// Add multiple tools to the request
    pub fn tools(mut self, tools: Vec<Tool>) -> Self {
        for tool in tools {
            self = self.tool(tool);
        }
        self
    }

    /// Enable or disable parallel tool calls
    pub fn parallel_tool_calls(mut self, enabled: bool) -> Self {
        self.parallel_tool_calls = Some(enabled);
        self
    }

    /// Send the request and get the response
    pub async fn send(
        self,
    ) -> Result<crate::llama_cpp::types::LlamaCppChatCompletionResponse, LlmError> {
        let request = LlamaCppChatCompletionRequest {
            model: self
                .model
                .ok_or_else(|| LlmError::invalid_request("Model must be specified"))?,
            messages: self.messages,
            max_tokens: self.max_tokens,
            temperature: self.temperature,
            top_p: self.top_p,
            stop: self.stop,
            stream: self.stream,
            tools: self.tools,
            parallel_tool_calls: self.parallel_tool_calls,
        };

        self.client.create_chat_completion(request).await
    }
}
