use serde::{Deserialize, Serialize};

use crate::openai::types::OpenAITool;

/// Ollama chat request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaChatRequest {
    /// Model name
    pub model: String,
    /// Chat history
    pub messages: Vec<OllamaMessage>,
    /// Optional tool definitions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<OpenAITool>>,
    /// Response format ("json" or JSON schema)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<OllamaFormat>,
    /// Model options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<OllamaOptions>,
    /// Stream responses (default true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// Thinking output control
    #[serde(skip_serializing_if = "Option::is_none")]
    pub think: Option<OllamaThink>,
    /// Model keep-alive duration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_alive: Option<OllamaKeepAlive>,
    /// Enable logprobs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<bool>,
    /// Number of top logprobs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_logprobs: Option<u32>,
}

/// Ollama response format
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OllamaFormat {
    /// Use JSON mode
    Json(String),
    /// Use a JSON schema
    Schema(serde_json::Value),
}

impl OllamaFormat {
    pub fn json() -> Self {
        Self::Json("json".to_string())
    }

    pub fn schema(schema: serde_json::Value) -> Self {
        Self::Schema(schema)
    }
}

/// Ollama model options
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<OllamaStop>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_ctx: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_predict: Option<u32>,
}

/// Stop sequences for Ollama
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OllamaStop {
    Single(String),
    Multiple(Vec<String>),
}

/// Thinking control
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OllamaThink {
    Bool(bool),
    Level(String),
}

/// Keep-alive duration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OllamaKeepAlive {
    Duration(String),
    Seconds(u64),
}

/// Ollama chat response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaChatResponse {
    pub model: String,
    pub created_at: String,
    pub message: OllamaMessage,
    pub done: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_duration: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_duration: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_eval_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_eval_duration: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_duration: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<Vec<OllamaLogprob>>,
}

/// Chat message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaMessage {
    pub role: OllamaRole,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<OllamaToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<String>>,
}

impl OllamaMessage {
    pub fn new(role: OllamaRole, content: impl Into<String>) -> Self {
        Self {
            role,
            content: content.into(),
            thinking: None,
            tool_calls: None,
            images: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OllamaRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaToolCall {
    pub function: OllamaFunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaFunctionCall {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaLogprob {
    pub token: String,
    pub logprob: f32,
    pub bytes: Vec<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_logprobs: Option<Vec<OllamaTokenLogprob>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaTokenLogprob {
    pub token: String,
    pub logprob: f32,
    pub bytes: Vec<i32>,
}
