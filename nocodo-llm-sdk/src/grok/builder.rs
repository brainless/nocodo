use crate::{
    error::LlmError,
    grok::{
        client::GrokClient,
        types::{GrokChatCompletionRequest, GrokMessage, GrokRole},
    },
};

/// Builder for creating Grok chat completion requests
pub struct GrokMessageBuilder<'a> {
    client: &'a GrokClient,
    model: Option<String>,
    max_tokens: Option<u32>,
    messages: Vec<GrokMessage>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    stop: Option<Vec<String>>,
    stream: Option<bool>,
}

impl<'a> GrokMessageBuilder<'a> {
    /// Create a new message builder
    pub fn new(client: &'a GrokClient) -> Self {
        Self {
            client,
            model: None,
            max_tokens: None,
            messages: Vec::new(),
            temperature: None,
            top_p: None,
            stop: None,
            stream: None,
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
        };

        self.client.create_chat_completion(request).await
    }
}

impl GrokClient {
    /// Start building a chat completion request
    pub fn message_builder(&self) -> GrokMessageBuilder<'_> {
        GrokMessageBuilder::new(self)
    }
}