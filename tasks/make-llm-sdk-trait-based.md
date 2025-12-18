# Make nocodo-llm-sdk Trait-Based and Remove Manager Abstraction Layer

**Status**: 🔄 In Progress
**Priority**: High
**Created**: 2024-12-18

## Summary

Refactor nocodo-llm-sdk to implement the `LlmClient` trait across all provider clients, making it a truly provider-agnostic library. Remove the manager's custom `LlmClient` trait and wrapper types, allowing direct usage of SDK types.

## Problem Statement

Currently we have two separate `LlmClient` traits and duplicate type systems:

**In nocodo-llm-sdk (src/client.rs):**
- `LlmClient` trait defined but **not implemented** by any client
- Each provider has its own client type (OpenAIClient, ClaudeClient, etc.)
- Provider clients don't share a common interface

**In manager (src/llm_client.rs):**
- Separate `LlmClient` trait with different method signatures
- Custom wrapper types (LlmCompletionRequest, LlmMessage, etc.)
- `SdkLlmClient` wrapper that translates between manager types and SDK types
- Unnecessary `Box<dyn LlmClient>` indirection for single implementation

This creates:
- Code duplication (~900 lines in manager)
- Type conversion overhead
- SDK that's hard to use by other projects
- Maintenance burden across two type systems

## Goals

1. **Make SDK trait-based**: All provider clients implement `LlmClient` trait
2. **Unify type system**: Manager uses SDK types directly
3. **Remove indirection**: Eliminate manager's wrapper layer
4. **Improve reusability**: Make SDK easy to use by other projects (e.g., Indistocks)

## Architecture Changes

### Before

```
┌─────────────────────────────────────┐
│           Manager                   │
│  ┌──────────────────────────────┐  │
│  │  LlmClient trait             │  │
│  │  (manager-specific)          │  │
│  └──────────────────────────────┘  │
│              ↓                      │
│  ┌──────────────────────────────┐  │
│  │  SdkLlmClient wrapper        │  │
│  │  (converts types)            │  │
│  └──────────────────────────────┘  │
│              ↓                      │
│  ┌──────────────────────────────┐  │
│  │  nocodo-llm-sdk              │  │
│  │  - OpenAIClient              │  │
│  │  - ClaudeClient              │  │
│  │  - XaiGrokClient             │  │
│  │  (no shared trait)           │  │
│  └──────────────────────────────┘  │
└─────────────────────────────────────┘
```

### After

```
┌─────────────────────────────────────┐
│           Manager                   │
│  ┌──────────────────────────────┐  │
│  │  Direct SDK usage            │  │
│  │  use nocodo_llm_sdk::*       │  │
│  └──────────────────────────────┘  │
└─────────────────────────────────────┘
              ↓
┌─────────────────────────────────────┐
│      nocodo-llm-sdk                 │
│  ┌──────────────────────────────┐  │
│  │  LlmClient trait             │  │
│  │  (provider-agnostic)         │  │
│  └──────────────────────────────┘  │
│         ↓          ↓          ↓     │
│  ┌────────┐  ┌────────┐  ┌──────┐ │
│  │OpenAI  │  │Claude  │  │Grok  │ │
│  │Client  │  │Client  │  │Client│ │
│  │(impl)  │  │(impl)  │  │(impl)│ │
│  └────────┘  └────────┘  └──────┘ │
└─────────────────────────────────────┘
```

## Implementation Plan

### Phase 1: Enhance SDK with Trait Implementations

#### 1.1 Update LlmClient Trait (nocodo-llm-sdk/src/client.rs)

**Current trait:**
```rust
#[allow(async_fn_in_trait)]
pub trait LlmClient {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError>;
    fn provider_name(&self) -> &str;
    fn model_name(&self) -> &str;
}
```

