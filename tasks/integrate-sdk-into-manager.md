# Migrate Manager LLM Client to nocodo-llm-sdk

## Overview

Replace manager's custom LLM client implementation with the nocodo-llm-sdk. The SDK provides all required functionality: Claude, OpenAI (including Responses API for gpt-5-codex), Grok, and GLM support with tool calling.

## Current State

**Manager has:**
- `manager/src/llm_client.rs` - Custom LLM client with trait and implementations
- `manager/src/llm_client/adapters/` - Provider-specific adapters (Claude, Responses API, GLM)
- `manager/src/llm_providers/` - Provider metadata
- `manager/src/llm_agent.rs` - Agent that uses the LLM client

**SDK provides:**
- All providers (Claude, OpenAI, Grok, GLM) with unified interface
- OpenAI Responses API support for gpt-5.1-codex
- Tool calling with schemars integration
- Builder pattern for requests
- Multi-provider support (xAI and Zen for Grok/GLM)

## Migration Steps

### Step 1: Add SDK Dependency

**File:** `manager/Cargo.toml`

```toml
[dependencies]
nocodo-llm-sdk = { path = "../nocodo-llm-sdk" }
```

### Step 2: Create SDK Wrapper Module

Create a new file that wraps the SDK and implements manager's `LlmClient` trait.

**File:** `manager/src/llm_sdk_wrapper.rs`

