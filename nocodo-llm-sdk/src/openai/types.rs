use serde::{Deserialize, Serialize};

/// OpenAI chat completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIChatCompletionRequest {
    /// The model to use for generation
    pub model: String,
    /// Input messages
    pub messages: Vec<OpenAIMessage>,
    /// Maximum number of tokens to generate (legacy, use max_completion_tokens)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Maximum number of completion tokens to generate (recommended)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u32>,
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
    /// Reasoning effort for GPT-5 models ("minimal", "low", "medium", "high")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
}

/// A message in the OpenAI conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIMessage {
    /// Role of the message sender
    pub role: OpenAIRole,
    /// Content of the message
    pub content: String,
}

/// Role of an OpenAI message
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OpenAIRole {
    /// System message
    System,
    /// User message
    User,
    /// Assistant message
    Assistant,
    /// Tool message
    Tool,
}

/// OpenAI chat completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIChatCompletionResponse {
    /// Unique identifier for the response
    pub id: String,
    /// Object type (always "chat.completion")
    pub object: String,
    /// Unix timestamp of creation
    pub created: u64,
    /// Model used for generation
    pub model: String,
    /// Completion choices
    pub choices: Vec<OpenAIChoice>,
    /// Token usage information
    pub usage: OpenAIUsage,
    /// System fingerprint (for reproducibility)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
}

/// A completion choice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIChoice {
    /// Index of the choice
    pub index: u32,
    /// The message content
    pub message: OpenAIMessage,
    /// Reason why generation stopped
    #[serde(rename = "finish_reason")]
    pub finish_reason: Option<String>,
    /// Log probability information (if requested)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<OpenAILogProbs>,
}

/// Log probability information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAILogProbs {
    /// Log probabilities for tokens
    pub content: Vec<OpenAILogProb>,
}

/// Log probability for a token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAILogProb {
    /// The token
    pub token: String,
    /// Log probability of the token
    pub logprob: f32,
    /// Bytes representation
    pub bytes: Option<Vec<u8>>,
    /// Top log probabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_logprobs: Option<Vec<OpenAITopLogProb>>,
}

/// Top log probability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAITopLogProb {
    /// The token
    pub token: String,
    /// Log probability
    pub logprob: f32,
    /// Bytes representation
    pub bytes: Option<Vec<u8>>,
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIUsage {
    /// Number of prompt tokens
    #[serde(rename = "prompt_tokens")]
    pub prompt_tokens: u32,
    /// Number of completion tokens
    #[serde(rename = "completion_tokens")]
    pub completion_tokens: u32,
    /// Total number of tokens
    #[serde(rename = "total_tokens")]
    pub total_tokens: u32,
    /// Completion token details (for GPT-5)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion_tokens_details: Option<OpenAICompletionTokensDetails>,
}

/// Completion token details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAICompletionTokensDetails {
    /// Number of reasoning tokens (for GPT-5)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_tokens: Option<u32>,
}

/// OpenAI API error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIErrorResponse {
    /// Error details
    pub error: OpenAIError,
}

/// OpenAI API error details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIError {
    /// Error message
    pub message: String,
    /// Error type
    #[serde(rename = "type")]
    pub error_type: String,
    /// Error code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// Parameter that caused the error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub param: Option<String>,
}

impl OpenAIMessage {
    /// Create a new text message
    pub fn new<S: Into<String>>(role: OpenAIRole, content: S) -> Self {
        Self {
            role,
            content: content.into(),
        }
    }

    /// Create a system message
    pub fn system<S: Into<String>>(content: S) -> Self {
        Self::new(OpenAIRole::System, content)
    }

    /// Create a user message
    pub fn user<S: Into<String>>(content: S) -> Self {
        Self::new(OpenAIRole::User, content)
    }

    /// Create an assistant message
    pub fn assistant<S: Into<String>>(content: S) -> Self {
        Self::new(OpenAIRole::Assistant, content)
    }

    /// Create a tool message
    pub fn tool<S: Into<String>>(content: S) -> Self {
        Self::new(OpenAIRole::Tool, content)
    }
}