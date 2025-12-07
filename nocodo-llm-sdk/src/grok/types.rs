use serde::{Deserialize, Serialize};

/// Grok chat completion request (OpenAI-compatible)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrokChatCompletionRequest {
    /// The model to use for generation
    pub model: String,
    /// Input messages
    pub messages: Vec<GrokMessage>,
    /// Maximum number of tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Temperature for randomness (0.0 to 2.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Top-p sampling parameter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Custom stop sequences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    /// Whether to stream the response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

/// A message in the Grok conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrokMessage {
    /// Role of the message sender
    pub role: GrokRole,
    /// Content of the message
    pub content: String,
}

/// Role of a Grok message
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GrokRole {
    /// System message
    System,
    /// User message
    User,
    /// Assistant message
    Assistant,
}

/// Grok chat completion response (OpenAI-compatible)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrokChatCompletionResponse {
    /// Unique identifier for the response
    pub id: String,
    /// Object type (always "chat.completion")
    pub object: String,
    /// Unix timestamp of creation
    pub created: u64,
    /// Model used for generation
    pub model: String,
    /// Completion choices
    pub choices: Vec<GrokChoice>,
    /// Token usage information
    pub usage: GrokUsage,
}

/// A completion choice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrokChoice {
    /// Index of the choice
    pub index: u32,
    /// The message content
    pub message: GrokMessage,
    /// Reason why generation stopped
    #[serde(rename = "finish_reason")]
    pub finish_reason: Option<String>,
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrokUsage {
    /// Number of prompt tokens
    #[serde(rename = "prompt_tokens")]
    pub prompt_tokens: u32,
    /// Number of completion tokens
    #[serde(rename = "completion_tokens")]
    pub completion_tokens: u32,
    /// Total number of tokens
    #[serde(rename = "total_tokens")]
    pub total_tokens: u32,
}

/// Grok API error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrokErrorResponse {
    /// Error details
    pub error: GrokError,
}

/// Grok API error details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrokError {
    /// Error message
    pub message: String,
    /// Error type
    #[serde(rename = "type")]
    pub error_type: String,
    /// Error code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

impl GrokMessage {
    /// Create a new text message
    pub fn new<S: Into<String>>(role: GrokRole, content: S) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }

    /// Create a system message
    pub fn system<S: Into<String>>(content: S) -> Self {
        Self::new(GrokRole::System, content)
    }

    /// Create a user message
    pub fn user<S: Into<String>>(content: S) -> Self {
        Self::new(GrokRole::User, content)
    }

    /// Create an assistant message
    pub fn assistant<S: Into<String>>(content: S) -> Self {
        Self::new(GrokRole::Assistant, content)
    }
}