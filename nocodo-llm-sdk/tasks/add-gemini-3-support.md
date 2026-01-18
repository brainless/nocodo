# Task: Add Google Gemini 3 Pro and Gemini 3 Flash Support

**Status**: Not Started
**Priority**: High
**Created**: 2026-01-18
**Estimated Effort**: 12-16 hours
**Related**: tasks/nocodo-llm-sdk-creation.md (Multi-Provider Architecture)

---

## Overview

Add Google Gemini 3 Pro and Gemini 3 Flash models to nocodo-llm-sdk. These are Google's latest reasoning-capable models with thinking level controls, tool calling, and multimodal support.

**Models to Support:**
- **Gemini 3 Pro** (`gemini-3-pro-preview`) - Most intelligent model for complex reasoning
- **Gemini 3 Flash** (`gemini-3-flash-preview`) - Pro-level intelligence at Flash speed

**Reference Documentation**:
- `external-docs/gemini_3_developer_guide.md` - Complete feature guide
- `external-docs/gemini_api_reference.md` - API reference

---

## Current State

### Existing Provider Integrations

The SDK currently supports:
- **Claude** (Anthropic) - Full Messages API with tool calling
- **OpenAI** (GPT models) - Chat Completions + Responses API
- **Grok** (xAI + Zen) - OpenAI-compatible API
- **GLM** (Cerebras + Zen) - OpenAI-compatible API
- **Voyage AI** - Text embeddings

### Current File Structure
```
nocodo-llm-sdk/src/
├── claude/
│   ├── mod.rs
│   ├── client.rs
│   ├── types.rs
│   ├── builder.rs
│   └── tools.rs
├── openai/
│   ├── mod.rs
│   ├── client.rs
│   ├── types.rs
│   ├── builder.rs
│   └── tools.rs
├── grok/
│   ├── xai/
│   └── zen/
├── glm/
│   ├── cerebras/
│   └── zen/
├── voyage/
├── models.rs          # Model constants (NO Gemini yet)
├── model_metadata.rs  # Model capabilities registry
├── providers.rs       # Provider name constants
├── types.rs
└── client.rs
```

---

## Target State

### New Gemini Module Structure
```
nocodo-llm-sdk/src/
├── gemini/                      # NEW: Gemini module
│   ├── mod.rs                   # Public exports
│   ├── types.rs                 # Gemini request/response types
│   ├── client.rs                # GeminiClient implementation
│   ├── builder.rs               # MessageBuilder pattern
│   └── tools.rs                 # GeminiToolFormat adapter
├── models.rs                    # ADD: gemini module with constants
├── model_metadata.rs            # ADD: Gemini 3 model metadata
├── providers.rs                 # ADD: GOOGLE provider constant
└── lib.rs                       # ADD: Gemini exports
```

---

## Assessment Summary

### ✅ No Blockers Found

After comprehensive analysis of the codebase and Gemini documentation:

**Architecture Compatibility**:
- Gemini API structure aligns well with existing patterns
- Similar to Claude's request/response model
- Tool calling support matches SDK conventions
- Streaming available (can be implemented later)

**Key Differences from Claude**:

| Aspect | Claude | Gemini | Implementation Impact |
|--------|--------|--------|---------------------|
| **Auth Header** | `x-api-key` | `x-goog-api-key` | Simple header change |
| **Endpoint** | `/v1/messages` | `/v1beta/models/{MODEL}:generateContent` | Dynamic URL construction |
| **Roles** | user/assistant | user/model | Enum mapping |
| **Content Structure** | Blocks | Parts within Contents | Similar structure |
| **Unique Feature** | Cache control | `thinking_level` parameter | Optional field |
| **Thought Preservation** | N/A | `thoughtSignature` for tool calls | New field, critical for function calling |

---

## Gemini 3 Model Specifications

### Gemini 3 Pro (`gemini-3-pro-preview`)

**Capabilities**:
- Context: 1M input / 64k output tokens
- Knowledge cutoff: January 2025
- Thinking levels: `low`, `high` (default)
- Supports: Tool calling, vision, structured outputs, reasoning
- Best for: Complex reasoning, autonomous coding, agentic workflows

**Pricing**:
- Input: $2 per 1M tokens (<200k), $4 per 1M (>200k)
- Output: $12 per 1M tokens (<200k), $18 per 1M (>200k)

**Special Features**:
- Dynamic thinking by default (can be controlled via `thinking_level`)
- Temperature recommendation: 1.0 (default, do NOT lower)
- Thought signatures for multi-step function calling

