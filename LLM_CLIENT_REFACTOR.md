# LLM Client Refactor: Adapter Pattern Implementation

## Executive Summary

This document outlines a refactoring plan to isolate model-specific LLM integration logic using the Adapter Pattern. The current architecture has become fragile due to shared code paths between different providers (OpenAI, GPT-5-codex, Claude, Grok), causing changes to one provider to break others.

**Goal**: Create a clean separation of concerns where each LLM provider/model has its own isolated adapter that handles API-specific quirks, while maintaining a unified interface for the rest of the application.

---

## Problem Statement

### Current Architecture Issues

1. **Tight Coupling**: The `OpenAiCompatibleClient` handles multiple distinct APIs:
   - Standard OpenAI Chat Completions (`v1/chat/completions`)
   - GPT-5-codex Responses API (`v1/responses`) - completely different format
   - Grok/xAI compatibility layer

2. **Cross-Contamination**: Recent changes for gpt-5-codex integration broke Claude integration:
   - Modified shared structures (`ContentItem`, `ResponsesApiRequest`)
   - Changed tool call extraction logic
   - Added model-specific routing (`if self.config.model == "gpt-5-codex"`)

3. **API Format Differences**:
   ```
   OpenAI Chat Completions:
   {
     "model": "gpt-4",
     "messages": [{"role": "user", "content": "..."}],
     "tools": [{"type": "function", "function": {...}}]
   }

   GPT-5-codex Responses API:
   {
     "model": "gpt-5-codex",
     "instructions": "...",  // System prompt here
     "input": [{"role": "user", "content": "..."}],
     "tools": [{"type": "function", "name": "...", "strict": true}]
   }

   Claude Messages API:
   {
     "model": "claude-3",
     "system": "...",  // Separate system field
     "messages": [{"role": "user", "content": [{"type": "text", "text": "..."}]}],
     "tools": [{"name": "...", "input_schema": {...}}]
   }
   ```

4. **Tool Call Format Differences**:
   - **OpenAI**: Tool calls in `message.tool_calls` array
   - **GPT-5-codex**: Tool calls as `ResponseItem::FunctionCall` in output array
   - **Claude**: Tool calls as `ContentBlock::ToolUse` in content array

5. **Violation of SOLID Principles**:
   - **Single Responsibility**: One client doing too much
   - **Open/Closed**: Hard to extend without modifying existing code

### Impact

- Maintenance burden increases with each new provider
- High risk of regression bugs
- Difficult to test in isolation
- Code complexity grows exponentially

---

## Solution: Adapter Pattern

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                      LlmAgent                                │
│                  (Application Layer)                         │
└─────────────────────┬───────────────────────────────────────┘
                      │ Uses
                      │ LlmClient trait
                      ▼
┌─────────────────────────────────────────────────────────────┐
│                  UnifiedLlmClient                            │
│              (Implements LlmClient trait)                    │
│                                                              │
│  • Delegates to appropriate adapter                         │
│  • Maintains unified interface                              │
│  • No model-specific logic                                  │
└─────────────────────┬───────────────────────────────────────┘
                      │ Delegates to
                      ▼
         ┌────────────────────────────────┐
         │     ProviderAdapter trait      │
         │                                │
         │  • convert_request()           │
         │  • send_request()              │
         │  • convert_response()          │
         │  • extract_tool_calls()        │
         └────────────┬───────────────────┘
                      │ Implemented by
         ┌────────────┴────────────┬──────────────┬──────────────┐
         ▼                         ▼              ▼              ▼
┌──────────────────┐   ┌──────────────────┐   ┌───────────┐  ┌───────┐
│ChatCompletions   │   │ ResponsesApi     │   │ Claude    │  │ Grok  │
│Adapter           │   │ Adapter          │   │ Messages  │  │Adapter│
│                  │   │                  │   │ Adapter   │  │       │
│• OpenAI std      │   │• GPT-5-codex     │   │• Anthropic│  │• xAI  │
│• GPT-4/3.5       │   │• Responses API   │   │• Claude   │  │       │
└──────────────────┘   └──────────────────┘   └───────────┘  └───────┘
```

### Key Components

#### 1. `ProviderAdapter` Trait

```rust
/// Adapter trait for provider-specific LLM API handling
#[async_trait]
pub trait ProviderAdapter: Send + Sync {
    /// Get the API endpoint URL for this provider
    fn get_api_url(&self) -> String;

    /// Check if this provider supports native tool calling
    fn supports_native_tools(&self) -> bool;

    /// Check if this provider supports legacy function calling
    fn supports_legacy_functions(&self) -> bool;

    /// Convert unified request to provider-specific format
    fn prepare_request(
        &self,
        request: LlmCompletionRequest,
    ) -> Result<Box<dyn ProviderRequest>>;

    /// Send request to provider and get raw response
    async fn send_request(
        &self,
        request: Box<dyn ProviderRequest>,
    ) -> Result<reqwest::Response>;

    /// Convert provider-specific response to unified format
    fn parse_response(
        &self,
        response_text: &str,
    ) -> Result<LlmCompletionResponse>;

    /// Extract tool calls from response
    fn extract_tool_calls(
        &self,
        response: &LlmCompletionResponse,
    ) -> Vec<LlmToolCall>;

    /// Get provider name for logging/debugging
    fn provider_name(&self) -> &str;

