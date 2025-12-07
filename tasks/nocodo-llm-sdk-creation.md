# Task: Create nocodo-llm-sdk - General Purpose LLM SDK for Rust

**Status**: Pending
**Priority**: High
**Created**: 2025-12-07
**Version**: v0.1 - Claude Implementation

---

## Overview

Create a new standalone crate `nocodo-llm-sdk` as a general-purpose LLM SDK for the Rust ecosystem.

**⚠️ IMPORTANT: This crate will be completely isolated from the manager crate in v0.1. NO integration with manager until v0.2+.**

The SDK will be developed independently, tested thoroughly, and stabilized before any manager integration is considered. This allows us to:
- Design the SDK API without manager constraints
- Learn from real-world usage patterns
- Iterate quickly without breaking manager
- Potentially release as a standalone community crate

### Goals

1. **Clean, standalone LLM SDK** that can be used by any Rust project
2. **Start with Claude** (Anthropic) as the first concrete implementation
3. **Defer adapter pattern** to future versions (v0.2+) after learning from v0.1
4. **Reference official SDKs** for API design patterns
5. **Community value** - make it useful beyond nocodo
6. **NO manager integration** in v0.1 - this is a separate, independent crate

---

## Design Principles

### v0.1 Focus
- **Simplicity**: Just Claude Messages API, done right
- **Type Safety**: Leverage Rust's type system (better than Python SDK)
- **Ergonomics**: Builder patterns, sensible defaults
- **Error Handling**: Comprehensive error types with context
- **Async First**: Tokio-based with proper timeouts
- **Well Tested**: Both unit and integration tests from the start

### Future (v0.2+)
- Adapter pattern for multiple providers
- OpenAI, Anthropic, xAI, zAI implementations
- Integration with manager crate
- Advanced features (streaming, tool use, etc.)

---

## Resources

### API Documentation
- **Claude Messages API**: `/claude_message_api.md`
- **Claude Errors**: `/claude_api_errors.md`
- **Existing Integration Specs**: `/specs/LLM_INTEGRATION.md`

### Reference Implementations
- **Official Anthropic Python SDK**: `~/Projects/anthropic-sdk-python/`
- **Current manager implementation**: `manager/src/llm_client/`
  - Types: `types/claude_types.rs`
  - Adapter: `adapters/claude_messages.rs`
  - Provider: `llm_providers/anthropic.rs`

---

## Implementation Plan

### Phase 1: Crate Foundation

**Location**: `nocodo-llm-sdk/` (new crate in workspace)

```
nocodo-llm-sdk/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs
│   ├── error.rs          # Error types
│   ├── types.rs          # Common types (Message, Usage, etc.)
│   ├── client.rs         # Core client traits
│   ├── claude/           # Claude-specific implementation
│   │   ├── mod.rs
│   │   ├── types.rs      # Claude request/response types
│   │   ├── client.rs     # Claude client implementation
│   │   └── builder.rs    # Request builder
│   └── utils.rs          # Common utilities
├── tests/
│   ├── unit/
│   │   └── claude.rs     # Unit tests with mocks
│   └── integration/
│       └── claude.rs     # Integration tests with real API
└── examples/
    ├── simple_completion.rs
    ├── tool_use.rs
    └── streaming.rs
```

**Key Files**:

1. **`Cargo.toml`**
   ```toml
   [package]
   name = "nocodo-llm-sdk"
   version = "0.1.0"
   edition = "2021"

   [dependencies]
   reqwest = { version = "0.11", features = ["json", "stream"] }
   serde = { version = "1.0", features = ["derive"] }
   serde_json = "1.0"
   async-trait = "0.1"
   tokio = { version = "1.0", features = ["full"] }
   thiserror = "1.0"
   anyhow = "1.0"
   tracing = "0.1"

   [dev-dependencies]
   mockito = "1.0"
   tokio-test = "0.4"
   ```

2. **`error.rs`**
   - `LlmError` enum with variants:
     - `AuthenticationError`
     - `RateLimitError { retry_after: Option<u64> }`
     - `InvalidRequestError { message: String }`
     - `ApiError { status: u16, message: String }`
     - `NetworkError`
     - `ParseError`
   - Implement `std::error::Error` and `Display`
   - Use `thiserror` for ergonomic error definitions

3. **`types.rs`**
   - Common types across providers:
     - `Message { role, content }`
     - `Usage { prompt_tokens, completion_tokens, total_tokens }`
     - `Role` enum
     - `ContentBlock` enum (for multimodal)
   - Keep provider-agnostic in v0.1

4. **`client.rs`**
   - Basic client traits (simple for v0.1):
     ```rust
     #[async_trait]
     pub trait LlmClient {
         async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError>;
         fn provider_name(&self) -> &str;
         fn model_name(&self) -> &str;
     }
     ```

### Phase 2: Claude Implementation

**Based on**:
- `claude_message_api.md` - API specification
- `claude_api_errors.md` - Error handling
- `anthropic-sdk-python` - Design patterns