### Gemini 3 Flash (`gemini-3-flash-preview`)

**Capabilities**:
- Context: 1M input / 64k output tokens
- Knowledge cutoff: January 2025
- Thinking levels: `minimal`, `low`, `medium`, `high` (default)
- Supports: Tool calling, vision, structured outputs, reasoning
- Best for: Fast responses with Pro-level intelligence

**Pricing**:
- Input: $0.50 per 1M tokens
- Output: $3 per 1M tokens

**Special Features**:
- Additional `minimal` and `medium` thinking levels
- Faster time-to-first-token than Pro
- Still requires thought signature circulation even at `minimal` level

---

## Implementation Plan

### Phase 1: Module Foundation (3-4 hours)

#### 1.1 Add Model Constants

**Update `nocodo-llm-sdk/src/models.rs`**:

```rust
/// Google Gemini model constants
pub mod gemini {
    /// Gemini 3 Pro - Most intelligent model for complex reasoning
    /// Released: Preview, Context: 1M/64k, Thinking: low/high
    pub const GEMINI_3_PRO_ID: &str = "gemini-3-pro-preview";
    pub const GEMINI_3_PRO_NAME: &str = "Gemini 3 Pro";

    /// Gemini 3 Flash - Pro-level intelligence at Flash speed
    /// Released: Preview, Context: 1M/64k, Thinking: minimal/low/medium/high
    pub const GEMINI_3_FLASH_ID: &str = "gemini-3-flash-preview";
    pub const GEMINI_3_FLASH_NAME: &str = "Gemini 3 Flash";

    // Backwards compatibility
    pub const GEMINI_3_PRO: &str = GEMINI_3_PRO_ID;
    pub const GEMINI_3_FLASH: &str = GEMINI_3_FLASH_ID;
}
```

#### 1.2 Add Provider Constant

**Update `nocodo-llm-sdk/src/providers.rs`**:

```rust
/// Google (Gemini models)
pub const GOOGLE: &str = "google";
```

#### 1.3 Register Model Metadata

**Update `nocodo-llm-sdk/src/model_metadata.rs`**:

```rust
use crate::models::gemini::*;
use crate::providers::GOOGLE;

// Add to the models() function:

// Gemini 3 Pro
ModelMetadata {
    provider: GOOGLE,
    model_id: GEMINI_3_PRO_ID,
    name: GEMINI_3_PRO_NAME,
    context_length: 1_000_000,
    max_output_tokens: Some(64_000),
    supports_streaming: true,
    supports_tool_calling: true,
    supports_vision: true,
    supports_reasoning: true,
    default_temperature: Some(1.0),
    input_cost_per_million_tokens: Some(2.0), // <200k tokens
    output_cost_per_million_tokens: Some(12.0),
    knowledge_cutoff: Some("2025-01"),
},

// Gemini 3 Flash
ModelMetadata {
    provider: GOOGLE,
    model_id: GEMINI_3_FLASH_ID,
    name: GEMINI_3_FLASH_NAME,
    context_length: 1_000_000,
    max_output_tokens: Some(64_000),
    supports_streaming: true,
    supports_tool_calling: true,
    supports_vision: true,
    supports_reasoning: true,
    default_temperature: Some(1.0),
    input_cost_per_million_tokens: Some(0.5),
    output_cost_per_million_tokens: Some(3.0),
    knowledge_cutoff: Some("2025-01"),
},
```

#### 1.4 Create Module Structure

```bash
cd nocodo-llm-sdk/src
mkdir gemini
touch gemini/mod.rs
touch gemini/types.rs
touch gemini/client.rs
touch gemini/builder.rs
touch gemini/tools.rs
```

---

### Phase 2: Implement Types (3-4 hours)

#### 2.1 Gemini Types Module

**Create `nocodo-llm-sdk/src/gemini/types.rs`**:

Key types to implement:

```rust
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Gemini API role enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum GeminiRole {
    User,
    Model,  // Equivalent to "assistant" in other APIs
}

/// A single part within content (text, function call, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiPart {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub inline_data: Option<GeminiBlob>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_call: Option<GeminiFunctionCall>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_response: Option<GeminiFunctionResponse>,

    /// Thought signature for maintaining reasoning context
    /// CRITICAL: Must be preserved and returned in subsequent requests
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thought_signature: Option<String>,
}

/// Content object representing a turn in conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiContent {
    pub role: GeminiRole,
    pub parts: Vec<GeminiPart>,
}

/// Thinking configuration for Gemini 3 models
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThinkingConfig {
    /// Thinking level: "minimal", "low", "medium", "high"
    /// Note: "minimal" and "medium" only supported by Gemini 3 Flash
    pub thinking_level: String,
}

/// Generation configuration parameters
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_config: Option<ThinkingConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_mime_type: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_json_schema: Option<Value>,
}

/// Main request structure for generateContent
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiGenerateContentRequest {
    /// List of Content objects (conversation history)
    pub contents: Vec<GeminiContent>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_instruction: Option<GeminiContent>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<GeminiTool>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_config: Option<GeminiToolConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,
}

/// Response candidate
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiCandidate {
    pub content: GeminiContent,
    pub finish_reason: String,
    pub index: u32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_ratings: Option<Vec<GeminiSafetyRating>>,
}

/// Usage metadata
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiUsageMetadata {
    pub prompt_token_count: u32,
    pub candidates_token_count: u32,
    pub total_token_count: u32,
}

/// Main response structure
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiGenerateContentResponse {
    pub candidates: Vec<GeminiCandidate>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_metadata: Option<GeminiUsageMetadata>,

    pub model_version: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_id: Option<String>,
}

/// Error response structure
#[derive(Debug, Clone, Deserialize)]
pub struct GeminiError {
    pub code: u16,
    pub message: String,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GeminiErrorResponse {
    pub error: GeminiError,
}

// Tool-related types (similar to Claude)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiFunctionDeclaration {
    pub name: String,
    pub description: String,
    pub parameters: Value,  // JSON Schema
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiTool {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub function_declarations: Option<Vec<GeminiFunctionDeclaration>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiFunctionCall {
    pub name: String,
    pub args: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiFunctionResponse {
    pub name: String,
    pub response: Value,
}

// Additional helper types...
```

**Implementation Notes**:
- All types must match Gemini API specification exactly
- `thought_signature` field is CRITICAL for function calling
- Role enum maps `Model` to generic `Assistant` concept
- Support for both text and multimodal content via `GeminiPart` union type

---

### Phase 3: Implement Client (4-5 hours)

#### 3.1 Gemini Client

**Create `nocodo-llm-sdk/src/gemini/client.rs`**:

```rust
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use crate::{
    error::LlmError,
    gemini::types::*,
};

/// Google Gemini API client
pub struct GeminiClient {
    api_key: String,
    base_url: String,
    http_client: reqwest::Client,
}

impl GeminiClient {
    /// Create a new Gemini client
    pub fn new(api_key: impl Into<String>) -> Result<Self, LlmError> {
        let api_key = api_key.into();
        if api_key.is_empty() {
            return Err(LlmError::authentication("API key cannot be empty"));
        }

        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .map_err(|e| LlmError::Network { source: e })?;

        Ok(Self {
            api_key,
            base_url: "https://generativelanguage.googleapis.com".to_string(),
            http_client,
        })
    }

    /// Set a custom base URL
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Create a content generation request
    ///
    /// NOTE: Model ID is part of the endpoint URL, not request body
    pub async fn generate_content(
        &self,
        model: impl Into<String>,
        request: GeminiGenerateContentRequest,
    ) -> Result<GeminiGenerateContentResponse, LlmError> {
        let model = model.into();

        // Construct endpoint with model ID
        let url = format!(
            "{}/v1beta/models/{}:generateContent",
            self.base_url, model
        );

        let mut headers = HeaderMap::new();

        // Gemini uses x-goog-api-key header (NOT Authorization)
        headers.insert(
            "x-goog-api-key",
            HeaderValue::from_str(&self.api_key)
                .map_err(|e| LlmError::authentication(format!("Invalid API key format: {}", e)))?,
        );

        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        let response = self
            .http_client
            .post(&url)
            .headers(headers)
            .json(&request)
            .send()
            .await
            .map_err(|e| LlmError::Network { source: e })?;

        let status = response.status();

        if !status.is_success() {
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unable to read error response".to_string());

            // Try to parse as Gemini error format
            if let Ok(error_response) = serde_json::from_str::<GeminiErrorResponse>(&error_body) {
                return Err(Self::map_error(
                    error_response.error.code,
                    error_response.error.message,
                ));
            }

            return Err(LlmError::api(status.as_u16(), error_body));
        }

        let generate_response = response
            .json::<GeminiGenerateContentResponse>()
            .await
            .map_err(|e| LlmError::Parse {
                message: format!("Failed to parse response: {}", e),
            })?;

        Ok(generate_response)
    }

    /// Map HTTP status codes to LlmError variants
    fn map_error(status: u16, message: String) -> LlmError {
        match status {
            400 => LlmError::InvalidRequest { message },
            401 | 403 => LlmError::Authentication { message },
            404 => LlmError::Api { status, message },
            429 => LlmError::RateLimit {
                message,
                retry_after: None,
            },
            500 | 503 => LlmError::Api { status, message },
            _ => LlmError::Api { status, message },
        }
    }

    /// Get the provider name
    pub fn provider_name(&self) -> &str {
        "Google"
    }

    /// Create a message builder for ergonomic API usage
    pub fn message_builder(&self) -> crate::gemini::builder::MessageBuilder {
        crate::gemini::builder::MessageBuilder::new(self)
    }
}

// Implement LlmClient trait for generic usage
impl crate::client::LlmClient for GeminiClient {
    async fn complete(
        &self,
        request: crate::types::CompletionRequest,
    ) -> Result<crate::types::CompletionResponse, LlmError> {
        // Convert generic request to Gemini-specific request
        let gemini_request = Self::from_completion_request(request)?;

        // Extract model from request
        let model = gemini_request.model.clone();

        // Make API call
        let response = self.generate_content(model, gemini_request).await?;

        // Convert Gemini response to generic response
        Self::to_completion_response(response)
    }

    fn provider_name(&self) -> &str {
        "Google"
    }

    fn model_name(&self) -> &str {
        "gemini-3-pro-preview"  // Default
    }
}
```