    /// Get model name for logging/debugging
    fn model_name(&self) -> &str;
}
```

#### 2. `ProviderRequest` Trait

```rust
/// Marker trait for provider-specific request types
pub trait ProviderRequest: Send + Sync {
    /// Serialize to JSON for HTTP request
    fn to_json(&self) -> Result<serde_json::Value>;

    /// Get any custom headers needed for this provider
    fn custom_headers(&self) -> Vec<(String, String)> {
        vec![]
    }
}
```

#### 3. `UnifiedLlmClient`

```rust
/// Unified LLM client that delegates to provider-specific adapters
pub struct UnifiedLlmClient {
    adapter: Box<dyn ProviderAdapter>,
    client: reqwest::Client,
    config: LlmProviderConfig,
}

#[async_trait]
impl LlmClient for UnifiedLlmClient {
    async fn complete(&self, request: LlmCompletionRequest) -> Result<LlmCompletionResponse> {
        // 1. Prepare provider-specific request
        let provider_request = self.adapter.prepare_request(request)?;

        // 2. Send request
        let response = self.adapter.send_request(provider_request).await?;

        // 3. Parse response
        let response_text = response.text().await?;
        let llm_response = self.adapter.parse_response(&response_text)?;

        Ok(llm_response)
    }

    fn extract_tool_calls_from_response(
        &self,
        response: &LlmCompletionResponse,
    ) -> Vec<LlmToolCall> {
        self.adapter.extract_tool_calls(response)
    }

    // ... other trait methods
}
```

---

## Implementation Plan

### Phase 1: Foundation Setup (2-3 hours)

**Objective**: Create the adapter infrastructure without breaking existing code.

#### Step 1.1: Create Adapter Module Structure

```
manager/src/llm_client/
├── mod.rs                      # Main module exports
├── traits.rs                   # LlmClient trait (existing)
├── unified_client.rs           # New UnifiedLlmClient
├── adapters/
│   ├── mod.rs                  # Adapter exports
│   ├── trait_adapter.rs        # ProviderAdapter trait definition
│   ├── chat_completions.rs     # OpenAI Chat Completions adapter
│   ├── responses_api.rs        # GPT-5-codex Responses API adapter
│   ├── claude_messages.rs      # Claude Messages API adapter
│   └── grok.rs                 # Grok/xAI adapter
└── types/
    ├── mod.rs                  # Type exports
    ├── requests.rs             # Provider-specific request types
    └── responses.rs            # Provider-specific response types
```

#### Step 1.2: Define Core Traits

**File**: `manager/src/llm_client/adapters/trait_adapter.rs`

```rust
use anyhow::Result;
use async_trait::async_trait;
use crate::llm_client::{LlmCompletionRequest, LlmCompletionResponse, LlmToolCall};

/// Trait for provider-specific request serialization
pub trait ProviderRequest: Send + Sync {
    fn to_json(&self) -> Result<serde_json::Value>;
    fn custom_headers(&self) -> Vec<(String, String)> {
        vec![]
    }
}

/// Adapter trait for provider-specific LLM API handling
#[async_trait]
pub trait ProviderAdapter: Send + Sync {
    /// Get the API endpoint URL
    fn get_api_url(&self) -> String;

    /// Provider capabilities
    fn supports_native_tools(&self) -> bool;
    fn supports_legacy_functions(&self) -> bool;
    fn supports_streaming(&self) -> bool;

    /// Request/response conversion
    fn prepare_request(&self, request: LlmCompletionRequest)
        -> Result<Box<dyn ProviderRequest>>;

    async fn send_request(&self, request: Box<dyn ProviderRequest>)
        -> Result<reqwest::Response>;

    fn parse_response(&self, response_text: &str)
        -> Result<LlmCompletionResponse>;

    /// Tool call extraction
    fn extract_tool_calls(&self, response: &LlmCompletionResponse)
        -> Vec<LlmToolCall>;

    /// Metadata
    fn provider_name(&self) -> &str;
    fn model_name(&self) -> &str;
}
```

#### Step 1.3: Create UnifiedLlmClient Skeleton

**File**: `manager/src/llm_client/unified_client.rs`

```rust
use anyhow::Result;
use async_trait::async_trait;
use crate::llm_client::{LlmClient, LlmCompletionRequest, LlmCompletionResponse, LlmToolCall};
use crate::llm_client::adapters::ProviderAdapter;
use crate::models::LlmProviderConfig;

pub struct UnifiedLlmClient {
    adapter: Box<dyn ProviderAdapter>,
    client: reqwest::Client,
    config: LlmProviderConfig,
}

impl UnifiedLlmClient {
    pub fn new(
        adapter: Box<dyn ProviderAdapter>,
        config: LlmProviderConfig,
    ) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()?;

        Ok(Self {
            adapter,
            client,
            config,
        })
    }
}