**Enhanced trait:**
```rust
#[async_trait]
pub trait LlmClient: Send + Sync {
    /// Complete a request (non-streaming)
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError>;

    /// Get provider name (e.g., "openai", "anthropic")
    fn provider_name(&self) -> &str;

    /// Get model name (e.g., "gpt-4o", "claude-sonnet-4-5")
    fn model_name(&self) -> &str;

    /// Check if streaming is supported
    fn supports_streaming(&self) -> bool {
        false
    }

    /// Stream completion (optional, returns error if not supported)
    fn stream_complete(
        &self,
        _request: CompletionRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, LlmError>> + Send>>, LlmError> {
        Err(LlmError::NotSupported("Streaming not supported".into()))
    }
}
```

**Files to modify:**
- `nocodo-llm-sdk/src/client.rs` - Update trait definition
- Add `async-trait` dependency to SDK Cargo.toml

#### 1.2 Implement Trait for OpenAI (nocodo-llm-sdk/src/openai/client.rs)

```rust
#[async_trait]
impl LlmClient for OpenAIClient {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        // Convert CompletionRequest to OpenAI-specific format
        let openai_request = self.build_request(request)?;
        let response = self.send_request(openai_request).await?;
        // Convert OpenAI response to CompletionResponse
        self.parse_response(response)
    }

    fn provider_name(&self) -> &str {
        "openai"
    }

    fn model_name(&self) -> &str {
        &self.model
    }
}
```

**Implementation details:**
- Add conversion methods between `CompletionRequest`/`CompletionResponse` and OpenAI types
- Handle both Chat Completions API and Responses API (gpt-5-codex)
- Support tool calling in unified format

#### 1.3 Implement Trait for Claude (nocodo-llm-sdk/src/claude/client.rs)

```rust
#[async_trait]
impl LlmClient for ClaudeClient {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let claude_request = self.build_claude_request(request)?;
        let response = self.send_request(claude_request).await?;
        self.parse_claude_response(response)
    }

    fn provider_name(&self) -> &str {
        "anthropic"
    }

    fn model_name(&self) -> &str {
        &self.model
    }
}
```

**Claude-specific considerations:**
- System messages handled separately (not in messages array)
- Tool results returned as user messages
- Content blocks structure

#### 1.4 Implement Trait for Grok (nocodo-llm-sdk/src/grok/xai/client.rs)

```rust
#[async_trait]
impl LlmClient for XaiGrokClient {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        // Similar to OpenAI implementation
        // Grok uses OpenAI-compatible API
        ...
    }

    fn provider_name(&self) -> &str {
        "xai"
    }

    fn model_name(&self) -> &str {
        &self.model
    }
}
```

#### 1.5 Implement Trait for GLM Providers

**Cerebras GLM** (nocodo-llm-sdk/src/glm/cerebras/client.rs):
```rust
#[async_trait]
impl LlmClient for CerebrasGlmClient {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        ...
    }

    fn provider_name(&self) -> &str {
        "cerebras"
    }

    fn model_name(&self) -> &str {
        &self.model
    }
}
```

**Zen GLM** (nocodo-llm-sdk/src/glm/zen/client.rs):
```rust
#[async_trait]
impl LlmClient for ZenGlmClient {
    ...
}
```

**Zen Grok** (nocodo-llm-sdk/src/grok/zen/client.rs):
```rust
#[async_trait]
impl LlmClient for ZenGrokClient {
    ...
}
```

#### 1.6 Add Helper Types to SDK

**StreamChunk** (nocodo-llm-sdk/src/types.rs):
```rust
/// Streaming response chunk
#[derive(Debug, Clone)]
pub struct StreamChunk {
    /// Text content in this chunk
    pub content: String,
    /// Whether this is the final chunk
    pub is_finished: bool,
    /// Tool calls (if any)
    pub tool_calls: Vec<ToolCall>,
}
```

**ToolCall/ToolResult** (nocodo-llm-sdk/src/tools.rs - already exists):
- Already has `Tool`, `ToolCall`, `ToolChoice`, `ToolResult`
- May need minor additions for consistency