```rust
use anyhow::Result;
use async_trait::async_trait;
use futures_util::Stream;
use std::pin::Pin;

use nocodo_llm_sdk::{
    claude::{client::ClaudeClient, types::ClaudeContentBlock},
    glm::cerebras::CerebrasGlmClient,
    grok::xai::XaiGrokClient,
    openai::client::OpenAIClient,
};

use crate::llm_client::{
    LlmClient, LlmCompletionRequest, LlmCompletionResponse, LlmChoice, LlmMessage, LlmToolCall,
    LlmToolCallFunction, LlmUsage, StreamChunk,
};
use crate::models::LlmProviderConfig;

pub struct SdkLlmClient {
    provider: String,
    model: String,
    inner: ClientType,
}

enum ClientType {
    OpenAI(OpenAIClient),
    Claude(ClaudeClient),
    Grok(XaiGrokClient),
    Glm(CerebrasGlmClient),
}

impl SdkLlmClient {
    pub fn new(config: &LlmProviderConfig) -> Result<Self> {
        let provider = config.provider.clone();
        let model = config.model.clone();

        let inner = match config.provider.to_lowercase().as_str() {
            "openai" => {
                let mut client = OpenAIClient::new(&config.api_key)?;
                if let Some(base_url) = &config.base_url {
                    client = client.with_base_url(base_url);
                }
                ClientType::OpenAI(client)
            }
            "anthropic" | "claude" => {
                let client = ClaudeClient::new(&config.api_key)?;
                ClientType::Claude(client)
            }
            "grok" | "xai" => {
                let client = XaiGrokClient::new(&config.api_key)?;
                ClientType::Grok(client)
            }
            "zai" | "glm" => {
                let client = CerebrasGlmClient::new(&config.api_key)?;
                ClientType::Glm(client)
            }
            _ => anyhow::bail!("Unsupported provider: {}", config.provider),
        };

        Ok(Self {
            provider,
            model,
            inner,
        })
    }
}

#[async_trait]
impl LlmClient for SdkLlmClient {
    async fn complete(&self, request: LlmCompletionRequest) -> Result<LlmCompletionResponse> {
        match &self.inner {
            ClientType::OpenAI(client) => {
                // Build OpenAI request using SDK builder
                let mut builder = client.message_builder().model(&request.model);

                // Add max_tokens
                if let Some(max_tokens) = request.max_tokens {
                    builder = builder.max_completion_tokens(max_tokens);
                }

                // Add temperature
                if let Some(temp) = request.temperature {
                    builder = builder.temperature(temp);
                }

                // Add messages
                for msg in &request.messages {
                    match msg.role.as_str() {
                        "system" => {
                            if let Some(content) = &msg.content {
                                builder = builder.system_message(content);
                            }
                        }
                        "user" => {
                            if let Some(content) = &msg.content {
                                builder = builder.user_message(content);
                            }
                        }
                        "assistant" => {
                            if let Some(content) = &msg.content {
                                builder = builder.assistant_message(content);
                            }
                            // TODO: Handle tool calls in assistant messages
                        }
                        "tool" => {
                            // TODO: Handle tool result messages
                        }
                        _ => {}
                    }
                }

                // Add tools if present
                if let Some(tools) = &request.tools {
                    for tool in tools {
                        // TODO: Convert manager tool format to SDK format
                        // Use builder.add_tool() method
                    }
                }

                // Send request
                let response = builder.send().await.map_err(|e| anyhow::anyhow!("{}", e))?;

                // Convert SDK response to manager format
                Ok(LlmCompletionResponse {
                    id: response.id,
                    model: response.model,
                    choices: response
                        .choices
                        .into_iter()
                        .enumerate()
                        .map(|(idx, choice)| LlmChoice {
                            index: idx as u32,
                            message: Some(LlmMessage {
                                role: choice.message.role.to_string(),
                                content: Some(choice.message.content),
                                tool_calls: choice.message.tool_calls.map(|tcs| {
                                    tcs.into_iter()
                                        .map(|tc| LlmToolCall {
                                            id: tc.id,
                                            r#type: tc.r#type,
                                            function: LlmToolCallFunction {
                                                name: tc.function.name,
                                                arguments: tc.function.arguments,
                                            },
                                        })
                                        .collect()
                                }),
                                function_call: None,
                                tool_call_id: None,
                            }),
                            finish_reason: choice.finish_reason,
                        })
                        .collect(),
                    usage: response.usage.map(|u| LlmUsage {
                        prompt_tokens: u.prompt_tokens,
                        completion_tokens: u.completion_tokens,
                        total_tokens: u.total_tokens,
                        reasoning_tokens: None,
                    }),
                })
            }

            ClientType::Claude(client) => {
                // Build Claude request using SDK builder
                let mut builder = client.message_builder().model(&request.model);

                // Add max_tokens (required for Claude)
                let max_tokens = request.max_tokens.unwrap_or(1024);
                builder = builder.max_tokens(max_tokens);

                // Add temperature
                if let Some(temp) = request.temperature {
                    builder = builder.temperature(temp);
                }

                // Add messages (Claude doesn't support system in messages array)
                for msg in &request.messages {
                    match msg.role.as_str() {
                        "system" => {
                            if let Some(content) = &msg.content {
                                builder = builder.system(content);
                            }
                        }
                        "user" => {
                            if let Some(content) = &msg.content {
                                builder = builder.user_message(content);
                            }
                        }
                        "assistant" => {
                            if let Some(content) = &msg.content {
                                builder = builder.assistant_message(content);
                            }
                        }
                        _ => {}
                    }
                }

                // Add tools if present
                if let Some(tools) = &request.tools {
                    for tool in tools {
                        // TODO: Convert manager tool format to SDK format
                    }
                }

                // Send request
                let response = builder.send().await.map_err(|e| anyhow::anyhow!("{}", e))?;

                // Convert SDK response to manager format
                let content_text = response
                    .content
                    .iter()
                    .filter_map(|block| match block {
                        ClaudeContentBlock::Text { text } => Some(text.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");

                Ok(LlmCompletionResponse {
                    id: response.id,
                    model: response.model,
                    choices: vec![LlmChoice {
                        index: 0,
                        message: Some(LlmMessage {
                            role: "assistant".to_string(),
                            content: Some(content_text),
                            tool_calls: None, // TODO: Extract tool uses from content blocks
                            function_call: None,
                            tool_call_id: None,
                        }),
                        finish_reason: Some(response.stop_reason),
                    }],
                    usage: Some(LlmUsage {
                        prompt_tokens: response.usage.input_tokens,
                        completion_tokens: response.usage.output_tokens,
                        total_tokens: response.usage.input_tokens + response.usage.output_tokens,
                        reasoning_tokens: None,
                    }),
                })
            }

            ClientType::Grok(client) => {
                // Similar to OpenAI implementation
                // Use client.message_builder() pattern
                let mut builder = client.message_builder().model(&request.model);

                if let Some(max_tokens) = request.max_tokens {
                    builder = builder.max_tokens(max_tokens);
                }

                if let Some(temp) = request.temperature {
                    builder = builder.temperature(temp);
                }

                for msg in &request.messages {
                    match msg.role.as_str() {
                        "system" => {
                            if let Some(content) = &msg.content {
                                builder = builder.system_message(content);
                            }
                        }
                        "user" => {
                            if let Some(content) = &msg.content {
                                builder = builder.user_message(content);
                            }
                        }
                        "assistant" => {
                            if let Some(content) = &msg.content {
                                builder = builder.assistant_message(content);
                            }
                        }
                        _ => {}
                    }
                }

                let response = builder.send().await.map_err(|e| anyhow::anyhow!("{}", e))?;

                // Convert response (similar to OpenAI)
                Ok(LlmCompletionResponse {
                    id: response.id,
                    model: response.model,
                    choices: response
                        .choices
                        .into_iter()
                        .enumerate()
                        .map(|(idx, choice)| LlmChoice {
                            index: idx as u32,
                            message: Some(LlmMessage {
                                role: choice.message.role.to_string(),
                                content: Some(choice.message.content),
                                tool_calls: None, // TODO: Handle tool calls
                                function_call: None,
                                tool_call_id: None,
                            }),
                            finish_reason: choice.finish_reason,
                        })
                        .collect(),
                    usage: response.usage.map(|u| LlmUsage {
                        prompt_tokens: u.prompt_tokens,
                        completion_tokens: u.completion_tokens,
                        total_tokens: u.total_tokens,
                        reasoning_tokens: None,
                    }),
                })
            }

            ClientType::Glm(client) => {
                // Similar to OpenAI/Grok implementation
                let mut builder = client.message_builder().model(&request.model);

                if let Some(max_tokens) = request.max_tokens {
                    builder = builder.max_tokens(max_tokens);
                }

                if let Some(temp) = request.temperature {
                    builder = builder.temperature(temp);
                }

                for msg in &request.messages {
                    match msg.role.as_str() {
                        "system" => {
                            if let Some(content) = &msg.content {
                                builder = builder.system_message(content);
                            }
                        }
                        "user" => {
                            if let Some(content) = &msg.content {
                                builder = builder.user_message(content);
                            }
                        }
                        "assistant" => {
                            if let Some(content) = &msg.content {
                                builder = builder.assistant_message(content);
                            }
                        }
                        _ => {}
                    }
                }

                let response = builder.send().await.map_err(|e| anyhow::anyhow!("{}", e))?;

                // Convert response
                Ok(LlmCompletionResponse {
                    id: response.id,
                    model: response.model,
                    choices: response
                        .choices
                        .into_iter()
                        .enumerate()
                        .map(|(idx, choice)| LlmChoice {
                            index: idx as u32,
                            message: Some(LlmMessage {
                                role: choice.message.role.to_string(),
                                content: choice.message.get_content(),
                                tool_calls: None, // TODO: Handle tool calls
                                function_call: None,
                                tool_call_id: None,
                            }),
                            finish_reason: choice.finish_reason,
                        })
                        .collect(),
                    usage: response.usage.map(|u| LlmUsage {
                        prompt_tokens: u.prompt_tokens,
                        completion_tokens: u.completion_tokens,
                        total_tokens: u.total_tokens,
                        reasoning_tokens: None,
                    }),
                })
            }
        }
    }

    fn stream_complete(
        &self,
        _request: LlmCompletionRequest,
    ) -> Pin<Box<dyn Stream<Item = Result<StreamChunk>> + Send>> {
        // Streaming not implemented yet - return error stream
        Box::pin(futures_util::stream::once(async {
            Err(anyhow::anyhow!("Streaming not yet implemented in SDK wrapper"))
        }))
    }

    fn extract_tool_calls_from_response(
        &self,
        response: &LlmCompletionResponse,
    ) -> Vec<LlmToolCall> {
        response
            .choices
            .first()
            .and_then(|c| c.message.as_ref())
            .and_then(|m| m.tool_calls.clone())
            .unwrap_or_default()
    }

    fn provider(&self) -> &str {
        &self.provider
    }

    fn model(&self) -> &str {
        &self.model
    }
}
```

