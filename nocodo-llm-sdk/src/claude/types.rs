use serde::{Deserialize, Serialize};

/// Claude message request for the Messages API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeMessageRequest {
    /// The model to use for generation
    pub model: String,
    /// Maximum number of tokens to generate
    pub max_tokens: u32,
    /// Input messages
    pub messages: Vec<ClaudeMessage>,
    /// System prompt (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// Temperature for randomness (0.0 to 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Top-p sampling parameter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Custom stop sequences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
}

/// A message in the Claude conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeMessage {
    /// Role of the message sender
    pub role: ClaudeRole,
    /// Content of the message
    pub content: Vec<ClaudeContentBlock>,
}

/// Role of a Claude message
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClaudeRole {
    /// User message
    User,
    /// Assistant message
    Assistant,
}

/// Content block in a Claude message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ClaudeContentBlock {
    /// Text content
    Text { text: String },
}

/// Claude message response from the Messages API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeMessageResponse {
    /// Unique identifier for the response
    pub id: String,
    /// Type of response (always "message")
    #[serde(rename = "type")]
    pub response_type: String,
    /// Role of the response (always "assistant")
    pub role: ClaudeRole,
    /// Model used for generation
    pub model: String,
    /// Content blocks in the response
    pub content: Vec<ClaudeContentBlock>,
    /// Reason why generation stopped
    pub stop_reason: Option<String>,
    /// Stop sequence that was encountered (if any)
    pub stop_sequence: Option<String>,
    /// Token usage information
    pub usage: ClaudeUsage,
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeUsage {
    /// Number of input tokens
    pub input_tokens: u32,
    /// Number of output tokens
    pub output_tokens: u32,
}

/// Claude API error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeErrorResponse {
    /// Type of response (always "error")
    #[serde(rename = "type")]
    pub response_type: String,
    /// Error details
    pub error: ClaudeError,
}

/// Claude API error details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeError {
    /// Error type
    #[serde(rename = "type")]
    pub error_type: String,
    /// Human-readable error message
    pub message: String,
}

impl ClaudeMessage {
    /// Create a new text message
    pub fn text<S: Into<String>>(role: ClaudeRole, text: S) -> Self {
        Self {
            role,
            content: vec![ClaudeContentBlock::Text { text: text.into() }],
        }
    }

    /// Create a user message with text content
    pub fn user<S: Into<String>>(text: S) -> Self {
        Self::text(ClaudeRole::User, text)
    }

    /// Create an assistant message with text content
    pub fn assistant<S: Into<String>>(text: S) -> Self {
        Self::text(ClaudeRole::Assistant, text)
    }
}