**Key Implementation Points**:
1. ✅ Model ID goes in URL, NOT request body
2. ✅ Auth header is `x-goog-api-key` (different from Claude/OpenAI)
3. ✅ Error mapping from Gemini error codes
4. ✅ Implements `LlmClient` trait for generic usage
5. ✅ Provides builder pattern for ergonomic usage

---

### Phase 4: Implement Builder (2-3 hours)

#### 4.1 Message Builder Pattern

**Create `nocodo-llm-sdk/src/gemini/builder.rs`**:

```rust
use crate::{
    error::LlmError,
    gemini::{
        client::GeminiClient,
        types::*,
    },
};

/// Fluent builder for Gemini generate content requests
pub struct MessageBuilder<'a> {
    client: &'a GeminiClient,
    model: Option<String>,
    contents: Vec<GeminiContent>,
    system_instruction: Option<String>,
    tools: Vec<GeminiTool>,
    generation_config: GenerationConfig,
}

impl<'a> MessageBuilder<'a> {
    pub fn new(client: &'a GeminiClient) -> Self {
        Self {
            client,
            model: None,
            contents: Vec::new(),
            system_instruction: None,
            tools: Vec::new(),
            generation_config: GenerationConfig::default(),
        }
    }

    /// Set the model to use
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Add a user message
    pub fn user_message(mut self, text: impl Into<String>) -> Self {
        self.contents.push(GeminiContent {
            role: GeminiRole::User,
            parts: vec![GeminiPart {
                text: Some(text.into()),
                inline_data: None,
                function_call: None,
                function_response: None,
                thought_signature: None,
            }],
        });
        self
    }

    /// Add a model (assistant) message
    pub fn model_message(mut self, text: impl Into<String>) -> Self {
        self.contents.push(GeminiContent {
            role: GeminiRole::Model,
            parts: vec![GeminiPart {
                text: Some(text.into()),
                inline_data: None,
                function_call: None,
                function_response: None,
                thought_signature: None,
            }],
        });
        self
    }

    /// Add a complete Content object (for complex scenarios)
    pub fn content(mut self, content: GeminiContent) -> Self {
        self.contents.push(content);
        self
    }

    /// Set system instruction
    pub fn system(mut self, text: impl Into<String>) -> Self {
        self.system_instruction = Some(text.into());
        self
    }

    /// Set thinking level (Gemini 3 specific)
    pub fn thinking_level(mut self, level: impl Into<String>) -> Self {
        self.generation_config.thinking_config = Some(ThinkingConfig {
            thinking_level: level.into(),
        });
        self
    }

    /// Set temperature
    pub fn temperature(mut self, temp: f32) -> Self {
        self.generation_config.temperature = Some(temp);
        self
    }

    /// Set max output tokens
    pub fn max_output_tokens(mut self, tokens: u32) -> Self {
        self.generation_config.max_output_tokens = Some(tokens);
        self
    }

    /// Set top_p
    pub fn top_p(mut self, top_p: f32) -> Self {
        self.generation_config.top_p = Some(top_p);
        self
    }

    /// Set top_k
    pub fn top_k(mut self, top_k: u32) -> Self {
        self.generation_config.top_k = Some(top_k);
        self
    }

    /// Add a tool (function declaration)
    pub fn tool(mut self, tool: GeminiTool) -> Self {
        self.tools.push(tool);
        self
    }

    /// Send the request
    pub async fn send(self) -> Result<GeminiGenerateContentResponse, LlmError> {
        // Validate required fields
        let model = self.model.ok_or_else(|| {
            LlmError::InvalidRequest {
                message: "Model is required".to_string(),
            }
        })?;

        if self.contents.is_empty() {
            return Err(LlmError::InvalidRequest {
                message: "At least one message is required".to_string(),
            });
        }

        // Build request
        let request = GeminiGenerateContentRequest {
            contents: self.contents,
            system_instruction: self.system_instruction.map(|text| GeminiContent {
                role: GeminiRole::User,  // System instructions use user role
                parts: vec![GeminiPart {
                    text: Some(text),
                    inline_data: None,
                    function_call: None,
                    function_response: None,
                    thought_signature: None,
                }],
            }),
            tools: if self.tools.is_empty() {
                None
            } else {
                Some(self.tools)
            },
            tool_config: None,
            generation_config: Some(self.generation_config),
        };

        // Send request
        self.client.generate_content(model, request).await
    }
}
```