### Step 3: Update Factory Function

Update the `create_llm_client` function to optionally use the SDK wrapper.

**File:** `manager/src/llm_client.rs`

Add this function at the end of the file:

```rust
/// Create an LLM client using the SDK (new implementation)
#[cfg(feature = "use-llm-sdk")]
pub fn create_llm_client(config: LlmProviderConfig) -> Result<Box<dyn LlmClient>> {
    use crate::llm_sdk_wrapper::SdkLlmClient;
    Ok(Box::new(SdkLlmClient::new(&config)?))
}

/// Create an LLM client using legacy implementation (old)
#[cfg(not(feature = "use-llm-sdk"))]
pub fn create_llm_client(config: LlmProviderConfig) -> Result<Box<dyn LlmClient>> {
    // Keep existing implementation
    use crate::llm_client::unified_client::UnifiedLlmClient;
    Ok(Box::new(UnifiedLlmClient::new(config)?))
}
```

### Step 4: Add Feature Flag

**File:** `manager/Cargo.toml`

```toml
[features]
default = []
use-llm-sdk = ["nocodo-llm-sdk"]
```

### Step 5: Update Module Declarations

**File:** `manager/src/lib.rs`

```rust
pub mod llm_client;

#[cfg(feature = "use-llm-sdk")]
pub mod llm_sdk_wrapper;
```