**Components**:

1. **`claude/types.rs`**
   - `ClaudeMessageRequest` - Maps to Messages API request format
   - `ClaudeMessageResponse` - Maps to Messages API response format
   - `ClaudeMessage`, `ClaudeContentBlock`, etc.
   - Derive `Serialize`, `Deserialize`
   - Handle all field types from API spec:
     - `max_tokens` (required)
     - `messages` (required)
     - `model` (required)
     - `metadata`, `stop_sequences`, `stream`, `system`, `temperature`, `thinking`, `tool_choice`, `tools`, `top_k`, `top_p`

2. **`claude/client.rs`**
   - `ClaudeClient` struct
   - Configuration:
     ```rust
     pub struct ClaudeClient {
         api_key: String,
         base_url: String,
         http_client: reqwest::Client,
     }
     ```
   - Methods:
     - `new(api_key: String) -> Result<Self>`
     - `with_base_url(mut self, url: String) -> Self`
     - `create_message(request: ClaudeMessageRequest) -> Result<ClaudeMessageResponse>`
   - HTTP request handling:
     - Headers: `x-api-key`, `anthropic-version`, `content-type`
     - Endpoint: `POST /v1/messages`
     - Timeout handling
     - Error parsing from response

3. **`claude/builder.rs`**
   - Ergonomic request builder:
     ```rust
     ClaudeClient::new(api_key)
         .create_message()
         .model("claude-sonnet-4-5-20250929")
         .max_tokens(1024)
         .message("user", "Hello")
         .temperature(0.7)
         .send()
         .await?
     ```

4. **Error Handling**
   - Parse Claude-specific errors from `claude_api_errors.md`:
     - HTTP 400 → `InvalidRequestError`
     - HTTP 401 → `AuthenticationError`
     - HTTP 403 → `PermissionError`
     - HTTP 404 → `NotFoundError`
     - HTTP 413 → `RequestTooLargeError`
     - HTTP 429 → `RateLimitError`
     - HTTP 500 → `ApiError`
     - HTTP 529 → `OverloadedError`
   - Extract `request_id` from response for debugging

### Phase 3: Testing

1. **Unit Tests** (`tests/unit/claude.rs`)
   - Mock HTTP responses using `mockito`
   - Test request serialization
   - Test response deserialization
   - Test error parsing
   - Test builder pattern

2. **Integration Tests** (`tests/integration/claude.rs`)
   - Require `ANTHROPIC_API_KEY` env var
   - Test real API calls:
     - Simple completion
     - Multi-turn conversation
     - Tool use (if implemented)
     - Error scenarios (invalid API key, etc.)
   - Use `#[tokio::test]`
   - Use `#[ignore]` for tests requiring API key

3. **Test Configuration**
   ```rust
   // tests/common/mod.rs
   pub fn get_api_key() -> Option<String> {
       std::env::var("ANTHROPIC_API_KEY").ok()
   }

   pub fn skip_if_no_api_key() {
       if get_api_key().is_none() {
           panic!("Skipping test - ANTHROPIC_API_KEY not set");
       }
   }
   ```

### Phase 4: Documentation & Examples

1. **README.md**
   - Quick start guide
   - Installation instructions
   - Basic usage example
   - Link to API documentation
   - Contribution guidelines

2. **Examples**
   - `examples/simple_completion.rs` - Basic completion
   - `examples/conversation.rs` - Multi-turn chat
   - `examples/tool_use.rs` - Function calling (if implemented)
   - `examples/error_handling.rs` - Error handling patterns

3. **API Documentation**
   - Comprehensive rustdoc comments
   - Code examples in doc comments
   - Link to Claude API docs

---

## Features for v0.1

### Must Have
- [x] Basic message completion (non-streaming)
- [x] Error handling with all HTTP error codes
- [x] Request builder pattern
- [x] Type-safe request/response types
- [x] Integration tests with real API
- [x] Documentation and examples

### Nice to Have (time permitting)
- [ ] Streaming support
- [ ] Tool use / function calling
- [ ] Vision support (image inputs)
- [ ] Extended thinking configuration
- [ ] Retry logic with exponential backoff
- [ ] Request/response logging

### Defer to v0.2+ (Future Work - NOT for v0.1)
- [ ] Adapter pattern for multiple providers
- [ ] OpenAI implementation
- [ ] xAI implementation
- [ ] **Integration with manager crate** ⚠️
- [ ] Provider discovery/registry
- [ ] Advanced configuration

**Note**: v0.1 is a completely standalone crate. Manager integration is explicitly deferred to v0.2+.

---

## Implementation Steps

### Step 1: Create Crate Structure
```bash
cd /Users/brainless/GitWorktrees/nocodo/openai-gpt-5-fixes
cargo new --lib nocodo-llm-sdk
cd nocodo-llm-sdk
```