#[async_trait]
impl LlmClient for UnifiedLlmClient {
    async fn complete(&self, request: LlmCompletionRequest) -> Result<LlmCompletionResponse> {
        let start_time = std::time::Instant::now();

        tracing::info!(
            provider = %self.adapter.provider_name(),
            model = %self.adapter.model_name(),
            message_count = %request.messages.len(),
            "Sending request via adapter"
        );

        // Prepare provider-specific request
        let provider_request = self.adapter.prepare_request(request)?;

        // Send request
        let response = self.adapter.send_request(provider_request).await?;

        let response_time = start_time.elapsed();
        let status = response.status();

        if !status.is_success() {
            let error_text = response.text().await?;
            tracing::error!(
                provider = %self.adapter.provider_name(),
                status = %status,
                error = %error_text,
                "Request failed"
            );
            return Err(anyhow::anyhow!("API error: {} - {}", status, error_text));
        }

        // Parse response
        let response_text = response.text().await?;

        tracing::info!(
            provider = %self.adapter.provider_name(),
            response_time_ms = %response_time.as_millis(),
            response_length = %response_text.len(),
            "Received response"
        );

        let llm_response = self.adapter.parse_response(&response_text)?;

        Ok(llm_response)
    }

    fn stream_complete(
        &self,
        _request: LlmCompletionRequest,
    ) -> Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>> {
        // TODO: Implement streaming via adapter
        Box::pin(futures_util::stream::once(async {
            Err(anyhow::anyhow!("Streaming not yet implemented for unified client"))
        }))
    }

    fn extract_tool_calls_from_response(
        &self,
        response: &LlmCompletionResponse,
    ) -> Vec<LlmToolCall> {
        self.adapter.extract_tool_calls(response)
    }

    fn provider(&self) -> &str {
        self.adapter.provider_name()
    }

    fn model(&self) -> &str {
        self.adapter.model_name()
    }
}
```

---

### Phase 2: Implement Adapters (4-6 hours)

#### Step 2.1: ChatCompletionsAdapter (OpenAI Standard)

**File**: `manager/src/llm_client/adapters/chat_completions.rs`

Extract logic from existing `OpenAiCompatibleClient`:
- Standard OpenAI Chat Completions API
- Handles GPT-4, GPT-3.5-turbo, etc.
- Native tool calling support
- Legacy function calling support

```rust
pub struct ChatCompletionsAdapter {
    config: LlmProviderConfig,
    client: reqwest::Client,
}

#[async_trait]
impl ProviderAdapter for ChatCompletionsAdapter {
    fn get_api_url(&self) -> String {
        if let Some(base_url) = &self.config.base_url {
            format!("{}/v1/chat/completions", base_url.trim_end_matches('/'))
        } else {
            "https://api.openai.com/v1/chat/completions".to_string()
        }
    }

    fn supports_native_tools(&self) -> bool {
        // GPT-4 and newer models support native tools
        self.config.model.to_lowercase().starts_with("gpt-4") ||
        self.config.model.to_lowercase().contains("gpt-4")
    }

    fn prepare_request(&self, request: LlmCompletionRequest)
        -> Result<Box<dyn ProviderRequest>> {
        // Convert to OpenAI Chat Completions format
        let mut prepared = request;

        // Handle tool/function conversion based on model capabilities
        if !self.supports_native_tools() && prepared.tools.is_some() {
            // Convert to legacy functions
            prepared = self.convert_tools_to_legacy_functions(prepared);
        }

        Ok(Box::new(ChatCompletionsRequest {
            model: prepared.model,
            messages: prepared.messages,
            max_tokens: prepared.max_tokens,
            temperature: prepared.temperature,
            stream: prepared.stream,
            tools: prepared.tools,
            tool_choice: prepared.tool_choice,
            functions: prepared.functions,
            function_call: prepared.function_call,
        }))
    }

    async fn send_request(&self, request: Box<dyn ProviderRequest>)
        -> Result<reqwest::Response> {
        let json = request.to_json()?;

        let response = self.client
            .post(self.get_api_url())
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&json)
            .send()
            .await?;

        Ok(response)
    }

    fn parse_response(&self, response_text: &str)
        -> Result<LlmCompletionResponse> {
        let response: LlmCompletionResponse = serde_json::from_str(response_text)?;
        Ok(response)
    }

    fn extract_tool_calls(&self, response: &LlmCompletionResponse)
        -> Vec<LlmToolCall> {
        let mut tool_calls = Vec::new();

        for choice in &response.choices {
            if let Some(message) = &choice.message {
                // Message-level tool calls (OpenAI format)
                if let Some(message_tool_calls) = &message.tool_calls {
                    tool_calls.extend(message_tool_calls.clone());
                }

                // Legacy function calls
                if let Some(function_call) = &message.function_call {
                    tool_calls.push(LlmToolCall {
                        id: format!("legacy-{}", uuid::Uuid::new_v4()),
                        r#type: "function".to_string(),
                        function: LlmToolCallFunction {
                            name: function_call.name.clone(),
                            arguments: function_call.arguments.clone(),
                        },
                    });
                }
            }
        }

        tool_calls
    }

    fn provider_name(&self) -> &str {
        &self.config.provider
    }

    fn model_name(&self) -> &str {
        &self.config.model
    }
}
```

#### Step 2.2: ResponsesApiAdapter (GPT-5-codex)

**File**: `manager/src/llm_client/adapters/responses_api.rs`

Extract GPT-5-codex specific logic:
- Uses OpenAI Responses API (`v1/responses`)
- Different request/response format
- Custom instructions handling
- Function call output format

```rust
pub struct ResponsesApiAdapter {
    config: LlmProviderConfig,
    client: reqwest::Client,
}

impl ResponsesApiAdapter {
    fn get_codex_instructions(&self) -> String {
        // Return the Codex-specific system instructions
        // (moved from llm_client.rs:852-994)
        r#"You are Codex, based on GPT-5..."#.to_string()
    }

