use crate::{
    error::LlmError,
    openai::{
        client::OpenAIClient,
        types::{OpenAIChatCompletionRequest, OpenAIMessage, OpenAIRole},
    },
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

    /// Send the request and get the response
    pub async fn send(self) -> Result<crate::openai::types::OpenAIChatCompletionResponse, LlmError> {
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
        };

        self.client.create_chat_completion(request).await
    }
}

impl OpenAIClient {
    /// Start building a chat completion request
    pub fn message_builder(&self) -> OpenAIMessageBuilder<'_> {
        OpenAIMessageBuilder::new(self)
    }
}