### Step 6: Test the Migration

Test with the SDK enabled:

```bash
# Build with SDK
cargo build --features use-llm-sdk

# Run tests with SDK
cargo test --features use-llm-sdk

# Run integration tests
cargo test --test llm_e2e_simple --features use-llm-sdk -- --ignored
```

Test without SDK (legacy):

```bash
# Build without SDK (default)
cargo build

# Run tests without SDK
cargo test
```

### Step 7: Handle Tool Calling

The SDK provides tool calling support. Update the wrapper to convert between formats:

**Add to `manager/src/llm_sdk_wrapper.rs`:**

```rust
use nocodo_llm_sdk::tools::{Tool, ToolChoice as SdkToolChoice};
use schemars::JsonSchema;

// Helper function to convert manager tools to SDK tools
fn convert_tools_to_sdk(tools: &[crate::llm_client::ToolDefinition]) -> Vec<Tool> {
    tools
        .iter()
        .map(|tool| {
            // Extract function definition
            let func = &tool.function;

            // Create a struct that can be used with schemars
            // This is a simplified example - you'll need to properly convert
            // the JSON schema from manager format to a Rust type with JsonSchema

            // For now, we can use the raw serde_json::Value
            // In the future, you might want to define proper structs
            Tool::new(
                func.name.clone(),
                func.description.clone(),
                func.parameters.clone(), // Pass through the JSON schema
            )
        })
        .collect()
}

// Update the complete() method in each match arm to add tools:
// if let Some(tools) = &request.tools {
//     for tool in convert_tools_to_sdk(tools) {
//         builder = builder.add_tool(tool);
//     }
// }
```

### Step 8: Handle OpenAI Responses API (gpt-5-codex)

The SDK already supports the Responses API. Update the OpenAI client handling:

