use crate::{
    error::LlmError,
    openai::{
        client::OpenAIClient,
        tools::{OpenAIResponseToolFormat, OpenAIToolFormat},
        types::{
            OpenAIChatCompletionRequest, OpenAIMessage, OpenAIResponseFormat,
            OpenAIResponseRequest, OpenAIResponseTool, OpenAIRole,
        },
    },
    tools::{ProviderToolFormat, Tool, ToolChoice, ToolResult},
};

/// Builder for creating OpenAI chat completion requests
pub struct OpenAIMessageBuilder<'a> {
    client: &'a OpenAIClient,
    model: Option<String>,
    max_tokens: Option<u32>,
    max_completion_tokens: Option<u32>,
    messages: Vec<OpenAIMessage>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    stop: Option<Vec<String>>,
    stream: Option<bool>,
    reasoning_effort: Option<String>,
    tools: Option<Vec<OpenAIResponseTool>>,
    tool_choice: Option<serde_json::Value>,
    parallel_tool_calls: Option<bool>,
    response_format: Option<OpenAIResponseFormat>,
}

impl<'a> OpenAIMessageBuilder<'a> {
    /// Create a new message builder
    pub fn new(client: &'a OpenAIClient) -> Self {
        Self {
            client,
            model: None,
            max_tokens: None,
            max_completion_tokens: None,
            messages: Vec::new(),
            temperature: None,
            top_p: None,
            stop: None,
            stream: None,
            reasoning_effort: None,
            tools: None,
            tool_choice: None,
            parallel_tool_calls: None,
            response_format: None,
        }
    }

    /// Set the model to use
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the maximum number of tokens to generate (legacy)
    pub fn max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set the maximum number of completion tokens to generate (recommended)
    pub fn max_completion_tokens(mut self, max_completion_tokens: u32) -> Self {
        self.max_completion_tokens = Some(max_completion_tokens);
        self
    }

    /// Add a message to the conversation
    ///
    /// Valid roles: "system", "user", "assistant", "tool"
    /// Invalid roles will be treated as "user" by default.
    pub fn message(mut self, role: impl Into<String>, content: impl Into<String>) -> Self {
        let role_str = role.into();
        let role = match role_str.as_str() {
            "system" => OpenAIRole::System,
            "user" => OpenAIRole::User,
            "assistant" => OpenAIRole::Assistant,
            "tool" => OpenAIRole::Tool,
            _ => {
                // Log warning and default to User role instead of panicking
                tracing::warn!("Invalid role '{}', defaulting to 'user'", role_str);
                OpenAIRole::User
            }
        };

        self.messages.push(OpenAIMessage::new(role, content));
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

    /// Set reasoning effort for GPT-5 models ("minimal", "low", "medium", "high")
    pub fn reasoning_effort(mut self, effort: impl Into<String>) -> Self {
        self.reasoning_effort = Some(effort.into());
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

    /// Set the response format (text or JSON object)
    pub fn response_format(mut self, format: OpenAIResponseFormat) -> Self {
        self.response_format = Some(format);
        self
    }

    /// Add a tool result to continue the conversation
    pub fn tool_result(mut self, result: ToolResult) -> Self {
        self.messages.push(OpenAIMessage::tool_result(
            result.tool_call_id(),
            result.content(),
        ));
        self
    }

    /// Continue a conversation from a previous response
    pub fn continue_from(
        mut self,
        response: &crate::openai::types::OpenAIChatCompletionResponse,
    ) -> Self {
        // Copy all messages from the previous response
        for choice in &response.choices {
            self.messages.push(choice.message.clone());
        }
        self
    }

    /// Send the request and get the response
    pub async fn send(
        self,
    ) -> Result<crate::openai::types::OpenAIChatCompletionResponse, LlmError> {
        let request = OpenAIChatCompletionRequest {
            model: self
                .model
                .ok_or_else(|| LlmError::invalid_request("Model must be specified"))?,
            messages: self.messages,
            max_tokens: self.max_tokens,
            max_completion_tokens: self.max_completion_tokens,
            temperature: self.temperature,
            top_p: self.top_p,
            stop: self.stop,
            stream: self.stream,
            reasoning_effort: self.reasoning_effort,
            tools: self.tools,
            tool_choice: self.tool_choice,
            parallel_tool_calls: self.parallel_tool_calls,
            response_format: self.response_format,
        };

        self.client.create_chat_completion(request).await
    }
}

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
    /// Start building a chat completion request
    pub fn message_builder(&self) -> OpenAIMessageBuilder<'_> {
        OpenAIMessageBuilder::new(self)
    }

    /// Start building a responses API request
    pub fn response_builder(&self) -> OpenAIResponseBuilder<'_> {
        OpenAIResponseBuilder::new(self)
    }
}