### Phase 2: Remove Manager Abstraction Layer

#### 2.1 Remove Manager Types (manager/src/llm_client.rs)

**Delete these types (~500 lines):**
```rust
// ❌ Remove
pub struct LlmMessage { ... }
pub struct LlmCompletionRequest { ... }
pub struct LlmCompletionResponse { ... }
pub struct LlmChoice { ... }
pub struct LlmToolCall { ... }
pub struct LlmToolCallFunction { ... }
pub struct LlmMessageDelta { ... }
pub struct LlmUsage { ... }
pub struct StreamChunk { ... }
pub struct ToolDefinition { ... }
pub struct FunctionDefinition { ... }
pub enum ToolChoice { ... }
pub struct CompletionResult { ... }
```

**Keep/Replace with SDK imports:**
```rust
// ✅ Use SDK types
pub use nocodo_llm_sdk::client::LlmClient;
pub use nocodo_llm_sdk::types::{
    CompletionRequest,
    CompletionResponse,
    Message,
    ContentBlock,
    Role,
    Usage,
};
pub use nocodo_llm_sdk::tools::{
    Tool,
    ToolCall,
    ToolChoice,
    ToolResult,
};
pub use nocodo_llm_sdk::error::LlmError;

// Re-export model constants for convenience
pub use nocodo_llm_sdk::claude::SONNET_4_5 as CLAUDE_SONNET_4_5_MODEL_ID;
```

#### 2.2 Remove SdkLlmClient Wrapper

**Delete (~400 lines):**
```rust
// ❌ Remove entire wrapper
pub struct SdkLlmClient {
    provider: String,
    model: String,
    inner: ClientType,
}

enum ClientType {
    OpenAI(nocodo_llm_sdk::openai::client::OpenAIClient),
    Claude(nocodo_llm_sdk::claude::client::ClaudeClient),
    Grok(nocodo_llm_sdk::grok::xai::XaiGrokClient),
    Glm(nocodo_llm_sdk::glm::cerebras::CerebrasGlmClient),
    ZenGrok(nocodo_llm_sdk::grok::zen::ZenGrokClient),
    ZenGlm(nocodo_llm_sdk::glm::zen::ZenGlmClient),
}

#[async_trait]
impl LlmClient for SdkLlmClient {
    // 400+ lines of type conversion code
}
```

#### 2.3 Replace Factory Function (manager/src/llm_client.rs)

**Before:**
```rust
pub fn create_llm_client(config: LlmProviderConfig) -> Result<Box<dyn LlmClient>> {
    Ok(Box::new(SdkLlmClient::new(&config)?))
}
```

**After:**
```rust
pub fn create_llm_client(config: LlmProviderConfig) -> Result<Box<dyn LlmClient>> {
    match config.provider.to_lowercase().as_str() {
        "openai" => {
            let mut client = OpenAIClient::new(&config.api_key)?;
            if let Some(base_url) = &config.base_url {
                client = client.with_base_url(base_url);
            }
            Ok(Box::new(client))
        }
        "anthropic" | "claude" => {
            let client = ClaudeClient::new(&config.api_key)?;
            Ok(Box::new(client))
        }
        "grok" | "xai" => {
            let client = XaiGrokClient::new(&config.api_key)?;
            Ok(Box::new(client))
        }
        "cerebras" | "zai" | "glm" => {
            let client = CerebrasGlmClient::new(&config.api_key)?;
            Ok(Box::new(client))
        }
        "zen-grok" | "zengrok" => {
            let client = if config.api_key.is_empty() {
                ZenGrokClient::new()?
            } else {
                ZenGrokClient::with_api_key(&config.api_key)?
            };
            Ok(Box::new(client))
        }
        "zen-glm" | "zenglm" | "zen" => {
            let client = if config.api_key.is_empty() {
                ZenGlmClient::new()?
            } else {
                ZenGlmClient::with_api_key(&config.api_key)?
            };
            Ok(Box::new(client))
        }
        _ => anyhow::bail!("Unsupported provider: {}", config.provider),
    }
}
```

