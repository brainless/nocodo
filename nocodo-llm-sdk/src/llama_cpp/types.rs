use serde::{Deserialize, Serialize};

use crate::openai::types::OpenAITool;

/// Llama.cpp chat completion request (OpenAI-compatible)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlamaCppChatCompletionRequest {
    pub model: String,
    pub messages: Vec<LlamaCppMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<OpenAITool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
}

/// Llama.cpp chat completion response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlamaCppChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<LlamaCppChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<LlamaCppUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlamaCppChoice {
    pub index: u32,
    pub message: LlamaCppMessage,
    #[serde(rename = "finish_reason")]
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlamaCppUsage {
    #[serde(rename = "prompt_tokens")]
    pub prompt_tokens: u32,
    #[serde(rename = "completion_tokens")]
    pub completion_tokens: u32,
    #[serde(rename = "total_tokens")]
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlamaCppMessage {
    pub role: LlamaCppRole,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<LlamaCppToolCall>>,
}

impl LlamaCppMessage {
    pub fn new(role: LlamaCppRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: Some(content.into()),
            tool_calls: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LlamaCppRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LlamaCppToolCall {
    /// Simplified tool call format (name + arguments string)
    Simple { name: String, arguments: String },
    /// OpenAI-style tool call format
    OpenAI(crate::openai::types::OpenAIResponseToolCall),
}