**Usage Example**:
```rust
let client = GeminiClient::new("your-api-key")?;

let response = client
    .message_builder()
    .model("gemini-3-pro-preview")
    .system("You are a helpful coding assistant")
    .user_message("Write a hello world program in Rust")
    .thinking_level("high")
    .temperature(1.0)
    .max_output_tokens(1024)
    .send()
    .await?;
```

---

### Phase 5: Tool Format Adapter (1-2 hours)

#### 5.1 Implement ProviderToolFormat

**Create `nocodo-llm-sdk/src/gemini/tools.rs`**:

```rust
use crate::{
    gemini::types::{GeminiFunctionDeclaration, GeminiTool},
    tools::{Tool, ToolChoice, ProviderToolFormat},
};
use serde_json::Value;

/// Gemini-specific tool format adapter
pub struct GeminiToolFormat;

impl ProviderToolFormat for GeminiToolFormat {
    type ProviderTool = GeminiTool;

    fn to_provider_tool(tool: &Tool) -> Self::ProviderTool {
        GeminiTool {
            function_declarations: Some(vec![GeminiFunctionDeclaration {
                name: tool.name.clone(),
                description: tool.description.clone(),
                parameters: serde_json::to_value(&tool.parameters)
                    .unwrap_or(Value::Object(serde_json::Map::new())),
            }]),
        }
    }

    fn to_provider_tool_choice(choice: &ToolChoice) -> Value {
        // Gemini uses toolConfig for tool choice
        match choice {
            ToolChoice::Auto => serde_json::json!({
                "functionCallingConfig": {
                    "mode": "AUTO"
                }
            }),
            ToolChoice::Required => serde_json::json!({
                "functionCallingConfig": {
                    "mode": "ANY"
                }
            }),
            ToolChoice::None => serde_json::json!({
                "functionCallingConfig": {
                    "mode": "NONE"
                }
            }),
            ToolChoice::Specific(name) => serde_json::json!({
                "functionCallingConfig": {
                    "mode": "ANY",
                    "allowedFunctionNames": [name]
                }
            }),
        }
    }
}
```

---

### Phase 6: Module Exports (30 minutes)

#### 6.1 Gemini Module File

**Create `nocodo-llm-sdk/src/gemini/mod.rs`**:

```rust
//! Google Gemini API client and types
//!
//! Supports Gemini 3 Pro and Gemini 3 Flash models with reasoning capabilities.

pub mod builder;
pub mod client;
pub mod tools;
pub mod types;

pub use builder::MessageBuilder;
pub use client::GeminiClient;
pub use tools::GeminiToolFormat;
pub use types::*;

// Re-export model constants
pub use crate::models::gemini::*;
```

#### 6.2 Update Main Library

**Update `nocodo-llm-sdk/src/lib.rs`**:

```rust
pub mod gemini;  // ADD THIS

// Update re-exports
pub use gemini::GeminiClient;
```

---