```rust
// In the OpenAI match arm of complete():
ClientType::OpenAI(client) => {
    // Check if this is a gpt-5-codex model (uses Responses API)
    if request.model.contains("gpt-5") && request.model.contains("codex") {
        // Use response_builder() instead of message_builder()
        let mut builder = client.response_builder().model(&request.model);

        // Responses API uses "input" instead of messages
        let input_text = request
            .messages
            .iter()
            .filter_map(|m| m.content.as_ref())
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");

        builder = builder.input(&input_text);

        if let Some(max_tokens) = request.max_tokens {
            builder = builder.max_output_tokens(max_tokens);
        }

        let response = builder.send().await.map_err(|e| anyhow::anyhow!("{}", e))?;

        // Convert Responses API response to manager format
        // Extract text from output items
        let content = response
            .output
            .iter()
            .filter(|item| item.item_type == "message")
            .filter_map(|item| item.content.as_ref())
            .flat_map(|blocks| blocks.iter())
            .filter(|block| block.content_type == "output_text")
            .map(|block| block.text.clone())
            .collect::<Vec<_>>()
            .join("\n");

        return Ok(LlmCompletionResponse {
            id: response.id,
            model: request.model.clone(),
            choices: vec![LlmChoice {
                index: 0,
                message: Some(LlmMessage {
                    role: "assistant".to_string(),
                    content: Some(content),
                    tool_calls: None,
                    function_call: None,
                    tool_call_id: None,
                }),
                finish_reason: Some("stop".to_string()),
            }],
            usage: response.usage.map(|u| LlmUsage {
                prompt_tokens: u.input_tokens,
                completion_tokens: u.output_tokens,
                total_tokens: u.input_tokens + u.output_tokens,
                reasoning_tokens: None,
            }),
        });
    }

    // Regular chat completions (existing code)
    // ...
}
```

## Testing Strategy

### Unit Tests

Create unit tests for the SDK wrapper:

**File:** `manager/src/llm_sdk_wrapper.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation_openai() {
        let config = LlmProviderConfig {
            provider: "openai".to_string(),
            model: "gpt-4o".to_string(),
            api_key: "test-key".to_string(),
            base_url: None,
            max_tokens: None,
            temperature: None,
        };

        let client = SdkLlmClient::new(&config);
        assert!(client.is_ok());
    }

    #[test]
    fn test_client_creation_claude() {
        let config = LlmProviderConfig {
            provider: "claude".to_string(),
            model: "claude-sonnet-4-5-20250929".to_string(),
            api_key: "test-key".to_string(),
            base_url: None,
            max_tokens: None,
            temperature: None,
        };

        let client = SdkLlmClient::new(&config);
        assert!(client.is_ok());
    }
}
```

### Integration Tests

Use existing manager integration tests with the feature flag:

```bash
# Test with SDK
cargo test --features use-llm-sdk --test llm_e2e_simple -- --ignored

# Test comparison (old vs new)
cargo test --features use-llm-sdk -- test_sdk_vs_legacy
```

## Cleanup After Validation

Once the SDK wrapper is working and tested, you can remove the old implementation:

### Files to Remove

```bash
rm -rf manager/src/llm_client/adapters/
rm -rf manager/src/llm_providers/
rm manager/src/llm_client/unified_client.rs
# Keep llm_client.rs but remove implementations, keep only trait definition
```

### Update llm_client.rs

After removing old code, `manager/src/llm_client.rs` should only contain:
- Type definitions (LlmMessage, LlmCompletionRequest, etc.)
- The LlmClient trait definition
- The create_llm_client factory (now always using SDK)

Move the SDK wrapper to be the main implementation:

```bash
mv manager/src/llm_sdk_wrapper.rs manager/src/llm_client_impl.rs
```

Update `lib.rs`:

```rust
pub mod llm_client;
mod llm_client_impl;  // Private implementation using SDK
```

## Notes

- The SDK uses different error types (`LlmError`) vs manager's `anyhow::Result`. The wrapper converts between them.
- The SDK's builder pattern is more ergonomic than manager's request structs.
- OpenAI Responses API is already supported in the SDK for gpt-5-codex models.
- Streaming is not used in manager (LlmAgent explicitly sets `stream: Some(false)`) so it's okay that the SDK wrapper doesn't implement it yet.
- Tool calling conversion needs proper schema mapping between manager and SDK formats.
