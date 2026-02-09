use crate::{
    error::LlmError,
    ollama::{
        client::OllamaClient,
        tools::OllamaToolFormat,
        types::{
            OllamaChatRequest, OllamaFormat, OllamaKeepAlive, OllamaMessage, OllamaOptions,
            OllamaRole, OllamaStop, OllamaThink,
        },
    },
    tools::{ProviderToolFormat, Tool, ToolResult},
};

/// Builder for creating Ollama chat requests
pub struct OllamaMessageBuilder<'a> {
    client: &'a OllamaClient,
    model: Option<String>,
    messages: Vec<OllamaMessage>,
    tools: Option<Vec<crate::openai::types::OpenAITool>>,
    format: Option<OllamaFormat>,
    options: OllamaOptions,
    has_options: bool,
    stream: Option<bool>,
    think: Option<OllamaThink>,
    keep_alive: Option<OllamaKeepAlive>,
    logprobs: Option<bool>,
    top_logprobs: Option<u32>,
}

impl<'a> OllamaMessageBuilder<'a> {
    /// Create a new message builder
    pub fn new(client: &'a OllamaClient) -> Self {
        Self {
            client,
            model: None,
            messages: Vec::new(),
            tools: None,
            format: None,
            options: OllamaOptions::default(),
            has_options: false,
            stream: None,
            think: None,
            keep_alive: None,
            logprobs: None,
            top_logprobs: None,
        }
    }

    /// Set the model to use
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Add a message to the conversation
    pub fn message(mut self, role: impl Into<String>, content: impl Into<String>) -> Self {
        let role_str = role.into();
        let role = match role_str.as_str() {
            "system" => OllamaRole::System,
            "user" => OllamaRole::User,
            "assistant" => OllamaRole::Assistant,
            "tool" => OllamaRole::Tool,
            _ => {
                tracing::warn!("Invalid role '{}', defaulting to 'user'", role_str);
                OllamaRole::User
            }
        };

        self.messages.push(OllamaMessage::new(role, content));
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

    /// Add a tool result (tool_call_id is not required by Ollama)
    pub fn tool_result(self, result: ToolResult) -> Self {
        self.tool_message(result.content())
    }

    /// Add a tool to the request
    pub fn tool(mut self, tool: Tool) -> Self {
        let tools = self.tools.get_or_insert_with(Vec::new);
        tools.push(OllamaToolFormat::to_provider_tool(&tool));
        self
    }

    /// Add multiple tools to the request
    pub fn tools(mut self, tools: Vec<Tool>) -> Self {
        for tool in tools {
            self = self.tool(tool);
        }
        self
    }

    /// Set JSON response format
    pub fn format_json(mut self) -> Self {
        self.format = Some(OllamaFormat::json());
        self
    }

    /// Set response format using a JSON schema
    pub fn format_schema(mut self, schema: serde_json::Value) -> Self {
        self.format = Some(OllamaFormat::schema(schema));
        self
    }

    /// Set temperature
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.options.temperature = Some(temperature);
        self.has_options = true;
        self
    }

    /// Set top-k sampling
    pub fn top_k(mut self, top_k: i32) -> Self {
        self.options.top_k = Some(top_k);
        self.has_options = true;
        self
    }

    /// Set top-p sampling
    pub fn top_p(mut self, top_p: f32) -> Self {
        self.options.top_p = Some(top_p);
        self.has_options = true;
        self
    }

    /// Set min-p sampling
    pub fn min_p(mut self, min_p: f32) -> Self {
        self.options.min_p = Some(min_p);
        self.has_options = true;
        self
    }

    /// Set stop sequences
    pub fn stop_sequences(mut self, stop: Vec<String>) -> Self {
        self.options.stop = Some(OllamaStop::Multiple(stop));
        self.has_options = true;
        self
    }

    /// Set context size
    pub fn num_ctx(mut self, num_ctx: u32) -> Self {
        self.options.num_ctx = Some(num_ctx);
        self.has_options = true;
        self
    }

    /// Set max tokens to generate
    pub fn num_predict(mut self, num_predict: u32) -> Self {
        self.options.num_predict = Some(num_predict);
        self.has_options = true;
        self
    }

    /// Set seed
    pub fn seed(mut self, seed: i64) -> Self {
        self.options.seed = Some(seed);
        self.has_options = true;
        self
    }

    /// Enable or disable streaming
    pub fn stream(mut self, stream: bool) -> Self {
        self.stream = Some(stream);
        self
    }

    /// Set thinking output
    pub fn think(mut self, think: OllamaThink) -> Self {
        self.think = Some(think);
        self
    }

    /// Set keep-alive duration
    pub fn keep_alive(mut self, keep_alive: OllamaKeepAlive) -> Self {
        self.keep_alive = Some(keep_alive);
        self
    }

    /// Enable logprobs
    pub fn logprobs(mut self, enabled: bool) -> Self {
        self.logprobs = Some(enabled);
        self
    }

    /// Set number of top logprobs
    pub fn top_logprobs(mut self, top_logprobs: u32) -> Self {
        self.top_logprobs = Some(top_logprobs);
        self
    }

    /// Send the request and get the response
    pub async fn send(self) -> Result<crate::ollama::types::OllamaChatResponse, LlmError> {
        let request = OllamaChatRequest {
            model: self
                .model
                .ok_or_else(|| LlmError::invalid_request("Model must be specified"))?,
            messages: self.messages,
            tools: self.tools,
            format: self.format,
            options: if self.has_options {
                Some(self.options)
            } else {
                None
            },
            stream: self.stream,
            think: self.think,
            keep_alive: self.keep_alive,
            logprobs: self.logprobs,
            top_logprobs: self.top_logprobs,
        };

        self.client.create_chat(request).await
    }
}