### Phase 7: Testing (3-4 hours)

#### 7.1 Unit Tests

**Add to `nocodo-llm-sdk/src/gemini/client.rs`**:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = GeminiClient::new("test-key");
        assert!(client.is_ok());
    }

    #[test]
    fn test_client_creation_empty_key() {
        let client = GeminiClient::new("");
        assert!(client.is_err());
    }

    #[test]
    fn test_url_construction() {
        let client = GeminiClient::new("test-key").unwrap();
        // Test that URL is constructed correctly with model ID
    }
}
```

#### 7.2 Integration Tests

**Create `nocodo-llm-sdk/tests/gemini_integration.rs`**:

```rust
use nocodo_llm_sdk::gemini::{GeminiClient, types::*};

#[tokio::test]
#[ignore] // Requires GEMINI_API_KEY
async fn test_gemini_3_pro_simple_completion() {
    let api_key = std::env::var("GEMINI_API_KEY")
        .expect("GEMINI_API_KEY environment variable required");

    let client = GeminiClient::new(api_key)
        .expect("Failed to create Gemini client");

    let response = client
        .message_builder()
        .model("gemini-3-pro-preview")
        .user_message("What is 2+2? Answer in one word.")
        .max_output_tokens(50)
        .send()
        .await
        .expect("Failed to get response");

    assert!(!response.candidates.is_empty());
    let text = response.candidates[0].content.parts[0]
        .text
        .as_ref()
        .expect("Expected text response");
    assert!(text.contains("4") || text.to_lowercase().contains("four"));
}

#[tokio::test]
#[ignore]
async fn test_gemini_3_flash_with_thinking_level() {
    let api_key = std::env::var("GEMINI_API_KEY")
        .expect("GEMINI_API_KEY required");

    let client = GeminiClient::new(api_key).unwrap();

    let response = client
        .message_builder()
        .model("gemini-3-flash-preview")
        .thinking_level("low")  // Fast response
        .user_message("Hello, Gemini!")
        .max_output_tokens(100)
        .send()
        .await
        .expect("Failed to get response");

    assert!(!response.candidates.is_empty());
    println!("Response: {:?}", response);
}

#[tokio::test]
#[ignore]
async fn test_gemini_with_system_instruction() {
    let api_key = std::env::var("GEMINI_API_KEY").unwrap();
    let client = GeminiClient::new(api_key).unwrap();

    let response = client
        .message_builder()
        .model("gemini-3-pro-preview")
        .system("You are a helpful coding assistant. Always respond concisely.")
        .user_message("Write a hello world function in Python")
        .max_output_tokens(500)
        .send()
        .await
        .expect("Failed");

    assert!(!response.candidates.is_empty());
}

#[tokio::test]
#[ignore]
async fn test_gemini_multi_turn_conversation() {
    let api_key = std::env::var("GEMINI_API_KEY").unwrap();
    let client = GeminiClient::new(api_key).unwrap();

    let response = client
        .message_builder()
        .model("gemini-3-flash-preview")
        .user_message("Hi, what's your name?")
        .model_message("I'm Gemini, a large language model from Google.")
        .user_message("What can you help me with?")
        .max_output_tokens(200)
        .send()
        .await
        .expect("Failed");

    assert!(!response.candidates.is_empty());
}
```

**Run tests**:
```bash
# Unit tests
cargo test --package nocodo-llm-sdk --lib gemini

# Integration tests (requires API key)
GEMINI_API_KEY="your-key" cargo test --test gemini_integration -- --ignored --nocapture
```

---

### Phase 8: Examples & Documentation (2-3 hours)

#### 8.1 Create Examples

**Create `nocodo-llm-sdk/examples/gemini_simple.rs`**:

```rust
//! Simple Gemini 3 Pro example
//!
//! Run with: GEMINI_API_KEY="..." cargo run --example gemini_simple

