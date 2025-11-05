use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Claude completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeCompletionRequest {
    pub model: String,
    pub messages: Vec<ClaudeMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// System prompt (separate field, NOT in messages array)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ClaudeToolDefinition>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ClaudeToolChoice>,
}

/// Claude message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeMessage {
    pub role: String, // "user" or "assistant"
    pub content: Vec<ClaudeContentBlock>,
}

/// Claude content block (all content must be wrapped in blocks)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClaudeContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
}

/// Claude tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeToolDefinition {
    pub name: String,
    pub description: String,
    /// Note: This is "input_schema" for Claude, NOT "parameters" like OpenAI
    pub input_schema: Value,
}

/// Claude tool choice
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClaudeToolChoice {
    Auto { r#type: String },               // {"type": "auto"}
    Any { r#type: String },                // {"type": "any"}
    Tool { r#type: String, name: String }, // {"type": "tool", "name": "tool_name"}
}

/// Claude completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeCompletionResponse {
    pub id: String,
    pub r#type: String,
    pub role: String,
    pub content: Vec<ClaudeContentBlock>,
    pub model: String,
    pub stop_reason: Option<String>,
    pub stop_sequence: Option<String>,
    pub usage: ClaudeUsage,
}

/// Claude token usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}
