use crate::{
    error::LlmError,
    openai::{
        client::OpenAIClient,
        tools::{OpenAIResponseToolFormat, OpenAIToolFormat},
        types::{OpenAIResponseRequest, OpenAIResponseTool},
    },
    tools::{ProviderToolFormat, Tool, ToolChoice},
};

/// Builder for creating OpenAI Responses API requests
pub struct OpenAIResponseBuilder<'a> {
    client: &'a OpenAIClient,
    model: Option<String>,
    input: Option<String>,
    stream: Option<bool>,
    previous_response_id: Option<String>,
    background: Option<bool>,
    prompt_cache_retention: Option<String>,
    tools: Option<Vec<OpenAIResponseTool>>,
    tool_choice: Option<serde_json::Value>,
    parallel_tool_calls: Option<bool>,
}

impl<'a> OpenAIResponseBuilder<'a> {
    /// Create a new response builder
    pub fn new(client: &'a OpenAIClient) -> Self {
        Self {
            client,
            model: None,
            input: None,
            stream: None,
            previous_response_id: None,
            background: None,
            prompt_cache_retention: None,
            tools: None,
            tool_choice: None,
            parallel_tool_calls: None,
        }
    }

    /// Set the model to use
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the input text for the response
    pub fn input(mut self, input: impl Into<String>) -> Self {
        self.input = Some(input.into());
        self
    }

    /// Enable streaming
    pub fn stream(mut self, stream: bool) -> Self {
        self.stream = Some(stream);
        self
    }

    /// Set previous response ID for conversation continuation
    pub fn previous_response_id(mut self, id: impl Into<String>) -> Self {
        self.previous_response_id = Some(id.into());
        self
    }

    /// Enable background processing for long tasks
    pub fn background(mut self, background: bool) -> Self {
        self.background = Some(background);
        self
    }

    /// Set prompt cache retention
    pub fn prompt_cache_retention(mut self, retention: impl Into<String>) -> Self {
        self.prompt_cache_retention = Some(retention.into());
        self
    }

    /// Add a tool to the request
    pub fn tool(mut self, tool: Tool) -> Self {
        let tools = self.tools.get_or_insert_with(Vec::new);
        tools.push(OpenAIResponseToolFormat::to_response_tool(&tool));
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
        self.tool_choice = Some(OpenAIToolFormat::to_provider_tool_choice(&choice));
        self
    }

    /// Enable or disable parallel tool calls (default: true)
    pub fn parallel_tool_calls(mut self, enabled: bool) -> Self {
        self.parallel_tool_calls = Some(enabled);
        self
    }

    /// Send the request and get the response
    pub async fn send(self) -> Result<crate::openai::types::OpenAIResponseResponse, LlmError> {
        let request = OpenAIResponseRequest {
            model: self
                .model
                .ok_or_else(|| LlmError::invalid_request("Model must be specified"))?,
            input: self
                .input
                .ok_or_else(|| LlmError::invalid_request("Input must be specified"))?,
            stream: self.stream,
            previous_response_id: self.previous_response_id,
            background: self.background,
            prompt_cache_retention: self.prompt_cache_retention,
            tools: self.tools,
            tool_choice: self.tool_choice,
            parallel_tool_calls: self.parallel_tool_calls,
        };

        self.client.create_response(request).await
    }
}

impl OpenAIClient {
    /// Start building a responses API request
    pub fn response_builder(&self) -> OpenAIResponseBuilder<'_> {
        OpenAIResponseBuilder::new(self)
    }
}