    fn convert_to_responses_format(&self, request: LlmCompletionRequest)
        -> Result<ResponsesApiRequest> {
        let instructions = self.get_codex_instructions();

        // Convert messages to input array
        let mut input = Vec::new();
        for message in &request.messages {
            match message.role.as_str() {
                "system" => continue, // Handled in instructions
                "user" => {
                    if let Some(content) = &message.content {
                        input.push(serde_json::json!({
                            "role": "user",
                            "content": content
                        }));
                    }
                }
                "assistant" => {
                    let mut msg_obj = serde_json::json!({"role": "assistant"});
                    if let Some(text) = &message.content {
                        msg_obj["content"] = serde_json::Value::String(text.clone());
                    }
                    if let Some(tool_calls) = &message.tool_calls {
                        // Convert tool calls for conversation history
                        msg_obj["tool_calls"] = serde_json::to_value(tool_calls)?;
                    }
                    input.push(msg_obj);
                }
                "tool" => {
                    if let Some(content) = &message.content {
                        input.push(serde_json::json!({
                            "role": "tool",
                            "content": content,
                            "tool_call_id": message.tool_call_id
                        }));
                    }
                }
                _ => {}
            }
        }

        // Convert tools
        let tools = request.tools.map(|tools| {
            tools.iter().map(|tool| ResponsesToolDefinition {
                r#type: tool.r#type.clone(),
                name: tool.function.name.clone(),
                description: tool.function.description.clone(),
                strict: true,
                parameters: tool.function.parameters.clone(),
            }).collect()
        });

        // Determine tool_choice
        let tool_choice = match request.tool_choice {
            Some(ToolChoice::None(_)) => "none",
            Some(ToolChoice::Auto(_)) => "auto",
            Some(ToolChoice::Required(_)) => "required",
            Some(ToolChoice::Specific { .. }) => "required",
            None => "auto",
        }.to_string();

        Ok(ResponsesApiRequest {
            model: request.model,
            instructions,
            input,
            tools,
            tool_choice,
            stream: request.stream.unwrap_or(false),
        })
    }

    fn convert_from_responses_format(&self, response: ResponsesApiResponse)
        -> Result<LlmCompletionResponse> {
        let mut content_text = String::new();
        let mut tool_calls = Vec::new();

        // Aggregate all output items
        for item in &response.output {
            match item {
                ResponseItem::Message { content, .. } => {
                    for content_item in content {
                        if let ContentItem::OutputText { text, .. } = content_item {
                            if !content_text.is_empty() {
                                content_text.push('\n');
                            }
                            content_text.push_str(text);
                        }
                    }
                }
                ResponseItem::FunctionCall { name, arguments, call_id, .. } => {
                    tool_calls.push(LlmToolCall {
                        id: call_id.clone(),
                        r#type: "function".to_string(),
                        function: LlmToolCallFunction {
                            name: name.clone(),
                            arguments: arguments.clone(),
                        },
                    });
                }
                ResponseItem::Reasoning { .. } => {
                    // Skip reasoning items
                }
            }
        }

        // Create unified response
        let choice = LlmChoice {
            index: 0,
            message: Some(LlmMessage {
                role: "assistant".to_string(),
                content: if content_text.is_empty() { None } else { Some(content_text) },
                tool_calls: if tool_calls.is_empty() { None } else { Some(tool_calls) },
                function_call: None,
                tool_call_id: None,
            }),
            delta: None,
            finish_reason: Some("stop".to_string()),
            tool_calls: None,
        };

        Ok(LlmCompletionResponse {
            id: response.id,
            object: "response".to_string(),
            created: 0,
            model: response.model,
            choices: vec![choice],
            usage: response.usage.map(|u| LlmUsage {
                prompt_tokens: u.input_tokens,
                completion_tokens: u.output_tokens,
                total_tokens: u.total_tokens,
            }),
        })
    }
}

#[async_trait]
impl ProviderAdapter for ResponsesApiAdapter {
    fn get_api_url(&self) -> String {
        if let Some(base_url) = &self.config.base_url {
            format!("{}/v1/responses", base_url.trim_end_matches('/'))
        } else {
            "https://api.openai.com/v1/responses".to_string()
        }
    }

    fn supports_native_tools(&self) -> bool {
        true // GPT-5-codex supports function calling
    }

    fn prepare_request(&self, request: LlmCompletionRequest)
        -> Result<Box<dyn ProviderRequest>> {
        let responses_request = self.convert_to_responses_format(request)?;
        Ok(Box::new(responses_request))
    }

    async fn send_request(&self, request: Box<dyn ProviderRequest>)
        -> Result<reqwest::Response> {
        let json = request.to_json()?;

        let response = self.client
            .post(self.get_api_url())
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .header("OpenAI-Beta", "responses=experimental")
            .json(&json)
            .send()
            .await?;

        Ok(response)
    }

    fn parse_response(&self, response_text: &str)
        -> Result<LlmCompletionResponse> {
        let responses_response: ResponsesApiResponse = serde_json::from_str(response_text)?;
        self.convert_from_responses_format(responses_response)
    }