### Step 2: Set Up Workspace
Add to root `Cargo.toml`:
```toml
[workspace]
members = [
    "manager",
    "nocodo-llm-sdk",
    # ... other crates
]
```

### Step 3: Implement Core
1. Define error types (`error.rs`)
2. Define common types (`types.rs`)
3. Define client trait (`client.rs`)

### Step 4: Implement Claude
1. Claude types (`claude/types.rs`) - based on `claude_message_api.md`
2. Claude client (`claude/client.rs`)
3. Request builder (`claude/builder.rs`)

### Step 5: Add Tests
1. Unit tests with mocks
2. Integration tests with real API
3. Run with: `ANTHROPIC_API_KEY=sk-... cargo test`

### Step 6: Documentation
1. Add rustdoc comments
2. Create examples
3. Write README

### Step 7: Validation
1. All tests pass
2. `cargo clippy` clean
3. `cargo fmt` applied
4. Documentation builds: `cargo doc --open`

---

## Success Criteria

### v0.1 is complete when:
1. ✅ **Compiles and tests pass**
   ```bash
   cargo build
   cargo test
   ANTHROPIC_API_KEY=sk-... cargo test --test integration
   ```

2. ✅ **Can make real API calls**
   ```rust
   let client = ClaudeClient::new(api_key)?;
   let response = client
       .create_message()
       .model("claude-sonnet-4-5-20250929")
       .max_tokens(1024)
       .message("user", "Hello, Claude!")
       .send()
       .await?;
   println!("Response: {}", response.content[0].text);
   ```

3. ✅ **Error handling works**
   - Invalid API key → clear error
   - Rate limiting → proper error with retry info
   - Malformed request → validation error

4. ✅ **Documentation complete**
   - README with examples
   - API docs (rustdoc)
   - Integration examples

5. ✅ **Code quality**
   - No clippy warnings
   - Formatted with rustfmt
   - Well-tested (>80% coverage)

---

## Future Integration with Manager (v0.2+ ONLY)

**⚠️ DO NOT integrate with manager in v0.1. This section is for future reference only.**

### When v0.1 is stable and we're ready for v0.2:

1. **Evaluate v0.1**:
   - Is the API stable?
   - Are all tests passing?
   - Is it production-ready?
   - Does it meet community standards?

2. **Plan v0.2** (multi-provider support):
   - Extract adapter pattern from manager
   - Implement multiple providers in SDK
   - Design integration points

3. **v0.3 - Manager Integration** (only after v0.2 is solid):
   - Add dependency in manager's `Cargo.toml`:
     ```toml
     [dependencies]
     nocodo-llm-sdk = { path = "../nocodo-llm-sdk", version = "0.2" }
     ```
   - Gradual migration:
     - Keep existing `llm_client` module working
     - Add new SDK-based implementation alongside
     - Test both implementations
     - Switch over when confident
   - Eventually deprecate manager's `llm_client` module

**Timeline**: v0.1 → stabilize → v0.2 (multi-provider) → stabilize → v0.3 (manager integration)

---

## Notes & Considerations

### Key Decisions for v0.1

1. **No adapter pattern yet**
   - Keep it simple for v0.1
   - Learn from concrete implementation
   - Add abstraction in v0.2 when patterns are clear

2. **Focus on correctness**
   - Get Claude API implementation right
   - Handle all edge cases
   - Comprehensive error handling

3. **Ergonomics matter**
   - Builder pattern for requests
   - Sensible defaults
   - Clear error messages

4. **Testing is critical**
   - Both unit and integration tests
   - Real API validation
   - Mock-based testing for CI/CD

### Lessons from Manager Implementation

**Good patterns to keep**:
- Separate types for request/response
- `ProviderRequest` trait for serialization
- Clear error types
- Async-first design

**Things to improve**:
- Simpler client creation (no factory yet)
- More ergonomic builder API
- Better error messages with context
- Comprehensive documentation

**Things to defer**:
- Adapter pattern (v0.2)
- Provider registry (v0.2)
- Multi-provider support (v0.2)
- Streaming (if time permits, otherwise v0.2)

---

## Timeline Estimate

- **Phase 1** (Foundation): 2-3 hours
- **Phase 2** (Claude Implementation): 4-6 hours
- **Phase 3** (Testing): 3-4 hours
- **Phase 4** (Documentation): 2-3 hours

**Total**: ~11-16 hours for a solid v0.1

---

## References

- Claude Messages API: `/claude_message_api.md`
- Claude Errors: `/claude_api_errors.md`
- LLM Integration Guide: `/specs/LLM_INTEGRATION.md`
- Anthropic Python SDK: `~/Projects/anthropic-sdk-python/`
- Current Implementation: `manager/src/llm_client/`

---

## Next Steps

When ready to start:
1. Create crate structure
2. Implement core types and traits
3. Implement Claude client
4. Add comprehensive tests
5. Write documentation
6. Validate with real API calls
7. Prepare for v0.2 (adapter pattern)