#### 2.4 Update LlmAgent Usage (manager/src/llm_agent.rs)

**Before (~1500 lines of type conversion):**
```rust
// Build conversation for LLM
let mut messages = Vec::new();
for msg in &history {
    let (content, tool_calls) = if msg.role == "assistant" {
        // Parse JSON to extract tool calls
        ...
    };

    messages.push(LlmMessage {
        role: msg.role.clone(),
        content,
        tool_calls,
        function_call: None,
        tool_call_id: None,
    });
}

let request = LlmCompletionRequest {
    model: session.model.clone(),
    messages,
    max_tokens: Some(4000),
    temperature,
    stream: Some(false),
    tools,
    tool_choice: Some(crate::llm_client::ToolChoice::Auto("auto".to_string())),
    functions: None,
    function_call: None,
};

let response = llm_client.complete(request).await?;
```

**After (simpler, cleaner):**
```rust
use nocodo_llm_sdk::types::{Message, ContentBlock, Role};

// Build conversation for LLM
let mut messages = Vec::new();
for msg in &history {
    messages.push(Message {
        role: match msg.role.as_str() {
            "user" => Role::User,
            "assistant" => Role::Assistant,
            "system" => Role::System,
            _ => continue,
        },
        content: vec![ContentBlock::Text {
            text: msg.content.clone(),
        }],
    });
}

let request = CompletionRequest {
    messages,
    max_tokens: 4000,
    model: session.model.clone(),
    system: None,
    temperature,
    top_p: None,
    stop_sequences: None,
};

let response = llm_client.complete(request).await?;
```

**Key simplifications:**
- Use SDK's `Message` type directly
- No JSON parsing for tool calls (handled by SDK)
- Cleaner enum usage (`Role::User` vs `"user"` strings)
- Fewer optional fields

### Phase 3: Update Tests

#### 3.1 Update Manager Tests

**Files to update:**
- `manager/tests/integration/llm_agent.rs`
- `manager/tests/common/llm_config.rs`

**Changes:**
```rust
// Before
use crate::llm_client::{create_llm_client, LlmCompletionRequest, LlmMessage};

// After
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::types::{CompletionRequest, Message, Role};
```

#### 3.2 Add SDK Trait Tests

**New file: nocodo-llm-sdk/tests/trait_tests.rs**
```rust
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::types::*;
use nocodo_llm_sdk::openai::OpenAIClient;
use nocodo_llm_sdk::claude::ClaudeClient;

#[test]
fn test_all_clients_implement_trait() {
    fn assert_implements_trait<T: LlmClient>() {}

    assert_implements_trait::<OpenAIClient>();
    assert_implements_trait::<ClaudeClient>();
    assert_implements_trait::<XaiGrokClient>();
    assert_implements_trait::<CerebrasGlmClient>();
}

#[tokio::test]
async fn test_trait_object_usage() {
    let client: Box<dyn LlmClient> = Box::new(
        OpenAIClient::new("test-key").unwrap()
    );

    assert_eq!(client.provider_name(), "openai");
}
```

### Phase 4: Update Documentation

#### 4.1 Update SDK README (nocodo-llm-sdk/README.md)

Add trait-based usage examples:
```markdown
## Trait-Based Usage

All clients implement the `LlmClient` trait, allowing provider-agnostic code:

\`\`\`rust
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::types::*;

fn create_client(provider: &str, api_key: &str) -> Box<dyn LlmClient> {
    match provider {
        "openai" => Box::new(OpenAIClient::new(api_key).unwrap()),
        "claude" => Box::new(ClaudeClient::new(api_key).unwrap()),
        "grok" => Box::new(XaiGrokClient::new(api_key).unwrap()),
        _ => panic!("Unsupported provider"),
    }
}

async fn chat(client: &dyn LlmClient, prompt: &str) -> String {
    let request = CompletionRequest {
        messages: vec![Message::user(prompt)],
        max_tokens: 1000,
        model: client.model_name().to_string(),
        system: None,
        temperature: Some(0.7),
        top_p: None,
        stop_sequences: None,
    };

    let response = client.complete(request).await.unwrap();
    extract_text(&response.content)
}
\`\`\`
```

