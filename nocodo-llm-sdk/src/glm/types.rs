use serde::{Deserialize, Serialize};

/// GLM chat completion request (Cerebras/OpenAI-compatible)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlmChatCompletionRequest {
    /// The model to use for generation
    pub model: String,
    /// Input messages
    pub messages: Vec<GlmMessage>,
    /// Maximum number of tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u32>,
    /// Temperature for randomness (0.0 to 1.5)
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
    /// Seed for deterministic sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i32>,
    /// Available tools for the model to use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<GlmTool>>,
    /// Tool choice strategy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<serde_json::Value>,
}

/// A message in the GLM conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlmMessage {
    /// Role of the message sender
    pub role: GlmRole,
    /// Content of the message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Reasoning content (for reasoning models like GLM)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
    /// Tool calls made by the assistant
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<GlmToolCall>>,
    /// Tool call ID for tool result messages
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

/// Role of a GLM message
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GlmRole {
    /// System message
    System,
    /// User message
    User,
    /// Assistant message
    Assistant,
}

/// GLM chat completion response (Cerebras/OpenAI-compatible)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlmChatCompletionResponse {
    /// Unique identifier for the response
    pub id: String,
    /// Object type (always "chat.completion")
    pub object: String,
    /// Unix timestamp of creation
    pub created: u64,
    /// Model used for generation
    pub model: String,
    /// Completion choices
    pub choices: Vec<GlmChoice>,
    /// Token usage information
    pub usage: GlmUsage,
}

/// A completion choice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlmChoice {
    /// Index of the choice
    pub index: u32,
    /// The message content
    pub message: GlmMessage,
    /// Reason why generation stopped
    #[serde(rename = "finish_reason")]
    pub finish_reason: Option<String>,
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlmUsage {
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

/// GLM API error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlmErrorResponse {
    /// Error details
    pub error: GlmError,
}

/// GLM API error details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlmError {
    /// Error message
    pub message: String,
    /// Error type
    #[serde(rename = "type")]
    pub error_type: String,
}

/// GLM tool definition (OpenAI-compatible)
pub type GlmTool = crate::openai::types::OpenAITool;

/// GLM function definition (OpenAI-compatible)
pub type GlmFunction = crate::openai::types::OpenAIFunction;

/// Tool call in GLM response (OpenAI-compatible)
pub type GlmToolCall = crate::openai::types::OpenAIToolCall;

/// Function call details (OpenAI-compatible)
pub type GlmFunctionCall = crate::openai::types::OpenAIFunctionCall;

impl GlmMessage {
    /// Create a new text message
    pub fn new<S: Into<String>>(role: GlmRole, content: S) -> Self {
        Self {
            role,
            content: Some(content.into()),
            reasoning: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Create a system message
    pub fn system<S: Into<String>>(content: S) -> Self {
        Self::new(GlmRole::System, content)
    }

    /// Create a user message
    pub fn user<S: Into<String>>(content: S) -> Self {
        Self::new(GlmRole::User, content)
    }

    /// Create an assistant message
    pub fn assistant<S: Into<String>>(content: S) -> Self {
        Self::new(GlmRole::Assistant, content)
    }

    /// Create an assistant message with tool calls
    pub fn assistant_with_tools<S: Into<String>>(content: S, tool_calls: Vec<GlmToolCall>) -> Self {
        Self {
            role: GlmRole::Assistant,
            content: Some(content.into()),
            reasoning: None,
            tool_calls: Some(tool_calls),
            tool_call_id: None,
        }
    }

    /// Create a tool result message
    pub fn tool_result<S: Into<String>>(tool_call_id: S, content: S) -> Self {
        Self {
            role: GlmRole::Assistant, // GLM uses Assistant role for tool results
            content: Some(content.into()),
            reasoning: None,
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
        }
    }

    /// Get the text content, combining content and reasoning if both exist
    pub fn get_text(&self) -> String {
        match (&self.content, &self.reasoning) {
            (Some(c), Some(r)) => format!("{}\n\nReasoning: {}", c, r),
            (Some(c), None) => c.clone(),
            (None, Some(r)) => r.clone(),
            (None, None) => String::new(),
        }
    }
}