    fn extract_tool_calls(&self, response: &LlmCompletionResponse)
        -> Vec<LlmToolCall> {
        // Tool calls are already in the message from conversion
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

#### Step 2.3: ClaudeMessagesAdapter

**File**: `manager/src/llm_client/adapters/claude_messages.rs`

Extract Claude-specific logic from existing `ClaudeClient`:
- Uses Anthropic Messages API
- Content blocks for tool calls
- Different tool definition format

```rust
pub struct ClaudeMessagesAdapter {
    config: LlmProviderConfig,
    client: reqwest::Client,
}

impl ClaudeMessagesAdapter {
    fn convert_to_claude_message(&self, message: &LlmMessage) -> ClaudeMessage {
        // Move existing logic from ClaudeClient::convert_to_claude_message
        // (from llm_client.rs:1682-1807)
        // ...
    }

    fn convert_request(&self, request: LlmCompletionRequest) -> ClaudeCompletionRequest {
        // Move existing logic from ClaudeClient::convert_request
        // (from llm_client.rs:1810-1876)
        // ...
    }

    fn convert_response(&self, response: ClaudeCompletionResponse) -> LlmCompletionResponse {
        // Move existing logic from ClaudeClient::convert_response
        // (from llm_client.rs:1879-1943)
        // ...
    }
}

#[async_trait]
impl ProviderAdapter for ClaudeMessagesAdapter {
    fn get_api_url(&self) -> String {
        if let Some(base_url) = &self.config.base_url {
            format!("{}/v1/messages", base_url.trim_end_matches('/'))
        } else {
            "https://api.anthropic.com/v1/messages".to_string()
        }
    }

    fn supports_native_tools(&self) -> bool {
        true // All Claude models support tools
    }

    fn prepare_request(&self, request: LlmCompletionRequest)
        -> Result<Box<dyn ProviderRequest>> {
        let claude_request = self.convert_request(request);
        Ok(Box::new(claude_request))
    }

    async fn send_request(&self, request: Box<dyn ProviderRequest>)
        -> Result<reqwest::Response> {
        let json = request.to_json()?;

        let response = self.client
            .post(self.get_api_url())
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&json)
            .send()
            .await?;

        Ok(response)
    }

    fn parse_response(&self, response_text: &str)
        -> Result<LlmCompletionResponse> {
        let claude_response: ClaudeCompletionResponse = serde_json::from_str(response_text)?;
        Ok(self.convert_response(claude_response))
    }

    fn extract_tool_calls(&self, response: &LlmCompletionResponse)
        -> Vec<LlmToolCall> {
        // Tool calls are already converted in convert_response
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

#### Step 2.4: GrokAdapter

**File**: `manager/src/llm_client/adapters/grok.rs`

```rust
pub struct GrokAdapter {
    config: LlmProviderConfig,
    client: reqwest::Client,
}

#[async_trait]
impl ProviderAdapter for GrokAdapter {
    fn get_api_url(&self) -> String {
        if let Some(base_url) = &self.config.base_url {
            format!("{}/v1/chat/completions", base_url.trim_end_matches('/'))
        } else {
            "https://api.x.ai/v1/chat/completions".to_string()
        }
    }

    fn supports_native_tools(&self) -> bool {
        // Grok Code Fast 1 and newer support tools
        self.config.model.to_lowercase().contains("grok-code-fast") ||
        self.config.model.to_lowercase().contains("grok-2") ||
        self.config.model.to_lowercase().contains("grok-3")
    }

    // Similar to ChatCompletionsAdapter but with xAI-specific handling
    // ...
}
```

---

### Phase 3: Integration and Migration (3-4 hours)

#### Step 3.1: Update Factory Function

**File**: `manager/src/llm_client/mod.rs`

```rust
use crate::models::LlmProviderConfig;
use anyhow::Result;

pub use unified_client::UnifiedLlmClient;
pub use traits::LlmClient;

mod adapters;
mod unified_client;
mod traits;
mod types;

/// Create an LLM client based on provider and model configuration
pub fn create_llm_client(config: LlmProviderConfig) -> Result<Box<dyn LlmClient>> {
    let adapter: Box<dyn adapters::ProviderAdapter> =
        match (config.provider.to_lowercase().as_str(), config.model.as_str()) {
            // GPT-5-codex requires Responses API
            ("openai", "gpt-5-codex") => {
                Box::new(adapters::ResponsesApiAdapter::new(config.clone())?)
            }

            // Standard OpenAI models use Chat Completions
            ("openai", _) => {
                Box::new(adapters::ChatCompletionsAdapter::new(config.clone())?)
            }

            // Anthropic/Claude use Messages API
            ("anthropic" | "claude", _) => {
                Box::new(adapters::ClaudeMessagesAdapter::new(config.clone())?)
            }

            // Grok/xAI use OpenAI-compatible API
            ("grok" | "xai", _) => {
                Box::new(adapters::GrokAdapter::new(config.clone())?)
            }

            // Unsupported provider
            _ => {
                return Err(anyhow::anyhow!(
                    "Unsupported provider/model combination: {} / {}",
                    config.provider,
                    config.model
                ));
            }
        };

    let client = UnifiedLlmClient::new(adapter, config)?;
    Ok(Box::new(client))
}

/// Legacy function for backward compatibility
/// Deprecated: Use create_llm_client instead
#[deprecated(note = "Use create_llm_client instead")]
pub fn create_llm_client_with_model(config: LlmProviderConfig) -> Result<Box<dyn LlmClient>> {
    create_llm_client(config)
}
```

#### Step 3.2: Maintain Backward Compatibility

Keep existing `OpenAiCompatibleClient` and `ClaudeClient` as deprecated wrappers:

```rust
/// Deprecated: Use create_llm_client with appropriate adapter instead
#[deprecated(note = "Use UnifiedLlmClient with adapters")]
pub struct OpenAiCompatibleClient {
    inner: UnifiedLlmClient,
}

impl OpenAiCompatibleClient {
    pub fn new(config: LlmProviderConfig) -> Result<Self> {
        let adapter = match config.model.as_str() {
            "gpt-5-codex" => Box::new(adapters::ResponsesApiAdapter::new(config.clone())?),
            _ => Box::new(adapters::ChatCompletionsAdapter::new(config.clone())?),
        };

        let inner = UnifiedLlmClient::new(adapter, config)?;
        Ok(Self { inner })
    }
}

#[async_trait]
impl LlmClient for OpenAiCompatibleClient {
    async fn complete(&self, request: LlmCompletionRequest) -> Result<LlmCompletionResponse> {
        self.inner.complete(request).await
    }

    // Delegate all other methods to inner...
}
```

---

### Phase 4: Testing and Validation (2-3 hours)

#### Step 4.1: Unit Tests for Each Adapter

**File**: `manager/src/llm_client/adapters/chat_completions.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prepare_request_with_native_tools() {
        let config = LlmProviderConfig {
            provider: "openai".to_string(),
            model: "gpt-4".to_string(),
            api_key: "test-key".to_string(),
            base_url: None,
            max_tokens: Some(1000),
            temperature: Some(0.7),
        };

        let adapter = ChatCompletionsAdapter::new(config).unwrap();

        let request = LlmCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![
                LlmMessage {
                    role: "user".to_string(),
                    content: Some("Test message".to_string()),
                    tool_calls: None,
                    function_call: None,
                    tool_call_id: None,
                }
            ],
            tools: Some(vec![/* test tools */]),
            tool_choice: Some(ToolChoice::Auto("auto".to_string())),
            max_tokens: Some(1000),
            temperature: Some(0.7),
            stream: Some(false),
            functions: None,
            function_call: None,
        };

        let prepared = adapter.prepare_request(request).unwrap();
        let json = prepared.to_json().unwrap();

        // Verify native tools are preserved
        assert!(json.get("tools").is_some());
        assert_eq!(json["model"], "gpt-4");
    }

