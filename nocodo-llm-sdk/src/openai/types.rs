//! OpenAI API types for the Responses API.
//!
//! This module provides types for OpenAI's Responses API, which supports all GPT-5+ models
//! including GPT-5, GPT-5 mini, GPT-5 nano, GPT-5.1, and GPT-5.1 Codex.
//!
//! ## Responses API
//!
//! The Responses API is the primary interface for all OpenAI models.
//!
//! - **Endpoint:** `/v1/responses`
//! - **Models:** `gpt-5`, `gpt-5-mini`, `gpt-5-nano`, `gpt-5.1-codex`, `gpt-5.1`, etc.
//! - **Request Type:** [`OpenAIResponseRequest`]
//! - **Response Type:** [`OpenAIResponseResponse`]
//! - **Features:**
//!   - Extended reasoning capabilities for complex tasks
//!   - Background processing for long-running tasks
//!   - Conversation continuation via `previous_response_id`
//!   - Prompt caching for efficiency
//!   - Reasoning traces in output items
//!   - Tool calling support
//!
//! ### Example
//!
//! ```rust,no_run
//! use nocodo_llm_sdk::openai::OpenAIClient;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = OpenAIClient::new("your-api-key")?;
//! let response = client
//!     .response_builder()
//!     .model("gpt-5-mini")
//!     .input("Write a Python function to calculate fibonacci")
//!     .send()
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! See [`crate::openai::client::OpenAIClient`] for more details.

use schemars::schema::RootSchema;
use serde::{Deserialize, Serialize};

/// A message in the OpenAI conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIMessage {
    /// Role of the message sender
    pub role: OpenAIRole,
    /// Content of the message
    pub content: String,
    /// Tool calls made by the assistant
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<OpenAIResponseToolCall>>,
    /// Tool call ID for tool result messages
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
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

/// Response format type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OpenAIResponseFormatType {
    /// Plain text response
    Text,
    /// JSON object response
    JsonObject,
}

/// Response format configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OpenAIResponseFormat {
    #[serde(rename = "type")]
    pub format_type: OpenAIResponseFormatType,
}

impl OpenAIResponseFormat {
    pub fn text() -> Self {
        Self {
            format_type: OpenAIResponseFormatType::Text,
        }
    }
    pub fn json_object() -> Self {
        Self {
            format_type: OpenAIResponseFormatType::JsonObject,
        }
    }
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIUsage {
    /// Number of prompt tokens (Chat Completions API)
    #[serde(rename = "prompt_tokens")]
    pub prompt_tokens: Option<u32>,
    /// Number of completion tokens (Chat Completions API)
    #[serde(rename = "completion_tokens")]
    pub completion_tokens: Option<u32>,
    /// Number of input tokens (Responses API)
    #[serde(rename = "input_tokens")]
    pub input_tokens: Option<u32>,
    /// Number of output tokens (Responses API)
    #[serde(rename = "output_tokens")]
    pub output_tokens: Option<u32>,
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

/// OpenAI Responses API request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIResponseRequest {
    /// The model to use for generation
    pub model: String,
    /// Input text for the response
    pub input: String,
    /// Whether to stream the response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// ID of previous response to continue the conversation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_response_id: Option<String>,
    /// Whether to run in background for long tasks
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<bool>,
    /// How long to retain the prompt in cache
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_cache_retention: Option<String>,
    /// Available tools for the model to use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<OpenAIResponseTool>>,
    /// Tool choice strategy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<serde_json::Value>,
    /// Whether to allow parallel tool calls
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
}

/// OpenAI Responses API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIResponseResponse {
    /// Unique identifier for the response
    pub id: String,
    /// Object type (always "response")
    pub object: String,
    /// Unix timestamp of creation
    pub created_at: u64,
    /// Status of the response
    pub status: String,
    /// Model used for generation
    pub model: String,
    /// Output items from the response
    pub output: Vec<OpenAIOutputItem>,
    /// Token usage information
    pub usage: OpenAIUsage,
    /// Background processing flag
    #[serde(default)]
    pub background: bool,
}

/// Output item in Responses API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIOutputItem {
    /// Unique identifier for the output item
    pub id: String,
    /// Type of the output item
    #[serde(rename = "type")]
    pub item_type: String,
    /// Content of the output item (for message types)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Vec<OpenAIContentBlock>>,
    /// Role of the output item (for message types)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// Status of the output item (for message types)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// Summary for reasoning types
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<Vec<serde_json::Value>>,
}

/// Content block in output item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIContentBlock {
    /// Type of content
    #[serde(rename = "type")]
    pub content_type: String,
    /// Annotations for the content
    pub annotations: Vec<serde_json::Value>,
    /// Log probabilities
    #[serde(default)]
    pub logprobs: Vec<serde_json::Value>,
    /// Text content
    pub text: String,
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

/// OpenAI tool definition for Chat Completions API
/// Note: This type is kept for backward compatibility with other providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAITool {
    /// Type of tool (always "function")
    #[serde(rename = "type")]
    pub r#type: String,
    /// Function definition
    pub function: OpenAIFunction,
}

/// OpenAI tool definition for Responses API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIResponseTool {
    /// Type of tool (always "function")
    #[serde(rename = "type")]
    pub r#type: String,
    /// Function name
    pub name: String,
    /// Function description
    pub description: String,
    /// Function parameters schema
    pub parameters: schemars::schema::RootSchema,
}

/// OpenAI function definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIFunction {
    /// Function name
    pub name: String,
    /// Function description
    pub description: String,
    /// Function parameters schema
    pub parameters: RootSchema,
}

/// Tool call in OpenAI response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIResponseToolCall {
    /// Unique identifier for the tool call
    pub id: String,
    /// Type of tool call (always "function")
    #[serde(rename = "type")]
    pub r#type: String,
    /// Function call details
    pub function: OpenAIFunctionCall,
}

/// Function call details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIFunctionCall {
    /// Function name
    pub name: String,
    /// Function arguments as JSON string
    pub arguments: String,
}

impl OpenAIMessage {
    /// Create a new text message
    pub fn new<S: Into<String>>(role: OpenAIRole, content: S) -> Self {
        Self {
            role,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
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

    /// Create an assistant message with tool calls
    pub fn assistant_with_tools<S: Into<String>>(
        content: S,
        tool_calls: Vec<OpenAIResponseToolCall>,
    ) -> Self {
        Self {
            role: OpenAIRole::Assistant,
            content: content.into(),
            tool_calls: Some(tool_calls),
            tool_call_id: None,
        }
    }

    /// Create a tool result message
    pub fn tool_result<S: Into<String>>(tool_call_id: S, content: S) -> Self {
        Self {
            role: OpenAIRole::Tool,
            content: content.into(),
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
        }
    }
}
