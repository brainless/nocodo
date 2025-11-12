use serde::{Deserialize, Serialize};

/// OpenAI Responses API structures
/// Content item for Responses API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentItem {
    #[serde(rename = "input_text")]
    InputText { text: String },
    #[serde(rename = "output_text")]
    OutputText {
        text: String,
        #[serde(default)]
        annotations: Vec<serde_json::Value>,
        #[serde(default)]
        logprobs: Vec<serde_json::Value>,
    },
}

/// Response item for Responses API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ResponseItem {
    #[serde(rename = "message")]
    Message {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        role: String,
        content: Vec<ContentItem>,
    },
    #[serde(rename = "reasoning")]
    Reasoning {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        summary: Vec<serde_json::Value>,
    },
    #[serde(rename = "function_call")]
    FunctionCall {
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        name: String,
        arguments: String,
        #[serde(rename = "call_id")]
        call_id: String,
    },
}

/// Usage statistics for Responses API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponsesUsage {
    pub input_tokens: u32,
    pub input_tokens_details: Option<serde_json::Value>,
    pub output_tokens: u32,
    pub output_tokens_details: Option<serde_json::Value>,
    pub total_tokens: u32,
}

/// Tool definition for Responses API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponsesToolDefinition {
    #[serde(rename = "type")]
    pub r#type: String,
    pub name: String,
    pub description: String,
    pub strict: bool,
    pub parameters: serde_json::Value,
}

/// Responses API request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponsesApiRequest {
    pub model: String,
    pub instructions: String,
    pub input: Vec<serde_json::Value>, // Raw message objects like [{"role": "user", "content": "..."}]
    pub tools: Option<Vec<ResponsesToolDefinition>>,
    pub tool_choice: String, // "auto", "required", or "none"
    pub stream: bool,
}

/// Responses API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponsesApiResponse {
    pub id: String,
    #[serde(rename = "type")]
    pub r#type: Option<String>,
    pub model: String,
    pub output: Vec<ResponseItem>,
    pub usage: Option<ResponsesUsage>,
}

/// Streaming event for Responses API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[allow(dead_code)]
#[allow(clippy::enum_variant_names)]
pub enum ResponsesStreamEvent {
    #[serde(rename = "response.created")]
    ResponseCreated { response: serde_json::Value },
    #[serde(rename = "response.output_item.done")]
    ResponseOutputItemDone { item: ResponseItem },
    #[serde(rename = "response.completed")]
    ResponseCompleted { response: serde_json::Value },
}