    #[test]
    fn test_extract_tool_calls() {
        // Test tool call extraction logic
    }

    #[test]
    fn test_legacy_function_conversion() {
        // Test conversion from tools to legacy functions for GPT-3.5
    }
}
```

Similar test suites for:
- `responses_api.rs` - Test Responses API format conversion
- `claude_messages.rs` - Test Claude content block handling
- `grok.rs` - Test Grok compatibility

#### Step 4.2: Integration Tests

**File**: `manager/tests/llm_client_integration.rs`

```rust
use nocodo_manager::llm_client::{create_llm_client, LlmCompletionRequest, LlmMessage};
use nocodo_manager::models::LlmProviderConfig;

#[tokio::test]
async fn test_openai_gpt4_integration() {
    let config = LlmProviderConfig {
        provider: "openai".to_string(),
        model: "gpt-4".to_string(),
        api_key: std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set"),
        base_url: None,
        max_tokens: Some(100),
        temperature: Some(0.7),
    };

    let client = create_llm_client(config).unwrap();

    let request = LlmCompletionRequest {
        model: "gpt-4".to_string(),
        messages: vec![
            LlmMessage {
                role: "user".to_string(),
                content: Some("Say hello".to_string()),
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
}

#[tokio::test]
async fn test_gpt5_codex_integration() {
    // Test GPT-5-codex with Responses API
}

#[tokio::test]
async fn test_claude_integration() {
    // Test Claude with Messages API
}

#[tokio::test]
async fn test_tool_calling_isolation() {
    // Test that tool calls work correctly for each provider
}
```

#### Step 4.3: Regression Testing

Create a test suite that verifies:
1. GPT-4 tool calling still works
2. GPT-5-codex function calling works
3. Claude tool calling works (the currently broken feature)
4. Grok tool calling works
5. Legacy function calling works for older models

**File**: `manager/tests/regression.rs`

```rust
#[tokio::test]
async fn test_all_providers_tool_calling() {
    let providers = vec![
        ("openai", "gpt-4"),
        ("openai", "gpt-5-codex"),
        ("anthropic", "claude-3-sonnet-20240229"),
        ("grok", "grok-code-fast-1"),
    ];

    for (provider, model) in providers {
        let config = get_test_config(provider, model);
        let client = create_llm_client(config).unwrap();

        let request = create_tool_calling_request(model);
        let response = client.complete(request).await.unwrap();

        let tool_calls = client.extract_tool_calls_from_response(&response);

        assert!(
            !tool_calls.is_empty(),
            "Provider {} model {} should support tool calling",
            provider,
            model
        );
    }
}
```

---

### Phase 5: Cleanup and Documentation (1-2 hours)

#### Step 5.1: Deprecate Old Clients

Add deprecation notices:

```rust
#[deprecated(
    since = "0.2.0",
    note = "Use create_llm_client which automatically selects the correct adapter"
)]
pub struct OpenAiCompatibleClient {
    // Keep implementation for backward compatibility
}

#[deprecated(
    since = "0.2.0",
    note = "Use create_llm_client which automatically selects the correct adapter"
)]
pub struct ClaudeClient {
    // Keep implementation for backward compatibility
}
```

#### Step 5.2: Update Internal Documentation

**File**: `manager/src/llm_client/README.md`

```markdown
# LLM Client Architecture

