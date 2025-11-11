use crate::llm_client::adapters::trait_adapter::ProviderRequest;
use anyhow::Result;
use serde::{Deserialize, Serialize, Serializer};
use serde_json::Value;

/// Custom serializer for temperature to avoid floating point precision issues
fn serialize_rounded_f32<S>(value: &Option<f32>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(v) => {
            // Round to 1 decimal place to avoid precision issues like 0.30000001192092896
            let rounded = (v * 10.0).round() / 10.0;
            serializer.serialize_f32(rounded)
        }
        None => serializer.serialize_none(),
    }
}

/// GLM Chat Completions API Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlmChatCompletionsRequest {
    pub model: String,
    pub messages: Vec<Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Value>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<String>,

    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_rounded_f32"
    )]
    pub temperature: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<GlmThinkingConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<GlmResponseFormat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlmThinkingConfig {
    pub r#type: String, // "enabled" or "disabled"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlmResponseFormat {
    pub r#type: String, // "text" or "json_object"
}

impl ProviderRequest for GlmChatCompletionsRequest {
    fn to_json(&self) -> Result<Value> {
        Ok(serde_json::to_value(self)?)
    }

    fn custom_headers(&self) -> Vec<(String, String)> {
        vec![("Accept-Language".to_string(), "en-US,en".to_string())]
    }
}

/// GLM Chat Completions API Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlmChatCompletionsResponse {
    pub id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,

    pub created: i64,
    pub model: String,
    pub choices: Vec<GlmChoice>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<GlmUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlmChoice {
    pub index: i32,
    pub message: GlmMessage,
    pub finish_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlmMessage {
    pub role: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<GlmToolCall>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlmToolCall {
    pub id: String,
    pub r#type: String,
    pub function: GlmFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlmFunction {
    pub name: String,
    pub arguments: Value, // Can be JSON object or string
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlmUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens_details: Option<GlmPromptTokensDetails>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlmPromptTokensDetails {
    pub cached_tokens: u32,
}