#### 4.2 Update Manager README

Document the simplified LLM client usage without wrapper layer.

## Testing Strategy

### Unit Tests
- ✅ Test each provider's `LlmClient` implementation
- ✅ Test type conversions in each provider
- ✅ Test trait object creation and usage

### Integration Tests
- ✅ Test manager with real SDK clients
- ✅ Test all providers through unified interface
- ✅ Test tool calling with SDK types

### Manual Testing
- ✅ Run existing LLM agent sessions
- ✅ Verify tool execution still works
- ✅ Test all supported providers (OpenAI, Claude, Grok, GLM)

## Migration Checklist

### SDK Changes
- [ ] Update `LlmClient` trait with async-trait
- [ ] Implement trait for `OpenAIClient`
- [ ] Implement trait for `ClaudeClient`
- [ ] Implement trait for `XaiGrokClient`
- [ ] Implement trait for `CerebrasGlmClient`
- [ ] Implement trait for `ZenGlmClient`
- [ ] Implement trait for `ZenGrokClient`
- [ ] Add `StreamChunk` type to SDK
- [ ] Add conversion helpers for each provider
- [ ] Update SDK tests
- [ ] Update SDK documentation

### Manager Changes
- [ ] Remove custom `LlmClient` trait
- [ ] Remove all custom LLM types (~500 lines)
- [ ] Remove `SdkLlmClient` wrapper (~400 lines)
- [ ] Update `create_llm_client()` factory
- [ ] Update `llm_agent.rs` to use SDK types
- [ ] Update conversation reconstruction logic
- [ ] Update tool calling to use SDK types
- [ ] Update manager tests
- [ ] Update manager documentation

### Validation
- [ ] All tests pass
- [ ] Integration tests work with all providers
- [ ] Tool calling still works
- [ ] LLM agent sessions work end-to-end
- [ ] Code size reduced by ~900 lines in manager
- [ ] No performance regression

## Expected Benefits

### Code Quality
- **~900 lines removed** from manager
- **Single source of truth** for types
- **Cleaner abstractions** via trait
- **Less type conversion** overhead

### Reusability
- SDK can be **used by other projects** (e.g., Indistocks)
- Provider-agnostic interface
- Easy to add new providers
- Better testability with mock implementations

### Maintainability
- **Single type system** to maintain
- Bug fixes benefit all users
- Easier to add features (streaming, etc.)
- Clear separation of concerns

## Rollout Plan

1. **Phase 1** (1-2 days): Implement trait in SDK
2. **Phase 2** (1 day): Remove manager abstractions
3. **Phase 3** (0.5 days): Update tests
4. **Phase 4** (0.5 days): Documentation
5. **Validation** (0.5 days): End-to-end testing

**Total estimated time**: 3-4 days

## Success Criteria

- [x] All provider clients implement `LlmClient` trait
- [x] Manager uses SDK types directly (no wrapper)
- [x] All existing functionality works
- [x] Tests pass
- [x] Code reduced by ~900 lines
- [x] SDK can be used by external projects
- [x] Documentation updated

## Related Tasks

- [Indistocks Migration](../../Projects/Indistocks/tasks/migrate-to-nocodo-llm-sdk.md) - Migrate Indistocks to use trait-based SDK

## References

- Current SDK: `nocodo-llm-sdk/src/client.rs`
- Manager LLM client: `manager/src/llm_client.rs`
- Manager LLM agent: `manager/src/llm_agent.rs`
- SDK documentation: `nocodo-llm-sdk/README.md`