## Overview

The LLM client uses an Adapter Pattern to isolate provider-specific API implementations.

## Architecture

```
LlmAgent → create_llm_client() → UnifiedLlmClient → ProviderAdapter
                                                           ↓
                                        ┌──────────────────┴──────────────────┐
                                        ↓                 ↓                   ↓
                              ChatCompletionsAdapter  ResponsesApiAdapter  ClaudeAdapter
```

## Adding a New Provider

1. Create a new adapter in `adapters/your_provider.rs`
2. Implement the `ProviderAdapter` trait
3. Add provider selection logic to `create_llm_client()`
4. Add tests

## Provider-Specific Details

### OpenAI Chat Completions
- **Adapter**: `ChatCompletionsAdapter`
- **Endpoint**: `/v1/chat/completions`
- **Models**: gpt-4, gpt-3.5-turbo
- **Tool Calling**: Native tools + legacy functions

### OpenAI Responses API
- **Adapter**: `ResponsesApiAdapter`
- **Endpoint**: `/v1/responses`
- **Models**: gpt-5-codex
- **Tool Calling**: Native function calling

### Anthropic Claude
- **Adapter**: `ClaudeMessagesAdapter`
- **Endpoint**: `/v1/messages`
- **Models**: claude-3-*
- **Tool Calling**: Content blocks with tool_use

### Grok/xAI
- **Adapter**: `GrokAdapter`
- **Endpoint**: `/v1/chat/completions`
- **Models**: grok-code-fast-1, grok-2
- **Tool Calling**: OpenAI-compatible
```

---

## Testing Strategy

### Test Matrix

| Provider | Model | Tool Calling | Streaming | Legacy Functions |
|----------|-------|--------------|-----------|------------------|
| OpenAI | gpt-4 | ✅ | ✅ | ✅ |
| OpenAI | gpt-3.5-turbo | ✅ | ✅ | ✅ |
| OpenAI | gpt-5-codex | ✅ | ⚠️ | ❌ |
| Anthropic | claude-3-sonnet | ✅ | ⚠️ | ❌ |
| Anthropic | claude-3-opus | ✅ | ⚠️ | ❌ |
| Grok | grok-code-fast-1 | ✅ | ✅ | ❌ |

Legend:
- ✅ Fully supported
- ⚠️ Not yet implemented (streaming via adapters)
- ❌ Not supported by provider

### Test Coverage Goals

- **Unit Tests**: 80%+ coverage for each adapter
- **Integration Tests**: All providers tested with real API calls
- **Regression Tests**: Ensure existing functionality isn't broken

---

## Migration Checklist

### Before Starting
- [ ] Review current `llm_client.rs` implementation
- [ ] Identify all model-specific code paths
- [ ] Create feature branch: `refactor/adapter-pattern`
- [ ] Set up test environment with API keys

### Phase 1: Foundation (Day 1)
- [ ] Create adapter module structure
- [ ] Define `ProviderAdapter` trait
- [ ] Define `ProviderRequest` trait
- [ ] Create `UnifiedLlmClient` skeleton
- [ ] Update module exports

### Phase 2: Adapters (Day 2-3)
- [ ] Implement `ChatCompletionsAdapter`
  - [ ] Extract OpenAI standard logic
  - [ ] Add unit tests
- [ ] Implement `ResponsesApiAdapter`
  - [ ] Extract gpt-5-codex logic
  - [ ] Add unit tests
- [ ] Implement `ClaudeMessagesAdapter`
  - [ ] Extract Claude logic
  - [ ] Add unit tests
  - [ ] Verify tool calling works
- [ ] Implement `GrokAdapter`
  - [ ] Extract Grok logic
  - [ ] Add unit tests

### Phase 3: Integration (Day 4)
- [ ] Update `create_llm_client()` factory function
- [ ] Add adapter selection logic
- [ ] Create backward compatibility wrappers
- [ ] Update `llm_agent.rs` to use new factory
- [ ] Run integration tests

### Phase 4: Testing (Day 5)
- [ ] Write unit tests for all adapters
- [ ] Write integration tests
- [ ] Run regression tests
- [ ] Manual testing with real APIs:
  - [ ] Test GPT-4 with tools
  - [ ] Test GPT-5-codex with tools
  - [ ] Test Claude with tools (verify fix)
  - [ ] Test Grok with tools
- [ ] Load testing (optional)

### Phase 5: Cleanup (Day 6)
- [ ] Add deprecation warnings
- [ ] Update documentation
- [ ] Code review
- [ ] Merge to main branch
- [ ] Monitor production logs

---

## Benefits of This Approach

### 1. **Isolation**
- Changes to one provider don't affect others
- Each adapter is self-contained and testable
- Clear boundaries between implementations

### 2. **Maintainability**
- Easy to understand which code handles which provider
- Simple to add new providers
- Reduced cognitive load when debugging

### 3. **Testability**
- Mock adapters for unit tests
- Test each provider in isolation
- Clear test boundaries

### 4. **Flexibility**
- Easy to swap implementations
- Can add new adapters without touching existing code
- Feature flags per provider

### 5. **Type Safety**
- Provider-specific types don't leak
- Compiler catches interface mismatches
- Cleaner error messages

---

## Risks and Mitigations

### Risk 1: Breaking Existing Code
**Mitigation**:
- Keep old clients as deprecated wrappers
- Comprehensive regression testing
- Gradual rollout with feature flags

### Risk 2: Performance Overhead
**Mitigation**:
- Trait objects have minimal overhead
- No additional allocations in hot paths
- Benchmark before/after

### Risk 3: Incomplete Migration
**Mitigation**:
- Clear migration checklist
- Document all steps
- Code review before merge

### Risk 4: Streaming Support
**Mitigation**:
- Phase streaming implementation separately
- Keep existing streaming code for now
- Migrate streaming after core refactor is stable

---

## Future Enhancements

### 1. Streaming Support via Adapters
Add streaming methods to `ProviderAdapter` trait:
```rust
async fn stream_complete(
    &self,
    request: LlmCompletionRequest,
) -> Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>>;
```

### 2. Provider-Specific Optimizations
- Prompt caching (Claude)
- Batch requests (OpenAI)
- Custom retry logic per provider

### 3. Model Registry
Create a registry of model capabilities:
```rust
pub struct ModelRegistry {
    models: HashMap<String, ModelCapabilities>,
}
```

### 4. Adapter Composition
Allow composing adapters for cross-cutting concerns:
```rust
pub struct LoggingAdapter<T: ProviderAdapter> {
    inner: T,
}