use nocodo_llm_sdk::gemini::GeminiClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("GEMINI_API_KEY")?;
    let client = GeminiClient::new(api_key)?;

    println!("=== Gemini 3 Pro Example ===\n");

    let response = client
        .message_builder()
        .model("gemini-3-pro-preview")
        .system("You are a helpful coding assistant")
        .user_message("Write a simple Rust function to calculate fibonacci numbers")
        .thinking_level("high")  // Maximum reasoning
        .temperature(1.0)        // Recommended default
        .max_output_tokens(1024)
        .send()
        .await?;

    println!("Model: {}", response.model_version);
    println!("\nResponse:");

    for candidate in &response.candidates {
        for part in &candidate.content.parts {
            if let Some(text) = &part.text {
                println!("{}", text);
            }
        }
    }

    if let Some(usage) = response.usage_metadata {
        println!("\n=== Token Usage ===");
        println!("Prompt: {}", usage.prompt_token_count);
        println!("Response: {}", usage.candidates_token_count);
        println!("Total: {}", usage.total_token_count);
    }

    Ok(())
}
```

**Create `nocodo-llm-sdk/examples/gemini_flash.rs`**:

```rust
//! Gemini 3 Flash example with fast responses
//!
//! Run with: GEMINI_API_KEY="..." cargo run --example gemini_flash

use nocodo_llm_sdk::gemini::GeminiClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("GEMINI_API_KEY")?;
    let client = GeminiClient::new(api_key)?;

    println!("=== Gemini 3 Flash (Fast Mode) ===\n");

    let response = client
        .message_builder()
        .model("gemini-3-flash-preview")
        .thinking_level("low")  // Fast response mode
        .user_message("Explain what a REST API is in one sentence")
        .max_output_tokens(200)
        .send()
        .await?;

    println!("Model: {}", response.model_version);
    println!("\nResponse:");

    for candidate in &response.candidates {
        for part in &candidate.content.parts {
            if let Some(text) = &part.text {
                println!("{}", text);
            }
        }
    }

    Ok(())
}
```

#### 8.2 Update README

**Update `nocodo-llm-sdk/README.md`**:

Add section:

```markdown
## Gemini (Google)

Google's Gemini 3 models with reasoning capabilities and thinking level controls.

### Gemini 3 Pro

```rust
use nocodo_llm_sdk::gemini::GeminiClient;

let client = GeminiClient::new("your-gemini-api-key")?;

let response = client
    .message_builder()
    .model("gemini-3-pro-preview")
    .system("You are a helpful assistant")
    .user_message("Hello, Gemini!")
    .thinking_level("high")  // Enable deep reasoning
    .temperature(1.0)        // Recommended: keep at 1.0
    .max_output_tokens(1024)
    .send()
    .await?;
```

### Gemini 3 Flash

```rust
let response = client
    .message_builder()
    .model("gemini-3-flash-preview")
    .thinking_level("low")  // Fast mode
    .user_message("Quick question...")
    .send()
    .await?;
```

### Thinking Levels

Gemini 3 supports dynamic thinking control:

- **Gemini 3 Pro**: `low`, `high` (default)
- **Gemini 3 Flash**: `minimal`, `low`, `medium`, `high` (default)

Use `low` or `minimal` for faster responses when complex reasoning isn't needed.

### Key Features

- ✅ 1M token context window
- ✅ 64k token output
- ✅ Tool/function calling
- ✅ Vision support
- ✅ Structured outputs
- ✅ Thinking level controls
- ✅ Thought signature preservation for multi-step reasoning
```

---

## Critical Implementation Notes

### 1. Thought Signatures (CRITICAL for Tool Calling)

**Problem**: Gemini 3 uses encrypted "thought signatures" to maintain reasoning context across API calls. Missing signatures cause 400 errors in function calling.

**Solution**:
- Always preserve `thought_signature` field from responses
- Include ALL signatures when sending conversation history
- For parallel function calls: only first call has signature
- For sequential function calls: each call has its own signature

**Example**:
```rust
// When processing function call responses
let mut contents = vec![];

// Previous user message
contents.push(previous_user_content);

// Model's function call WITH thought signature
contents.push(GeminiContent {
    role: GeminiRole::Model,
    parts: vec![GeminiPart {
        function_call: Some(function_call),
        thought_signature: Some(preserved_signature),  // CRITICAL
        ..Default::default()
    }],
});

// User's function response
contents.push(function_response_content);
```

### 2. Temperature Recommendation

**Important**: Gemini 3 documentation strongly recommends keeping temperature at `1.0` (default).

- Lowering temperature may cause looping or degraded performance
- Unlike previous models, don't tune temperature for determinism
- Use thinking level instead to control response quality/speed

### 3. Model ID in URL vs Body

**Key Difference**: Unlike Claude/OpenAI, Gemini includes model ID in the endpoint URL, NOT in the request body.

```rust
// CORRECT
let url = format!(
    "{}/v1beta/models/{}:generateContent",
    base_url, model_id
);

