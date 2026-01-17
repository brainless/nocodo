use crate::{
    error::LlmError,
    grok::{
        tools::GrokToolFormat,
        types::{
            GrokChatCompletionRequest, GrokChatCompletionResponse, GrokMessage, GrokResponseFormat,
            GrokRole, GrokTool,
        },
        xai::XaiGrokClient,
        zen::ZenGrokClient,
    },
    tools::{ProviderToolFormat, Tool, ToolChoice, ToolResult},
};

/// Trait for Grok clients
pub trait GrokClientTrait {
    fn create_chat_completion(
        &self,
        request: GrokChatCompletionRequest,
    ) -> impl std::future::Future<Output = Result<GrokChatCompletionResponse, LlmError>> + Send;
}

/// Builder for creating Grok chat completion requests
pub struct GrokMessageBuilder<'a, T: GrokClientTrait> {
    client: &'a T,
    model: Option<String>,
    max_tokens: Option<u32>,
    messages: Vec<GrokMessage>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    stop: Option<Vec<String>>,
    stream: Option<bool>,
    tools: Option<Vec<GrokTool>>,
    tool_choice: Option<serde_json::Value>,
    response_format: Option<GrokResponseFormat>,
}

impl GrokClientTrait for XaiGrokClient {
    fn create_chat_completion(
        &self,
        request: GrokChatCompletionRequest,
    ) -> impl std::future::Future<Output = Result<GrokChatCompletionResponse, LlmError>> + Send
    {
        self.create_chat_completion(request)
    }
}

impl GrokClientTrait for ZenGrokClient {
    fn create_chat_completion(
        &self,
        request: GrokChatCompletionRequest,
    ) -> impl std::future::Future<Output = Result<GrokChatCompletionResponse, LlmError>> + Send
    {
        self.create_chat_completion(request)
    }
}

impl<'a, T: GrokClientTrait> GrokMessageBuilder<'a, T> {
    /// Create a new message builder
    pub fn new(client: &'a T) -> Self {
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
            tool_choice: None,
            response_format: None,
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
    /// Valid roles: "system", "user", "assistant"
    /// Invalid roles will be treated as "user" by default.
    pub fn message(mut self, role: impl Into<String>, content: impl Into<String>) -> Self {
        let role_str = role.into();
        let role = match role_str.as_str() {
            "system" => GrokRole::System,
            "user" => GrokRole::User,
            "assistant" => GrokRole::Assistant,
            _ => {
                // Log warning and default to User role instead of panicking
                tracing::warn!("Invalid role '{}', defaulting to 'user'", role_str);
                GrokRole::User
            }
        };

        self.messages.push(GrokMessage::new(role, content));
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
        self.stop = Some(stop_sequences);
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
        tools.push(GrokToolFormat::to_provider_tool(&tool));
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
        self.tool_choice = Some(GrokToolFormat::to_provider_tool_choice(&choice));
        self
    }

    /// Set the response format (text or JSON object)
    pub fn response_format(mut self, format: GrokResponseFormat) -> Self {
        self.response_format = Some(format);
        self
    }

    /// Add a tool result to continue the conversation
    pub fn tool_result(mut self, result: ToolResult) -> Self {
        self.messages.push(GrokMessage::tool_result(
            result.tool_call_id(),
            result.content(),
        ));
        self
    }

    /// Send the request and get the response
    pub async fn send(self) -> Result<crate::grok::types::GrokChatCompletionResponse, LlmError> {
        let request = GrokChatCompletionRequest {
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
            tool_choice: self.tool_choice,
            response_format: self.response_format,
        };

        self.client.create_chat_completion(request).await
    }
}

impl XaiGrokClient {
    /// Start building a chat completion request
    pub fn message_builder(&self) -> GrokMessageBuilder<'_, XaiGrokClient> {
        GrokMessageBuilder::new(self)
    }
}