pub struct RetryAdapter<T: ProviderAdapter> {
    inner: T,
    max_retries: u32,
}
```

---

## References

- **Design Patterns**: Gang of Four - Adapter Pattern
- **Rust Traits**: https://doc.rust-lang.org/book/ch10-02-traits.html
- **OpenAI API Docs**: https://platform.openai.com/docs/api-reference
- **Anthropic API Docs**: https://docs.anthropic.com/claude/reference
- **xAI API Docs**: https://docs.x.ai/api

---

## Appendix: Code Examples

### Example: Adding a New Provider

```rust
// 1. Create adapter file: manager/src/llm_client/adapters/google_gemini.rs

pub struct GeminiAdapter {
    config: LlmProviderConfig,
    client: reqwest::Client,
}

impl GeminiAdapter {
    pub fn new(config: LlmProviderConfig) -> Result<Self> {
        let client = reqwest::Client::new();
        Ok(Self { config, client })
    }
}

#[async_trait]
impl ProviderAdapter for GeminiAdapter {
    fn get_api_url(&self) -> String {
        format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
            self.config.model
        )
    }

    fn supports_native_tools(&self) -> bool {
        true
    }

    fn prepare_request(&self, request: LlmCompletionRequest)
        -> Result<Box<dyn ProviderRequest>> {
        // Convert to Gemini format
        let gemini_request = GeminiRequest {
            contents: convert_messages(request.messages),
            generation_config: Some(GeminiGenerationConfig {
                temperature: request.temperature,
                max_output_tokens: request.max_tokens,
            }),
            tools: convert_tools(request.tools),
        };
        Ok(Box::new(gemini_request))
    }

    // ... implement other trait methods
}

// 2. Update factory in mod.rs
match config.provider.to_lowercase().as_str() {
    "google" | "gemini" => {
        Box::new(adapters::GeminiAdapter::new(config.clone())?)
    }
    // ... other providers
}

// 3. Add tests
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_gemini_integration() {
        // Test implementation
    }
}
```

---

## Success Criteria

The refactor is considered successful when:

1. ✅ All existing tests pass
2. ✅ Claude tool calling works correctly (bug fixed)
3. ✅ GPT-5-codex tool calling continues to work
4. ✅ No regression in other providers
5. ✅ Code coverage >= 80% for new adapters
6. ✅ Documentation is complete
7. ✅ Performance benchmarks show < 5% overhead
8. ✅ Production deployment without incidents

---

## Timeline

**Total Estimated Time**: 12-18 hours over 6 days

- **Day 1** (2-3 hours): Foundation setup
- **Day 2-3** (4-6 hours): Implement adapters
- **Day 4** (3-4 hours): Integration and migration
- **Day 5** (2-3 hours): Testing and validation
- **Day 6** (1-2 hours): Cleanup and documentation

**Contingency**: +4 hours for unexpected issues

---

## Questions and Answers

### Q: Why not use feature flags instead?
**A**: Feature flags control which features are enabled, but don't solve the architectural problem of tangled code. Adapters provide clean separation regardless of which features are enabled.

### Q: What about backwards compatibility?
**A**: We maintain backward compatibility by keeping the old clients as thin wrappers around the new adapter-based implementation. This allows gradual migration.

### Q: How do we handle streaming?
**A**: Initially, streaming will use the existing implementation. In a follow-up phase, we'll add streaming support to the adapter trait.

### Q: What about error handling?
**A**: Each adapter can implement provider-specific error handling and conversion to our unified error type.

### Q: How do we test without API keys?
**A**: We'll use a combination of unit tests with mocked HTTP responses and integration tests that require API keys (run in CI with secrets).

---

**Document Version**: 1.0
**Last Updated**: 2025-01-05
**Author**: AI Assistant
**Status**: Draft - Awaiting Review