// Request body does NOT include model field
```

### 4. Role Mapping

Gemini uses `model` instead of `assistant` for responses:

```rust
pub enum GeminiRole {
    User,
    Model,  // Maps to "assistant" in generic types
}
```

---

## Success Criteria

### Implementation Complete When:

1. ✅ **Code compiles and all tests pass**
   ```bash
   cargo build
   cargo test --package nocodo-llm-sdk
   cargo clippy -- -D warnings
   ```

2. ✅ **Integration tests pass with real API**
   ```bash
   GEMINI_API_KEY="..." cargo test --test gemini_integration -- --ignored
   ```

3. ✅ **Both models work**
   - Gemini 3 Pro with high thinking level
   - Gemini 3 Flash with multiple thinking levels
   - System instructions
   - Multi-turn conversations

4. ✅ **Examples run successfully**
   ```bash
   GEMINI_API_KEY="..." cargo run --example gemini_simple
   GEMINI_API_KEY="..." cargo run --example gemini_flash
   ```

5. ✅ **Documentation complete**
   - Model constants in `models.rs`
   - Model metadata registered
   - README updated with usage examples
   - API docs in rustdoc comments

6. ✅ **Code quality maintained**
   - No clippy warnings
   - Consistent with existing provider patterns
   - Well-tested (unit + integration tests)
   - Thought signature handling implemented correctly

---

## Timeline Estimate

- **Phase 1** (Foundation): 3-4 hours
- **Phase 2** (Types): 3-4 hours
- **Phase 3** (Client): 4-5 hours
- **Phase 4** (Builder): 2-3 hours
- **Phase 5** (Tools): 1-2 hours
- **Phase 6** (Exports): 30 minutes
- **Phase 7** (Testing): 3-4 hours
- **Phase 8** (Examples/Docs): 2-3 hours

**Total**: ~19-26 hours

**Fast Track** (MVP): ~12-16 hours
- Skip tool calling support initially
- Minimal examples
- Basic testing only

---

## Future Enhancements

### After Initial Implementation:

1. **Streaming Support**
   - Implement `streamGenerateContent` endpoint
   - Add streaming examples

2. **Advanced Tool Calling**
   - Parallel function calling
   - Sequential multi-step function calling
   - Full thought signature management

3. **Multimodal Support**
   - Image inputs (`inline_data` with base64)
   - Vision examples

4. **Media Resolution Control**
   - `media_resolution` parameter for images/video
   - Optimal defaults per media type

5. **Grounding with Google Search**
   - Built-in tool integration
   - Search grounding examples

6. **Context Caching**
   - Implement caching support
   - Cost optimization examples

---

## Migration from Other Providers

### For Users Coming from Claude:

| Claude | Gemini | Notes |
|--------|--------|-------|
| `claude-sonnet-4-5` | `gemini-3-pro-preview` | Similar capabilities |
| `max_tokens` | `max_output_tokens` | Different param name |
| `system` message | `system_instruction` | Different field |
| Cache control | Thinking level | Different optimization approach |

### For Users Coming from OpenAI:

| OpenAI | Gemini | Notes |
|--------|--------|-------|
| `gpt-4o` | `gemini-3-pro-preview` | Similar tier |
| `gpt-4o-mini` | `gemini-3-flash-preview` | Fast variant |
| `reasoning_effort` | `thinking_level` | Maps to Gemini's levels |
| Assistant role | Model role | Terminology difference |

---

## References

- **Gemini 3 Developer Guide**: `external-docs/gemini_3_developer_guide.md`
- **Gemini API Reference**: `external-docs/gemini_api_reference.md`
- **Task Plan**: `tasks/nocodo-llm-sdk-creation.md`
- **Assessment Report**: See agent output from 2026-01-18
- **Existing Implementations**:
  - `nocodo-llm-sdk/src/claude/` (Reference pattern)
  - `nocodo-llm-sdk/src/openai/` (Similar OpenAI-style API)

---

## Next Steps

1. **Obtain API Key**
   - Get Gemini API key from Google AI Studio
   - Set `GEMINI_API_KEY` environment variable

2. **Start Implementation**
   - Begin with Phase 1 (Foundation)
   - Follow implementation plan sequentially
   - Test each phase before proceeding

3. **Validation**
   - Run integration tests with real API
   - Test both Pro and Flash models
   - Verify thinking level controls work

4. **Documentation**
   - Update README
   - Add usage examples
   - Document Gemini-specific features

5. **Release**
   - Tag as appropriate version (v0.x.0)
   - Update changelog
   - Announce new provider support
