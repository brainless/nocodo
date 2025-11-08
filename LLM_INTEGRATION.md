# LLM Integration Guide

A comprehensive guide for integrating new LLM providers and models into nocodo's software architecture.

**Document Version**: 1.0
**Last Updated**: 2025-11-08
**Status**: Production Guide

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Architecture Overview](#architecture-overview)
3. [Prerequisites](#prerequisites)
4. [Integration Checklist](#integration-checklist)
5. [Step-by-Step Integration Process](#step-by-step-integration-process)
6. [Testing & Validation](#testing--validation)
7. [Common Patterns & Best Practices](#common-patterns--best-practices)
8. [Troubleshooting](#troubleshooting)
9. [Reference Documentation](#reference-documentation)

---

## Executive Summary

### What This Document Covers

This guide explains how to integrate new LLM providers (like OpenAI, Anthropic, zAI, xAI) and models into the nocodo codebase. The integration process follows the **Adapter Pattern** architecture, which provides:

- **Clean separation** between providers
- **Isolated testing** capabilities
- **Type-safe** API interactions
- **Easy maintenance** and extensibility

### Time Estimate

- **Simple integration** (OpenAI-compatible API): ~1 hour
- **Complex integration** (custom API format): ~2-3 hours
- **Full integration with tests**: ~3-4 hours

### Prerequisites

Before starting, you should be familiar with:
- Rust programming language
- async/await patterns in Rust
- RESTful APIs and JSON
- The provider's API documentation

---

## Architecture Overview

### Three-Layer Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Application Layer                     │
│                  (LlmAgent, Manager)                     │
│         Uses: LlmClient trait (provider-agnostic)        │
└─────────────────────┬───────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────┐
│                 Unified Client Layer                     │
│                  UnifiedLlmClient                        │
│         Delegates to ProviderAdapter                     │
└─────────────────────┬───────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────┐
│              Provider Adapter Layer                      │
│    ClaudeAdapter | ResponsesApiAdapter | GlmAdapter     │
│         Handles provider-specific API details            │
└─────────────────────────────────────────────────────────┘
```

### Key Components

#### 1. Core Traits

**Location**: `manager/src/llm_client.rs`

- **`LlmClient`** - Main interface used by application code
  - `complete()` - Make completion requests
  - `extract_tool_calls_from_response()` - Parse tool calls
  - `provider()`, `model()` - Metadata

- **`ProviderAdapter`** - Low-level provider-specific interface
  - `prepare_request()` - Convert unified → provider format
  - `send_request()` - Make HTTP request
  - `parse_response()` - Convert provider → unified format
  - `extract_tool_calls()` - Extract tool calls from response

- **`ProviderRequest`** - Serialization interface
  - `to_json()` - Convert to JSON for HTTP body
  - `custom_headers()` - Provider-specific HTTP headers

- **`LlmProvider`** - High-level provider management
  - `list_available_models()` - List supported models
  - `create_client()` - Factory method for clients
  - `test_connection()` - Validate API connectivity

- **`LlmModel`** - Model metadata and capabilities
  - Model ID, name, context length
  - Capabilities (streaming, tools, vision, reasoning)
  - Pricing information

#### 2. Unified Data Types

**Location**: `manager/src/llm_client.rs`

These types are used across ALL providers:

```rust
// Request types
pub struct LlmCompletionRequest {
    pub model: String,
    pub messages: Vec<LlmMessage>,
    pub tools: Option<Vec<LlmTool>>,
    pub tool_choice: Option<ToolChoice>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub stream: Option<bool>,
}

pub struct LlmMessage {
    pub role: String,              // "system", "user", "assistant", "tool"
    pub content: Option<String>,
    pub tool_calls: Option<Vec<LlmToolCall>>,
    pub tool_call_id: Option<String>,
}

// Response types
pub struct LlmCompletionResponse {
    pub id: String,
    pub model: String,
    pub choices: Vec<LlmChoice>,
    pub usage: Option<LlmUsage>,
}

pub struct LlmToolCall {
    pub id: String,
    pub r#type: String,
    pub function: LlmToolCallFunction,
}
```

#### 3. File Structure

```
manager/
├── src/
│   ├── llm_client.rs                    # Core traits, factory, types
│   ├── llm_client/
│   │   ├── adapters/
│   │   │   ├── mod.rs                   # Adapter exports
│   │   │   ├── trait_adapter.rs         # ProviderAdapter trait
│   │   │   ├── claude_messages.rs       # Claude adapter
│   │   │   ├── responses_api.rs         # GPT-5 Responses API
│   │   │   └── glm_chat_completions.rs  # zAI GLM adapter
│   │   ├── types/
│   │   │   ├── mod.rs                   # Type exports
│   │   │   ├── claude_types.rs          # Claude-specific types
│   │   │   ├── responses_types.rs       # Responses API types
│   │   │   └── glm_types.rs             # GLM-specific types
│   │   └── unified_client.rs            # UnifiedLlmClient impl
│   ├── llm_providers/
│   │   ├── mod.rs                       # Provider exports
│   │   ├── anthropic.rs                 # AnthropicProvider
│   │   ├── openai.rs                    # OpenAiProvider
│   │   ├── xai.rs                       # XaiProvider (Grok)
│   │   └── zai.rs                       # ZaiProvider (GLM)
│   ├── models.rs                        # LlmProviderConfig
│   └── config.rs                        # AppConfig, ApiKeysConfig
└── tests/
    ├── common/
    │   └── llm_config.rs                # Test configuration helpers
    ├── llm_e2e_real_test.rs             # E2E integration tests
    └── integration/
        └── llm_agent.rs                 # Integration tests

run_llm_e2e_test.sh                      # E2E test runner script
```

---

## Prerequisites

### Required Knowledge

1. **Rust Programming**
   - async/await and futures
   - trait implementations
   - error handling with `Result<T, E>`
   - serde for JSON serialization

2. **Provider API Knowledge**
   - API endpoint URLs
   - Authentication method (API key, OAuth, etc.)
   - Request/response format
   - Tool calling format (if supported)
   - Rate limits and quotas

3. **Tools & Setup**
   - Rust toolchain (1.70+)
   - API key for the provider
   - HTTP client experience (reqwest)
   - JSON manipulation skills

### Environment Setup

1. **API Key Configuration**

   Add your API key to `~/.config/nocodo/manager.toml`:

   ```toml
   [api_keys]
   your_provider_api_key = "sk-..."
   ```

2. **Environment Variables** (for testing)

   ```bash
   export YOUR_PROVIDER_API_KEY="sk-..."
   export PROVIDER="your_provider"
   export MODEL="your_model"
   ```

3. **Development Dependencies**

   Already included in `Cargo.toml`:
   ```toml
   [dependencies]
   reqwest = { version = "0.11", features = ["json"] }
   serde = { version = "1.0", features = ["derive"] }
   serde_json = "1.0"
   async-trait = "0.1"
   anyhow = "1.0"
   tracing = "0.1"
   ```

---

## Integration Checklist

Use this checklist to track your integration progress:

### Phase 1: Planning & Research
- [ ] Read provider's API documentation
- [ ] Obtain API key for testing
- [ ] Identify API endpoint(s)
- [ ] Document request/response format
- [ ] Identify authentication method
- [ ] Check tool calling support
- [ ] Review rate limits

### Phase 2: Type Definitions
- [ ] Create provider-specific request type
- [ ] Create provider-specific response type
- [ ] Implement `ProviderRequest` trait
- [ ] Add to `manager/src/llm_client/types/`
- [ ] Export from `types/mod.rs`

### Phase 3: Adapter Implementation
- [ ] Create adapter struct
- [ ] Implement `ProviderAdapter` trait
- [ ] Implement request conversion
- [ ] Implement response parsing
- [ ] Implement tool call extraction
- [ ] Add to `manager/src/llm_client/adapters/`
- [ ] Export from `adapters/mod.rs`

### Phase 4: Provider Registration
- [ ] Create provider struct
- [ ] Create model struct(s)
- [ ] Implement `LlmProvider` trait
- [ ] Implement `LlmModel` trait(s)
- [ ] Add to `manager/src/llm_providers/`
- [ ] Export from `llm_providers/mod.rs`

### Phase 5: Factory Integration
- [ ] Update `create_llm_client()` in `llm_client.rs`
- [ ] Add provider/model matching logic
- [ ] Update `ApiKeysConfig` in `config.rs`
- [ ] Add config validation

### Phase 6: Testing
- [ ] Add test config in `tests/common/llm_config.rs`
- [ ] Write unit tests for adapter
- [ ] Write integration test
- [ ] Test with `run_llm_e2e_test.sh`
- [ ] Verify tool calling (if supported)
- [ ] Test error handling

### Phase 7: Documentation
- [ ] Document API quirks
- [ ] Add usage examples
- [ ] Update this guide
- [ ] Add to README (if applicable)

---

## Step-by-Step Integration Process

### Step 1: Define Provider-Specific Types

**Location**: `manager/src/llm_client/types/your_provider_types.rs`

#### 1.1 Create Request Type

```rust
use serde::{Deserialize, Serialize};
use anyhow::Result;
use super::super::adapters::trait_adapter::ProviderRequest;

/// Request format for YourProvider API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YourProviderRequest {
    pub model: String,
    pub messages: Vec<serde_json::Value>,

    // Provider-specific fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    // Add other provider-specific fields here
}

impl ProviderRequest for YourProviderRequest {
    fn to_json(&self) -> Result<serde_json::Value> {
        Ok(serde_json::to_value(self)?)
    }

    fn custom_headers(&self) -> Vec<(String, String)> {
        vec![
            // Add provider-specific headers if needed
            // ("X-Custom-Header".to_string(), "value".to_string()),
        ]
    }
}
```

#### 1.2 Create Response Type

```rust
/// Response format from YourProvider API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YourProviderResponse {
    pub id: String,
    pub model: String,
    pub choices: Vec<YourProviderChoice>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<YourProviderUsage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YourProviderChoice {
    pub index: u32,
    pub message: YourProviderMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YourProviderMessage {
    pub role: String,
    pub content: Option<String>,

    // Add tool calling fields if supported
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YourProviderUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}
```

#### 1.3 Export Types

**File**: `manager/src/llm_client/types/mod.rs`

```rust
pub mod your_provider_types;
pub use your_provider_types::{
    YourProviderRequest,
    YourProviderResponse,
    // ... other types
};
```

### Step 2: Implement Provider Adapter

**Location**: `manager/src/llm_client/adapters/your_provider_adapter.rs`

#### 2.1 Create Adapter Struct

```rust
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;

use crate::llm_client::{
    LlmCompletionRequest, LlmCompletionResponse, LlmToolCall,
    LlmMessage, LlmChoice, LlmUsage,
};
use crate::llm_client::adapters::trait_adapter::{ProviderAdapter, ProviderRequest};
use crate::llm_client::types::{YourProviderRequest, YourProviderResponse};
use crate::models::LlmProviderConfig;

/// Adapter for YourProvider API
pub struct YourProviderAdapter {
    config: LlmProviderConfig,
    client: Client,
}

impl YourProviderAdapter {
    pub fn new(config: LlmProviderConfig) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()?;

        Ok(Self { config, client })
    }
}
```

#### 2.2 Implement Request Conversion

```rust
impl YourProviderAdapter {
    /// Convert unified request to provider-specific format
    fn convert_to_provider_request(
        &self,
        request: LlmCompletionRequest,
    ) -> Result<YourProviderRequest> {
        // Convert messages to provider format
        let messages: Vec<Value> = request
            .messages
            .iter()
            .map(|msg| {
                let mut message = serde_json::json!({
                    "role": msg.role
                });

                // Add content field
                if let Some(content) = &msg.content {
                    message["content"] = Value::String(content.clone());
                }

                // Handle tool calls for assistant messages
                if msg.role == "assistant" {
                    if let Some(tool_calls) = &msg.tool_calls {
                        message["tool_calls"] = serde_json::to_value(tool_calls)
                            .unwrap_or(Value::Array(vec![]));
                    }
                }

                // Handle tool message responses
                if msg.role == "tool" {
                    if let Some(tool_call_id) = &msg.tool_call_id {
                        message["tool_call_id"] = Value::String(tool_call_id.clone());
                    }
                }

                message
            })
            .collect();

        // Build provider-specific request
        Ok(YourProviderRequest {
            model: request.model,
            messages,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
        })
    }

    /// Convert provider response to unified format
    fn convert_from_provider_response(
        &self,
        response: YourProviderResponse,
    ) -> Result<LlmCompletionResponse> {
        let choices = response
            .choices
            .into_iter()
            .map(|choice| LlmChoice {
                index: choice.index,
                message: Some(LlmMessage {
                    role: choice.message.role,
                    content: choice.message.content,
                    tool_calls: choice.message.tool_calls.and_then(|calls| {
                        serde_json::from_value(Value::Array(calls)).ok()
                    }),
                    function_call: None,
                    tool_call_id: None,
                }),
                delta: None,
                finish_reason: choice.finish_reason,
                tool_calls: None,
            })
            .collect();

        Ok(LlmCompletionResponse {
            id: response.id,
            object: "chat.completion".to_string(),
            created: 0,
            model: response.model,
            choices,
            usage: response.usage.map(|u| LlmUsage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            }),
        })
    }
}
```

#### 2.3 Implement ProviderAdapter Trait

```rust
#[async_trait]
impl ProviderAdapter for YourProviderAdapter {
    fn get_api_url(&self) -> String {
        // Use base_url from config if provided, otherwise use default
        if let Some(base_url) = &self.config.base_url {
            format!("{}/v1/chat/completions", base_url.trim_end_matches('/'))
        } else {
            "https://api.yourprovider.com/v1/chat/completions".to_string()
        }
    }

    fn supports_native_tools(&self) -> bool {
        // Return true if this provider supports tool calling
        true
    }

    fn prepare_request(
        &self,
        request: LlmCompletionRequest,
    ) -> Result<Box<dyn ProviderRequest>> {
        let provider_request = self.convert_to_provider_request(request)?;
        Ok(Box::new(provider_request))
    }

    async fn send_request(
        &self,
        request: Box<dyn ProviderRequest>,
    ) -> Result<reqwest::Response> {
        let json = request.to_json()?;

        let mut req = self.client
            .post(self.get_api_url())
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json");

        // Add custom headers
        for (key, value) in request.custom_headers() {
            req = req.header(key, value);
        }

        let response = req.json(&json).send().await?;

        Ok(response)
    }

    fn parse_response(&self, response_text: &str) -> Result<LlmCompletionResponse> {
        let provider_response: YourProviderResponse =
            serde_json::from_str(response_text)?;
        self.convert_from_provider_response(provider_response)
    }

    fn extract_tool_calls(&self, response: &LlmCompletionResponse) -> Vec<LlmToolCall> {
        response
            .choices
            .first()
            .and_then(|choice| choice.message.as_ref())
            .and_then(|message| message.tool_calls.clone())
            .unwrap_or_default()
    }

    fn provider_name(&self) -> &str {
        &self.config.provider
    }

    fn model_name(&self) -> &str {
        &self.config.model
    }
}
```

#### 2.4 Export Adapter

**File**: `manager/src/llm_client/adapters/mod.rs`

```rust
pub mod your_provider_adapter;
pub use your_provider_adapter::YourProviderAdapter;
```

### Step 3: Create Provider Implementation

**Location**: `manager/src/llm_providers/your_provider.rs`

#### 3.1 Define Provider Struct

```rust
use crate::llm_client::{
    create_llm_client, LlmClient, LlmModel, LlmProvider,
    ModelCapabilities, ModelPricing, ProviderError, ProviderType,
};
use crate::models::LlmProviderConfig;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// YourProvider implementation
pub struct YourProviderImpl {
    config: LlmProviderConfig,
    models: HashMap<String, Arc<dyn LlmModel>>,
}

impl YourProviderImpl {
    pub fn new(config: LlmProviderConfig) -> Result<Self> {
        let mut provider = Self {
            config,
            models: HashMap::new(),
        };
        provider.initialize_models();
        Ok(provider)
    }

    fn initialize_models(&mut self) {
        // Register all models for this provider
        let models: Vec<Arc<dyn LlmModel>> = vec![
            Arc::new(YourModel::new("your-model-v1")),
            Arc::new(YourModel::new("your-model-v2")),
        ];

        for model in models {
            self.models.insert(model.id().to_string(), model);
        }
    }
}
```

#### 3.2 Implement LlmProvider Trait

```rust
#[async_trait]
impl LlmProvider for YourProviderImpl {
    fn id(&self) -> &str {
        &self.config.provider
    }

    fn name(&self) -> &str {
        "Your Provider Name"
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Custom  // or OpenAICompatible, Anthropic, etc.
    }

    fn supports_streaming(&self) -> bool {
        true  // if provider supports streaming
    }

    fn supports_tool_calling(&self) -> bool {
        true  // if provider supports tool calling
    }

    fn supports_vision(&self) -> bool {
        false  // if provider supports vision/image inputs
    }

    async fn list_available_models(&self) -> Result<Vec<Arc<dyn LlmModel>>> {
        Ok(self.models.values().cloned().collect())
    }

    fn get_model(&self, model_id: &str) -> Option<Arc<dyn LlmModel>> {
        self.models.get(model_id).cloned()
    }

    async fn test_connection(&self) -> Result<()> {
        // Test connection by making a simple API call
        let client = reqwest::Client::new();
        let response = client
            .get("https://api.yourprovider.com/v1/models")
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .send()
            .await?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(ProviderError::Authentication("Invalid API key".to_string()).into())
        }
    }

    fn create_client(&self, model_id: &str) -> Result<Box<dyn LlmClient>> {
        let mut config = self.config.clone();
        config.model = model_id.to_string();
        create_llm_client(config)
    }
}
```

#### 3.3 Implement Model Struct

```rust
/// Your model implementation
pub struct YourModel {
    id: String,
    capabilities: ModelCapabilities,
    pricing: Option<ModelPricing>,
}

impl YourModel {
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            capabilities: ModelCapabilities {
                supports_streaming: true,
                supports_tool_calling: true,
                supports_vision: false,
                supports_reasoning: false,
                supports_json_mode: true,
            },
            pricing: Some(ModelPricing {
                input_cost_per_million_tokens: 3.0,
                output_cost_per_million_tokens: 15.0,
                reasoning_cost_per_million_tokens: None,
            }),
        }
    }
}

impl LlmModel for YourModel {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        "Your Model Name"
    }

    fn provider_id(&self) -> &str {
        "yourprovider"
    }

    fn context_length(&self) -> u32 {
        128000  // Maximum context window size
    }

    fn max_output_tokens(&self) -> Option<u32> {
        Some(4096)
    }

    fn supports_streaming(&self) -> bool {
        self.capabilities.supports_streaming
    }

    fn supports_tool_calling(&self) -> bool {
        self.capabilities.supports_tool_calling
    }

    fn supports_vision(&self) -> bool {
        self.capabilities.supports_vision
    }

    fn supports_reasoning(&self) -> bool {
        self.capabilities.supports_reasoning
    }

    fn input_cost_per_token(&self) -> Option<f64> {
        self.pricing
            .as_ref()
            .map(|p| p.input_cost_per_million_tokens / 1_000_000.0)
    }

    fn output_cost_per_token(&self) -> Option<f64> {
        self.pricing
            .as_ref()
            .map(|p| p.output_cost_per_million_tokens / 1_000_000.0)
    }

    fn default_temperature(&self) -> Option<f32> {
        Some(0.7)
    }

    fn default_max_tokens(&self) -> Option<u32> {
        Some(2048)
    }

    fn estimate_tokens(&self, text: &str) -> u32 {
        // Simple estimation: ~4 characters per token
        (text.len() as f32 / 4.0).ceil() as u32
    }
}
```

#### 3.4 Export Provider

**File**: `manager/src/llm_providers/mod.rs`

```rust
pub mod your_provider;
pub use your_provider::YourProviderImpl;
```

### Step 4: Register in Factory

**Location**: `manager/src/llm_client.rs`

Update the `create_llm_client()` function:

```rust
pub fn create_llm_client(config: LlmProviderConfig) -> Result<Box<dyn LlmClient>> {
    match (
        config.provider.to_lowercase().as_str(),
        config.model.as_str(),
    ) {
        // ... existing providers ...

        // YourProvider models
        ("yourprovider", _) => {
            let adapter = Box::new(adapters::YourProviderAdapter::new(config.clone())?);
            Ok(Box::new(UnifiedLlmClient::new(adapter, config)?))
        }

        // ... rest of matches ...
    }
}
```

### Step 5: Add Configuration Support

**Location**: `manager/src/config.rs`

#### 5.1 Update ApiKeysConfig

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeysConfig {
    // ... existing fields ...

    #[serde(skip_serializing_if = "Option::is_none")]
    pub yourprovider_api_key: Option<String>,
}
```

#### 5.2 Update AppConfig::load()

Ensure the config loading logic can read your new API key field.

### Step 6: Add Test Configuration

**Location**: `manager/tests/common/llm_config.rs`

```rust
impl LlmProviderTestConfig {
    /// Create YourProvider configuration with validation
    pub fn yourprovider_with_validation(requested_model: Option<&str>) -> Option<Self> {
        // Check if API key is available
        if env::var("YOURPROVIDER_API_KEY").is_err() {
            return None;
        }

        // Get available models from actual provider implementation
        let available_models = Self::get_available_yourprovider_models();

        // If a specific model was requested, validate it exists
        if let Some(model) = requested_model {
            if !available_models.contains(&model.to_string()) {
                eprintln!("❌ Error: Model '{}' not available for YourProvider", model);
                eprintln!("   Available models: {:?}", available_models);
                return None;
            }
        }

        Some(Self {
            name: "yourprovider".to_string(),
            models: available_models,
            api_key_env: "YOURPROVIDER_API_KEY".to_string(),
            enabled: true,
            test_prompts: LlmTestPrompts::default(),
        })
    }

    fn get_available_yourprovider_models() -> Vec<String> {
        vec![
            "your-model-v1".to_string(),
            "your-model-v2".to_string(),
        ]
    }
}
```

Update `LlmTestConfig::from_environment()`:

```rust
if env::var("YOURPROVIDER_API_KEY").is_ok() {
    if let Some(provider_config) =
        LlmProviderTestConfig::yourprovider_with_validation(forced_model.as_deref())
    {
        providers.push(provider_config);
    }
}
```

### Step 7: Update E2E Test Script

**Location**: `run_llm_e2e_test.sh`

Add your provider to the API key checking logic:

```bash
case "$PROVIDER" in
    # ... existing cases ...
    "yourprovider")
        if grep -q '^yourprovider_api_key\s*=' "$CONFIG_FILE" && ! grep -q '^#.*yourprovider_api_key' "$CONFIG_FILE"; then
            API_KEY_FOUND=true
            echo "✅ yourprovider_api_key found in config"
        fi
        ;;
esac
```

And add to the environment variable setting section:

```bash
elif [[ "$PROVIDER" == "yourprovider" ]] && grep -q '^yourprovider_api_key\s*=' "$CONFIG_FILE"; then
    YOURPROVIDER_KEY=$(grep '^yourprovider_api_key\s*=' "$CONFIG_FILE" | sed 's/.*= *"\?\([^"]*\)"\?/\1/')
    export YOURPROVIDER_API_KEY="$YOURPROVIDER_KEY"
    echo "   ✅ Set YOURPROVIDER_API_KEY from config (selected provider)"
```

---

## Testing & Validation

### Unit Tests

Create unit tests for your adapter:

**Location**: `manager/src/llm_client/adapters/your_provider_adapter.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_conversion() {
        let config = LlmProviderConfig {
            provider: "yourprovider".to_string(),
            model: "your-model-v1".to_string(),
            api_key: "test-key".to_string(),
            base_url: None,
            max_tokens: Some(1000),
            temperature: Some(0.7),
        };

        let adapter = YourProviderAdapter::new(config).unwrap();

        let request = LlmCompletionRequest {
            model: "your-model-v1".to_string(),
            messages: vec![
                LlmMessage {
                    role: "user".to_string(),
                    content: Some("Test message".to_string()),
                    tool_calls: None,
                    function_call: None,
                    tool_call_id: None,
                }
            ],
            tools: None,
            tool_choice: None,
            max_tokens: Some(1000),
            temperature: Some(0.7),
            stream: Some(false),
            functions: None,
            function_call: None,
        };

        let prepared = adapter.prepare_request(request).unwrap();
        let json = prepared.to_json().unwrap();

        assert_eq!(json["model"], "your-model-v1");
        assert!(json["messages"].is_array());
    }

    #[test]
    fn test_response_parsing() {
        let config = LlmProviderConfig {
            provider: "yourprovider".to_string(),
            model: "your-model-v1".to_string(),
            api_key: "test-key".to_string(),
            base_url: None,
            max_tokens: Some(1000),
            temperature: Some(0.7),
        };

        let adapter = YourProviderAdapter::new(config).unwrap();

        let response_json = r#"{
            "id": "test-123",
            "model": "your-model-v1",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "Test response"
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            }
        }"#;

        let parsed = adapter.parse_response(response_json).unwrap();

        assert_eq!(parsed.id, "test-123");
        assert_eq!(parsed.choices.len(), 1);
        assert_eq!(
            parsed.choices[0].message.as_ref().unwrap().content,
            Some("Test response".to_string())
        );
    }
}
```

### Integration Tests

**Location**: `manager/tests/integration/llm_agent.rs`

Add a test for your provider:

```rust
#[tokio::test]
async fn test_yourprovider_integration() {
    let api_key = match env::var("YOURPROVIDER_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            println!("Skipping YourProvider test - no API key");
            return;
        }
    };

    let config = LlmProviderConfig {
        provider: "yourprovider".to_string(),
        model: "your-model-v1".to_string(),
        api_key,
        base_url: None,
        max_tokens: Some(100),
        temperature: Some(0.7),
    };

    let client = create_llm_client(config).unwrap();

    let request = LlmCompletionRequest {
        model: "your-model-v1".to_string(),
        messages: vec![
            LlmMessage {
                role: "user".to_string(),
                content: Some("Say 'Hello, World!'".to_string()),
                tool_calls: None,
                function_call: None,
                tool_call_id: None,
            }
        ],
        tools: None,
        tool_choice: None,
        max_tokens: Some(100),
        temperature: Some(0.7),
        stream: Some(false),
        functions: None,
        function_call: None,
    };

    let response = client.complete(request).await.unwrap();

    assert!(!response.choices.is_empty());
    assert!(response.choices[0].message.is_some());
    let content = &response.choices[0].message.as_ref().unwrap().content;
    assert!(content.is_some());
    println!("Response: {:?}", content);
}
```

### E2E Testing

Run the end-to-end test:

```bash
# Set API key in config
vim ~/.config/nocodo/manager.toml
# Add: yourprovider_api_key = "your-key"

# Run E2E test
./run_llm_e2e_test.sh yourprovider your-model-v1
```

### Acceptance Criteria

Your integration is complete when:

1. ✅ **Unit tests pass**
   - Request conversion works correctly
   - Response parsing works correctly
   - Tool call extraction works (if supported)

2. ✅ **Integration test passes**
   - Can create client
   - Can make API call
   - Can parse response
   - Tool calls work (if supported)

3. ✅ **E2E test passes**
   ```bash
   ./run_llm_e2e_test.sh yourprovider your-model-v1
   ```
   - API key validation works
   - Provider/model validation works
   - Real API calls succeed
   - Response validation passes

4. ✅ **Error handling works**
   - Invalid API key → clear error
   - Network errors → graceful handling
   - Malformed responses → clear error
   - Rate limits → appropriate retry/error

5. ✅ **Documentation complete**
   - API quirks documented
   - Usage examples added
   - Configuration documented

---

## Common Patterns & Best Practices

### Pattern 1: OpenAI-Compatible APIs

Many providers (xAI/Grok, Groq, Together, etc.) use OpenAI-compatible APIs. For these:

1. **Reuse OpenAI types** (or create minimal wrappers)
2. **Focus on differences**:
   - Authentication headers
   - Base URL
   - Model names
   - Tool calling variations

**Example**: See `xai.rs` for Grok implementation - it uses standard OpenAI format with different base URL.

### Pattern 2: Custom API Formats

For providers with unique APIs (Anthropic, zAI/GLM, GPT-5 Responses API):

1. **Define complete request/response types**
2. **Implement full conversion logic**
3. **Handle format quirks carefully**

**Examples**:
- Claude: See `claude_messages.rs` - content blocks instead of simple strings
- GLM: See `glm_chat_completions.rs` - temperature parameter handling quirks
- GPT-5: See `responses_api.rs` - completely different API structure

### Pattern 3: Tool Calling Variations

Different providers handle tool calls differently:

#### OpenAI Format
```json
{
  "message": {
    "tool_calls": [{
      "id": "call_123",
      "type": "function",
      "function": {
        "name": "get_weather",
        "arguments": "{\"city\":\"SF\"}"
      }
    }]
  }
}
```

#### Claude Format
```json
{
  "content": [{
    "type": "tool_use",
    "id": "toolu_123",
    "name": "get_weather",
    "input": {"city": "SF"}
  }]
}
```

#### Handling Strategy
1. **Parse in adapter** - Convert to unified `LlmToolCall` format
2. **Extract consistently** - Implement `extract_tool_calls()` correctly
3. **Test thoroughly** - Verify tool calling works end-to-end

### Pattern 4: Error Handling

#### Provider-Specific Error Types

```rust
#[derive(Debug)]
pub enum YourProviderError {
    RateLimitExceeded { retry_after: u64 },
    InvalidApiKey,
    ModelNotFound { model: String },
    ContextLengthExceeded { max: u32, requested: u32 },
}
```

#### Graceful Degradation

```rust
async fn send_request(&self, request: Box<dyn ProviderRequest>) -> Result<reqwest::Response> {
    let response = self.client
        .post(self.get_api_url())
        .json(&request.to_json()?)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await?;

        // Parse provider-specific error
        if status == 429 {
            // Rate limit - could extract retry-after header
            return Err(anyhow::anyhow!("Rate limit exceeded: {}", error_text));
        } else if status == 401 {
            return Err(anyhow::anyhow!("Authentication failed: Invalid API key"));
        }

        return Err(anyhow::anyhow!("API error {}: {}", status, error_text));
    }

    Ok(response)
}
```

### Pattern 5: Configuration Best Practices

#### Optional vs Required Fields

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YourProviderRequest {
    // Required fields - no Option<>
    pub model: String,
    pub messages: Vec<Value>,

    // Optional fields - use skip_serializing_if
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    // Provider-specific optional field
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_parameter: Option<String>,
}
```

#### Field Name Mapping

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YourProviderRequest {
    // Use serde rename for different field names
    #[serde(rename = "max_output_tokens")]
    pub max_tokens: Option<u32>,

    // Use alias for backwards compatibility
    #[serde(alias = "temp")]
    pub temperature: Option<f32>,
}
```

### Pattern 6: Streaming Support (Future)

While streaming is not currently required, here's the pattern for when you add it:

```rust
#[async_trait]
impl ProviderAdapter for YourProviderAdapter {
    async fn stream_request(
        &self,
        request: Box<dyn ProviderRequest>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>> {
        // Implementation for streaming
        todo!("Streaming support")
    }
}
```

### Best Practices Summary

1. **Type Safety**
   - Use strong types for requests/responses
   - Avoid `serde_json::Value` where possible
   - Use `#[serde(rename)]` for API field mapping

2. **Error Handling**
   - Provide clear error messages
   - Include API error details
   - Handle rate limits gracefully

3. **Logging**
   - Log request/response for debugging
   - Use structured logging (`tracing`)
   - Don't log API keys!

4. **Testing**
   - Write unit tests for conversions
   - Write integration tests with real API
   - Use E2E test for validation

5. **Documentation**
   - Document API quirks
   - Provide usage examples
   - Update this guide with learnings

---

## Troubleshooting

### Common Issues

#### Issue 1: 400 Bad Request

**Symptoms**: API returns 400 error

**Common Causes**:
1. Missing required field
2. Invalid field value
3. Empty array when provider expects `null`
4. Wrong field name/format

**Solution**:
```rust
// Before sending, log the request JSON
tracing::debug!("Request JSON: {}", serde_json::to_string_pretty(&json)?);

// Check provider's error response
if !response.status().is_success() {
    let error_text = response.text().await?;
    tracing::error!("API error: {}", error_text);
}
```

**Example from GLM integration**:
- **Problem**: Empty `tools` array caused 400 error
- **Solution**: Set `tools` to `None` instead of `Some(vec![])`

```rust
let tools = request.tools.as_ref().and_then(|tools| {
    if tools.is_empty() {
        None  // Don't send empty array
    } else {
        Some(tools.clone())
    }
});
```

#### Issue 2: 401 Unauthorized

**Symptoms**: API returns 401 error

**Causes**:
1. Invalid API key
2. Missing authorization header
3. Wrong header format
4. Expired API key

**Solution**:
```rust
// Verify header format matches provider docs
.header("Authorization", format!("Bearer {}", self.config.api_key))

// Or for some providers:
.header("X-API-Key", &self.config.api_key)
.header("anthropic-version", "2023-06-01")  // API version
```

#### Issue 3: Tool Calls Not Working

**Symptoms**: Tool calls not extracted or executed

**Debug Steps**:

1. **Log the raw response**:
```rust
tracing::debug!("Raw response: {}", response_text);
```

2. **Check tool call format**:
```rust
// Different providers use different formats
// OpenAI: message.tool_calls
// Claude: content[].type == "tool_use"
// GPT-5: output[].type == "FunctionCall"
```

3. **Verify extraction logic**:
```rust
fn extract_tool_calls(&self, response: &LlmCompletionResponse) -> Vec<LlmToolCall> {
    // Add debug logging
    tracing::debug!("Extracting tool calls from response: {:?}", response);

    let calls = /* extraction logic */;

    tracing::debug!("Extracted {} tool calls", calls.len());
    calls
}
```

#### Issue 4: Type Deserialization Errors

**Symptoms**: `serde_json` deserialization fails

**Causes**:
1. Response format changed
2. Optional field missing
3. Type mismatch
4. Extra fields in response

**Solutions**:

1. **Use `#[serde(default)]` for optional fields**:
```rust
#[derive(Deserialize)]
pub struct YourProviderResponse {
    pub id: String,

    #[serde(default)]
    pub usage: Option<Usage>,  // Won't fail if missing
}
```

2. **Allow extra fields**:
```rust
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]  // Remove this if provider adds new fields
pub struct YourProviderResponse {
    // ...
}
```

3. **Use `serde_json::Value` for flexible parsing**:
```rust
let response: serde_json::Value = serde_json::from_str(response_text)?;

// Parse known fields manually
let id = response["id"].as_str().ok_or_else(|| anyhow::anyhow!("Missing id"))?;
```

#### Issue 5: Test Failures

**E2E test script issues**:

1. **API key not found**:
   - Check `~/.config/nocodo/manager.toml`
   - Ensure key is uncommented
   - Verify correct format: `provider_api_key = "sk-..."`

2. **Provider/model not recognized**:
   - Update `run_llm_e2e_test.sh` with your provider
   - Update `tests/common/llm_config.rs` validation
   - Check provider/model ID matches exactly

3. **Test timeout**:
   - Increase timeout in test:
     ```rust
     #[tokio::test(flavor = "multi_thread")]
     async fn test_with_timeout() {
         tokio::time::timeout(
             Duration::from_secs(120),
             actual_test()
         ).await.unwrap();
     }
     ```

### Debugging Checklist

When integration doesn't work:

- [ ] Print request JSON before sending
- [ ] Print response status code and body
- [ ] Check API documentation for recent changes
- [ ] Verify API key is valid and has quota
- [ ] Test with provider's official SDK/curl
- [ ] Compare request format with working example
- [ ] Check for required headers
- [ ] Verify endpoint URL is correct
- [ ] Test with minimal request first
- [ ] Add extensive logging/tracing
- [ ] Check for rate limiting
- [ ] Verify field names match exactly (case-sensitive)

---

## Reference Documentation

### File Locations Quick Reference

| Component | Location |
|-----------|----------|
| Core traits | `manager/src/llm_client.rs` |
| Adapter trait | `manager/src/llm_client/adapters/trait_adapter.rs` |
| Your adapter | `manager/src/llm_client/adapters/your_provider_adapter.rs` |
| Your types | `manager/src/llm_client/types/your_provider_types.rs` |
| Your provider | `manager/src/llm_providers/your_provider.rs` |
| Config | `manager/src/config.rs` |
| Test config | `manager/tests/common/llm_config.rs` |
| E2E test | `run_llm_e2e_test.sh` |

### Existing Implementations Reference

Study these implementations as examples:

#### Simple: xAI/Grok (OpenAI-compatible)
- **Types**: Uses standard OpenAI types
- **Adapter**: `ChatCompletionsAdapter` (shared with OpenAI)
- **Provider**: `manager/src/llm_providers/xai.rs`
- **Complexity**: Low - just different base URL and auth

#### Medium: zAI/GLM
- **Types**: `manager/src/llm_client/types/glm_types.rs`
- **Adapter**: `manager/src/llm_client/adapters/glm_chat_completions.rs`
- **Provider**: `manager/src/llm_providers/zai.rs`
- **Quirks**: Temperature parameter handling, thinking config
- **Complexity**: Medium - OpenAI-like but with quirks

#### Complex: Anthropic/Claude
- **Types**: `manager/src/llm_client/types/claude_types.rs`
- **Adapter**: `manager/src/llm_client/adapters/claude_messages.rs`
- **Provider**: `manager/src/llm_providers/anthropic.rs`
- **Quirks**: Content blocks, separate system field, different tool format
- **Complexity**: High - completely different API structure

#### Very Complex: OpenAI GPT-5 Responses API
- **Types**: `manager/src/llm_client/types/responses_types.rs`
- **Adapter**: `manager/src/llm_client/adapters/responses_api.rs`
- **Provider**: Part of `manager/src/llm_providers/openai.rs`
- **Quirks**: Completely different endpoint, response format, output items
- **Complexity**: Very High - fundamentally different API design

### Key Rust Crates Used

- **`serde`** (1.0) - Serialization/deserialization
- **`serde_json`** (1.0) - JSON handling
- **`reqwest`** (0.11) - HTTP client
- **`async-trait`** (0.1) - Async trait methods
- **`anyhow`** (1.0) - Error handling
- **`tracing`** (0.1) - Structured logging
- **`tokio`** (1.0) - Async runtime

### Testing Commands

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_yourprovider_integration

# Run with logging
RUST_LOG=debug cargo test -- --nocapture

# Run E2E test
./run_llm_e2e_test.sh yourprovider your-model-v1

# Check compilation
cargo check

# Format code
cargo fmt

# Run clippy (linter)
cargo clippy
```

### Configuration File Format

**Location**: `~/.config/nocodo/manager.toml`

```toml
[server]
host = "127.0.0.1"
port = 8080

[database]
path = "/var/lib/nocodo/nocodo.db"

[socket]
path = "/var/run/nocodo.sock"

[auth]
jwt_secret = "your-secret-key"

[api_keys]
anthropic_api_key = "sk-ant-..."
openai_api_key = "sk-..."
xai_api_key = "xai-..."
zai_api_key = "..."
yourprovider_api_key = "..."

# Provider-specific settings (optional)
zai_coding_plan = true
```

### Useful Links

- **nocodo Repository**: (your repo URL)
- **LLM_CLIENT_REFACTOR.md**: Architecture design document
- **LLM_REFACTOR_PROCESS.md**: Refactoring process notes

---

## Appendix: Complete Example

Here's a complete minimal integration for a hypothetical "FooAI" provider:

### A.1 Types (`foo_types.rs`)

```rust
use serde::{Deserialize, Serialize};
use anyhow::Result;
use crate::llm_client::adapters::trait_adapter::ProviderRequest;

#[derive(Debug, Serialize)]
pub struct FooAiRequest {
    pub model: String,
    pub messages: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

impl ProviderRequest for FooAiRequest {
    fn to_json(&self) -> Result<serde_json::Value> {
        Ok(serde_json::to_value(self)?)
    }
}

#[derive(Debug, Deserialize)]
pub struct FooAiResponse {
    pub id: String,
    pub model: String,
    pub message: FooAiMessage,
}

#[derive(Debug, Deserialize)]
pub struct FooAiMessage {
    pub role: String,
    pub text: String,
}
```

### A.2 Adapter (`foo_adapter.rs`)

```rust
use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;

use crate::llm_client::{
    LlmCompletionRequest, LlmCompletionResponse, LlmToolCall,
    LlmMessage, LlmChoice,
};
use crate::llm_client::adapters::trait_adapter::{ProviderAdapter, ProviderRequest};
use crate::llm_client::types::{FooAiRequest, FooAiResponse};
use crate::models::LlmProviderConfig;

pub struct FooAiAdapter {
    config: LlmProviderConfig,
    client: Client,
}

impl FooAiAdapter {
    pub fn new(config: LlmProviderConfig) -> Result<Self> {
        Ok(Self {
            config,
            client: Client::new(),
        })
    }
}

#[async_trait]
impl ProviderAdapter for FooAiAdapter {
    fn get_api_url(&self) -> String {
        "https://api.fooai.com/v1/completions".to_string()
    }

    fn supports_native_tools(&self) -> bool {
        false
    }

    fn prepare_request(&self, request: LlmCompletionRequest) -> Result<Box<dyn ProviderRequest>> {
        let messages = request.messages.iter()
            .map(|m| serde_json::json!({
                "role": m.role,
                "content": m.content.as_ref().unwrap_or(&String::new())
            }))
            .collect();

        Ok(Box::new(FooAiRequest {
            model: request.model,
            messages,
            max_tokens: request.max_tokens,
        }))
    }

    async fn send_request(&self, request: Box<dyn ProviderRequest>) -> Result<reqwest::Response> {
        let response = self.client
            .post(self.get_api_url())
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .json(&request.to_json()?)
            .send()
            .await?;
        Ok(response)
    }

    fn parse_response(&self, response_text: &str) -> Result<LlmCompletionResponse> {
        let foo_response: FooAiResponse = serde_json::from_str(response_text)?;

        Ok(LlmCompletionResponse {
            id: foo_response.id,
            object: "completion".to_string(),
            created: 0,
            model: foo_response.model,
            choices: vec![LlmChoice {
                index: 0,
                message: Some(LlmMessage {
                    role: foo_response.message.role,
                    content: Some(foo_response.message.text),
                    tool_calls: None,
                    function_call: None,
                    tool_call_id: None,
                }),
                delta: None,
                finish_reason: Some("stop".to_string()),
                tool_calls: None,
            }],
            usage: None,
        })
    }

    fn extract_tool_calls(&self, _response: &LlmCompletionResponse) -> Vec<LlmToolCall> {
        vec![]  // No tool calling support
    }

    fn provider_name(&self) -> &str {
        &self.config.provider
    }

    fn model_name(&self) -> &str {
        &self.config.model
    }
}
```

### A.3 Provider (`foo_provider.rs`)

```rust
use crate::llm_client::{
    create_llm_client, LlmClient, LlmModel, LlmProvider,
    ModelCapabilities, ProviderType,
};
use crate::models::LlmProviderConfig;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

pub struct FooAiProvider {
    config: LlmProviderConfig,
    models: HashMap<String, Arc<dyn LlmModel>>,
}

impl FooAiProvider {
    pub fn new(config: LlmProviderConfig) -> Result<Self> {
        let mut provider = Self {
            config,
            models: HashMap::new(),
        };
        provider.models.insert(
            "foo-1".to_string(),
            Arc::new(FooModel::new()),
        );
        Ok(provider)
    }
}

#[async_trait]
impl LlmProvider for FooAiProvider {
    fn id(&self) -> &str {
        "fooai"
    }

    fn name(&self) -> &str {
        "FooAI"
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Custom
    }

    fn supports_streaming(&self) -> bool {
        false
    }

    fn supports_tool_calling(&self) -> bool {
        false
    }

    fn supports_vision(&self) -> bool {
        false
    }

    async fn list_available_models(&self) -> Result<Vec<Arc<dyn LlmModel>>> {
        Ok(self.models.values().cloned().collect())
    }

    fn get_model(&self, model_id: &str) -> Option<Arc<dyn LlmModel>> {
        self.models.get(model_id).cloned()
    }

    async fn test_connection(&self) -> Result<()> {
        Ok(())
    }

    fn create_client(&self, model_id: &str) -> Result<Box<dyn LlmClient>> {
        let mut config = self.config.clone();
        config.model = model_id.to_string();
        create_llm_client(config)
    }
}

pub struct FooModel;

impl FooModel {
    pub fn new() -> Self {
        Self
    }
}

impl LlmModel for FooModel {
    fn id(&self) -> &str {
        "foo-1"
    }

    fn name(&self) -> &str {
        "Foo Model 1"
    }

    fn provider_id(&self) -> &str {
        "fooai"
    }

    fn context_length(&self) -> u32 {
        4096
    }

    fn max_output_tokens(&self) -> Option<u32> {
        Some(2048)
    }

    fn supports_streaming(&self) -> bool {
        false
    }

    fn supports_tool_calling(&self) -> bool {
        false
    }

    fn supports_vision(&self) -> bool {
        false
    }

    fn supports_reasoning(&self) -> bool {
        false
    }

    fn input_cost_per_token(&self) -> Option<f64> {
        Some(0.000001)
    }

    fn output_cost_per_token(&self) -> Option<f64> {
        Some(0.000002)
    }

    fn default_temperature(&self) -> Option<f32> {
        Some(0.7)
    }

    fn default_max_tokens(&self) -> Option<u32> {
        Some(1024)
    }

    fn estimate_tokens(&self, text: &str) -> u32 {
        (text.len() / 4) as u32
    }
}
```

### A.4 Registration

**In `llm_client.rs`**:
```rust
("fooai", _) => {
    let adapter = Box::new(adapters::FooAiAdapter::new(config.clone())?);
    Ok(Box::new(UnifiedLlmClient::new(adapter, config)?))
}
```

**In `config.rs`**:
```rust
pub struct ApiKeysConfig {
    // ...
    pub fooai_api_key: Option<String>,
}
```

**Test it**:
```bash
# Add to config
echo 'fooai_api_key = "your-key"' >> ~/.config/nocodo/manager.toml

# Run test
./run_llm_e2e_test.sh fooai foo-1
```

---

## Summary

Integrating a new LLM provider involves:

1. **Define types** - Request/response structs
2. **Implement adapter** - Convert to/from unified format
3. **Create provider** - High-level provider management
4. **Register in factory** - Wire everything together
5. **Add configuration** - API key and settings
6. **Write tests** - Unit, integration, E2E
7. **Run E2E test** - `./run_llm_e2e_test.sh provider model` ✅

The architecture is designed to make this process straightforward and maintainable. Each provider is isolated, making it easy to add, modify, or remove providers without affecting others.

**Key Success Factors**:
- Study existing implementations
- Follow the adapter pattern
- Test thoroughly at each step
- Document quirks and workarounds
- Use the E2E test as acceptance criteria

Good luck with your integration! 🚀
