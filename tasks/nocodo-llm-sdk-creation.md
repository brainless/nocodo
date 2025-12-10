# Task: Create nocodo-llm-sdk - General Purpose LLM SDK for Rust

**Status**: In Progress
**Priority**: High
**Created**: 2025-12-07
**Current Version**: v0.1 - Complete (Claude + Grok)
**Next Version**: v0.2 - Multi-Provider Architecture

---

## Overview

Create a new standalone crate `nocodo-llm-sdk` as a general-purpose LLM SDK for the Rust ecosystem.

**⚠️ IMPORTANT: This crate is completely isolated from the manager crate. Manager integration is postponed until multi-provider architecture is robust and stable.**

The SDK is being developed independently as a standalone, general-purpose crate. This allows us to:
- Design the SDK API without manager constraints
- Learn from real-world usage patterns
- Build a robust multi-provider architecture first
- Iterate quickly without breaking manager
- Potentially release as a standalone community crate

**Priority**: Robustness and multi-provider support are more important than manager integration.

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

### v0.1 Status (COMPLETE ✅)
- ✅ Claude (Anthropic) - Full Messages API implementation
- ✅ Grok (xAI) - OpenAI-compatible API with grok-code-fast-1
- ✅ Generic types layer (CompletionRequest/Response)
- ✅ LlmClient trait for provider abstraction
- ✅ Comprehensive error handling
- ✅ Builder pattern for ergonomic API
- ✅ Integration tests with real APIs
- ✅ Documentation and examples

### v0.2 Goals - Multi-Provider Architecture
- **Primary Focus**: Support same models via different providers
  - Claude via: Anthropic (native), Google Vertex AI, AWS Bedrock, Azure
  - GPT via: OpenAI (native), Azure OpenAI
- **Authentication Abstraction**: Support API keys, OAuth2, service accounts, IAM
- **Provider Selection**: Clear mechanism for users to choose provider
- **Feature Parity Handling**: Deal with provider-specific features and lag
- **Robust Architecture**: Extensible design for future providers

### Future (v0.3+)
- Manager integration (only after v0.2 is stable)
- Advanced features (streaming, tool use, vision, etc.)

---

## Resources

### API Documentation
- **Claude Messages API**: `external-docs/claude_message_api.md`
- **Claude Errors**: `external-docs/claude_api_errors.md`
- **xAI API Reference**: `external-docs/xai_api_reference.md`
- **Existing Integration Specs**: `specs/LLM_INTEGRATION.md`

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
- `external-docs/claude_message_api.md` - API specification
- `external-docs/claude_api_errors.md` - Error handling
- Official Anthropic Python SDK - Design patterns

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
   - Parse Claude-specific errors from `external-docs/claude_api_errors.md`:
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

### Completed in v0.1
- [x] xAI implementation (Grok)
- [x] Basic provider abstraction (LlmClient trait)

### Defer to v0.2 (Multi-Provider Architecture)
- [ ] Multiple providers for same models (Vertex AI, Bedrock, Azure)
- [ ] Authentication abstraction (OAuth2, IAM, etc.)
- [ ] Provider-specific clients (AnthropicClaudeClient, VertexClaudeClient, etc.)
- [ ] Feature flags for optional providers

### Defer to Future Versions (After v0.2 is Stable)
- [ ] Streaming support
- [ ] Tool use / function calling
- [ ] Vision support
- [ ] OpenAI GPT models
- [ ] Provider discovery/registry
- [ ] **Integration with manager crate** (postponed indefinitely)

**Note**: Manager integration is explicitly postponed until multi-provider architecture is robust.

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
1. Claude types (`claude/types.rs`) - based on `external-docs/claude_message_api.md`
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

## v0.2: Multi-Provider Architecture

**Status**: Planned
**Priority**: Critical for Robustness
**Goal**: Support same models via different providers with extensible architecture

### Overview

v0.2 focuses on building a robust multi-provider architecture that allows users to access the same LLM models (like Claude) through different providers (Anthropic, Google Vertex AI, AWS Bedrock, Azure). This is critical for:

- **Infrastructure Flexibility**: Users can choose providers based on existing cloud infrastructure
- **Cost Optimization**: Different providers have different pricing
- **Regional Availability**: Some providers may be available in regions others aren't
- **Redundancy**: Ability to switch providers if one has outages
- **Feature Access**: Early access to new features via specific providers

**⚠️ Manager integration is explicitly postponed until this architecture is stable.**

---

### The Multi-Provider Challenge

#### Same Model, Different Providers

Current reality in the LLM ecosystem:

**Claude Models Available Via:**
- **Anthropic** (native) - Direct API with API key
- **Google Vertex AI** - Google Cloud infrastructure with OAuth2/service accounts
- **AWS Bedrock** - AWS infrastructure with IAM roles
- **Azure** - Azure OpenAI Service with Azure AD

**GPT Models Available Via:**
- **OpenAI** (native) - Direct API with API key
- **Azure OpenAI** - Azure infrastructure with different endpoints

**Key Challenge**: Each provider has:
- Different authentication mechanisms (API key vs OAuth2 vs IAM)
- Different endpoint structures
- Different request/response formats (minor variations)
- Different feature availability and rollout timing

---

### Architecture Approach

#### Option A: Separate Client Types (RECOMMENDED)

Create distinct client types for each provider, sharing common types and traits.

**Structure:**
```
src/
├── types.rs              # Shared generic types
├── client.rs             # LlmClient trait
├── error.rs              # Shared error types
├── auth/                 # Authentication abstractions
│   ├── mod.rs
│   ├── api_key.rs       # API key auth
│   ├── oauth2.rs        # OAuth2/Google Cloud auth
│   ├── aws_iam.rs       # AWS IAM auth
│   └── azure_ad.rs      # Azure AD auth
├── claude/
│   ├── types.rs         # Shared Claude request/response types
│   ├── anthropic/
│   │   ├── client.rs    # AnthropicClaudeClient
│   │   └── builder.rs
│   ├── vertex/
│   │   ├── client.rs    # VertexClaudeClient
│   │   └── builder.rs
│   ├── bedrock/
│   │   ├── client.rs    # BedrockClaudeClient
│   │   └── builder.rs
│   └── azure/
│       ├── client.rs    # AzureClaudeClient
│       └── builder.rs
├── gpt/
│   ├── types.rs         # Shared GPT request/response types
│   ├── openai/
│   │   ├── client.rs    # OpenAIClient
│   │   └── builder.rs
│   └── azure/
│       ├── client.rs    # AzureOpenAIClient
│       └── builder.rs
└── grok/
    ├── types.rs
    └── client.rs         # xAI only provider for Grok
```

**Usage Example:**
```rust
// Anthropic native
let client = AnthropicClaudeClient::new("sk-ant-...")?;

// Google Vertex AI
let client = VertexClaudeClient::new(
    "my-project",
    "us-central1",
    GoogleCredentials::from_file("service-account.json")?
)?;

// AWS Bedrock
let client = BedrockClaudeClient::new(
    "us-east-1",
    AwsCredentials::from_environment()?
)?;

// All implement the same LlmClient trait and use same model names
let response = client
    .message_builder()
    .model("claude-3-sonnet-20240229")  // Same model name
    .max_tokens(1024)
    .user_message("Hello!")
    .send()
    .await?;
```

**Pros:**
- ✅ Clear and explicit - users know exactly which provider they're using
- ✅ Each client can have provider-specific methods if needed
- ✅ Different authentication per client type (compile-time safety)
- ✅ Easy to maintain - provider logic is isolated
- ✅ No complex enums or runtime dispatch

**Cons:**
- ⚠️ More types for users to learn
- ⚠️ Some code duplication (mitigated by shared types and helper functions)

---

#### Option B: Provider Adapter Pattern (ALTERNATIVE)

Single client type with pluggable provider adapters.

**Structure:**
```rust
pub trait ClaudeProvider {
    async fn authenticate(&self) -> Result<AuthToken>;
    fn build_endpoint(&self, model: &str) -> String;
    fn prepare_request(&self, req: &ClaudeMessageRequest) -> ProviderSpecificRequest;
    fn parse_response(&self, resp: ProviderSpecificResponse) -> ClaudeMessageResponse;
}

pub struct AnthropicProvider { api_key: String }
pub struct VertexProvider { project_id: String, region: String, creds: GoogleCredentials }
pub struct BedrockProvider { region: String, creds: AwsCredentials }

pub struct ClaudeClient<P: ClaudeProvider> {
    provider: P,
    http_client: reqwest::Client,
}
```

**Usage:**
```rust
// Anthropic
let client = ClaudeClient::with_provider(
    AnthropicProvider::new("sk-ant-...")?
);

// Vertex AI
let client = ClaudeClient::with_provider(
    VertexProvider::new("my-project", "us-central1", creds)?
);
```

**Pros:**
- ✅ Single client type
- ✅ Very extensible - easy to add new providers
- ✅ Shared logic in client, provider-specific in adapters

**Cons:**
- ⚠️ More complex implementation
- ⚠️ Generic types may confuse users
- ⚠️ Harder to add provider-specific methods

---

### Authentication Requirements

#### Authentication Types to Support

1. **API Key Authentication** (Anthropic, OpenAI, xAI)
   ```rust
   pub struct ApiKeyAuth {
       api_key: String,
       header_name: String,  // "x-api-key" or "Authorization"
       header_format: String, // "{key}" or "Bearer {key}"
   }
   ```

2. **Google Cloud OAuth2** (Vertex AI)
   ```rust
   pub struct GoogleCloudAuth {
       credentials: GoogleCredentials,
       scopes: Vec<String>,
   }

   pub enum GoogleCredentials {
       ServiceAccount { json_key: String },
       ApplicationDefault,
       ComputeEngine,
   }
   ```

3. **AWS IAM** (Bedrock)
   ```rust
   pub struct AwsIamAuth {
       credentials: AwsCredentials,
       region: String,
   }

   pub enum AwsCredentials {
       AccessKey { access_key_id: String, secret_access_key: String },
       Environment,
       InstanceProfile,
   }
   ```

4. **Azure AD** (Azure OpenAI)
   ```rust
   pub struct AzureAdAuth {
       tenant_id: String,
       client_id: String,
       client_secret: String,
   }
   ```

#### Dependencies Required

```toml
[dependencies]
# Existing
reqwest = { version = "0.11", features = ["json", "stream"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
thiserror = "1.0"
tracing = "0.1"

# New for v0.2
google-authz = { version = "1.0", optional = true }      # Google Cloud auth
aws-config = { version = "1.0", optional = true }        # AWS auth
aws-credential-types = { version = "1.0", optional = true }
azure-identity = { version = "1.0", optional = true }    # Azure AD auth

[features]
default = ["anthropic", "xai"]
anthropic = []
xai = []
vertex = ["google-authz"]
bedrock = ["aws-config", "aws-credential-types"]
azure = ["azure-identity"]
all-providers = ["anthropic", "xai", "vertex", "bedrock", "azure"]
```

---

### Provider-Specific Differences

#### Anthropic vs Vertex AI: Claude API Differences

| Aspect | Anthropic | Vertex AI |
|--------|-----------|-----------|
| **Authentication** | `x-api-key: {key}` header | OAuth2 access token in `Authorization: Bearer {token}` |
| **Endpoint** | `https://api.anthropic.com/v1/messages` | `https://{region}-aiplatform.googleapis.com/v1/projects/{project}/locations/{region}/publishers/anthropic/models/{model}:rawPredict` |
| **Model Parameter** | In request body: `{"model": "claude-3-sonnet-20240229", ...}` | In URL path, NOT in body |
| **anthropic_version** | Header: `anthropic-version: 2023-06-01` | Body field: `{"anthropic_version": "vertex-2023-10-16", ...}` |
| **Streaming** | `POST /v1/messages` with `stream: true` | `:streamRawPredict` endpoint |
| **Feature Rollout** | Immediate | May lag behind Anthropic |

**Implementation Impact:**
- Need conditional URL building
- Need conditional serialization (model in body vs URL)
- Need different auth flows
- Need to handle version parameter differently

---

### Implementation Plan for v0.2

#### Phase 1: Authentication Abstraction (Week 1)

1. **Design auth trait and types**
   ```rust
   #[async_trait]
   pub trait AuthProvider: Send + Sync {
       async fn get_auth_header(&self) -> Result<(String, String), LlmError>;
       fn requires_refresh(&self) -> bool;
       async fn refresh(&mut self) -> Result<(), LlmError>;
   }
   ```

2. **Implement API key auth** (already mostly done)

3. **Implement Google Cloud OAuth2 auth**
   - Service account JSON key support
   - Application default credentials
   - Token refresh logic

4. **Add tests for all auth types**

#### Phase 2: Provider-Specific Clients (Week 2-3)

1. **Refactor existing AnthropicClaudeClient**
   - Extract shared logic
   - Make it explicitly Anthropic-specific

2. **Implement VertexClaudeClient**
   - Google Cloud endpoint building
   - Request transformation (model in URL, version in body)
   - Response handling

3. **Implement BedrockClaudeClient** (if AWS credentials available)
   - AWS endpoint structure
   - IAM signature v4

4. **Update builders**
   - Ensure all providers share same builder API
   - Provider-specific options as separate methods

#### Phase 3: Shared Types and Traits (Week 3)

1. **Refine LlmClient trait**
   ```rust
   #[async_trait]
   pub trait LlmClient: Send + Sync {
       async fn complete(&self, request: CompletionRequest)
           -> Result<CompletionResponse, LlmError>;

       fn provider_name(&self) -> &str;
       fn provider_type(&self) -> ProviderType;  // New
       fn supports_streaming(&self) -> bool;     // New
       fn max_context_tokens(&self, model: &str) -> Option<u32>;  // New
   }

   pub enum ProviderType {
       Anthropic,
       VertexAI,
       Bedrock,
       Azure,
       OpenAI,
       XAI,
   }
   ```

2. **Update error types**
   - Add provider-specific error variants if needed
   - Ensure errors include provider context

3. **Ensure type compatibility**
   - All provider clients use same generic types
   - Clean conversions between provider-specific and generic types

#### Phase 4: Testing & Documentation (Week 4)

1. **Integration tests**
   - Tests for each provider (with feature flags)
   - Cross-provider compatibility tests
   - Authentication tests

2. **Examples**
   - `examples/anthropic_claude.rs`
   - `examples/vertex_claude.rs`
   - `examples/bedrock_claude.rs`
   - `examples/provider_comparison.rs`

3. **Documentation**
   - Update README with all providers
   - Provider selection guide
   - Authentication setup guides
   - Migration guide from v0.1

---

### Success Criteria for v0.2

v0.2 is complete when:

1. ✅ **Multi-provider support for Claude**
   - AnthropicClaudeClient (Anthropic native)
   - VertexClaudeClient (Google Cloud)
   - BedrockClaudeClient (AWS) - optional

2. ✅ **Authentication abstraction works**
   - API key auth
   - Google Cloud OAuth2
   - AWS IAM (if Bedrock implemented)

3. ✅ **All providers pass integration tests**
   - Real API calls to each provider
   - Same model works across providers
   - Error handling is consistent

4. ✅ **Clean, documented API**
   - Clear provider selection
   - Comprehensive examples
   - Migration guide from v0.1

5. ✅ **Feature flags work correctly**
   - Users can opt into only needed providers
   - Dependencies are optional based on features

6. ✅ **Code quality maintained**
   - No clippy warnings
   - >80% test coverage
   - All documentation updated

---

### Migration from v0.1 to v0.2

#### Breaking Changes

```rust
// v0.1
use nocodo_llm_sdk::claude::ClaudeClient;
let client = ClaudeClient::new(api_key)?;

// v0.2 - Explicit provider
use nocodo_llm_sdk::claude::anthropic::AnthropicClaudeClient;
let client = AnthropicClaudeClient::new(api_key)?;

// Or with type alias for backwards compatibility
use nocodo_llm_sdk::claude::ClaudeClient;  // Type alias to AnthropicClaudeClient
let client = ClaudeClient::new(api_key)?;  // Still works
```

#### Compatibility Layer

Provide type aliases for smooth migration:
```rust
// In src/claude/mod.rs
#[deprecated(since = "0.2.0", note = "Use AnthropicClaudeClient explicitly")]
pub type ClaudeClient = anthropic::AnthropicClaudeClient;
```

---

### Timeline Estimate

- **Week 1**: Authentication abstraction (20-30 hours)
- **Week 2-3**: Provider implementations (40-60 hours)
  - Anthropic refactor: 5 hours
  - Vertex AI: 20-25 hours
  - Bedrock: 15-20 hours (optional)
- **Week 4**: Testing, docs, examples (20-30 hours)

**Total**: 80-120 hours for robust v0.2

**Staged Rollout Option**:
- v0.2.0-alpha: Anthropic + Vertex AI only
- v0.2.0-beta: Add Bedrock
- v0.2.0: Stable with all providers

---

## Future: Manager Integration (Version TBD)

**Status**: Postponed indefinitely
**Priority**: Low - SDK robustness is more important

Manager integration is **explicitly postponed** until the SDK has a robust multi-provider architecture. The focus is on making `nocodo-llm-sdk` a solid, standalone library first.

### Prerequisites for Manager Integration

Manager integration will ONLY be considered when:

1. ✅ **v0.2 is stable and well-tested**
   - Multi-provider architecture proven
   - All providers work reliably
   - Authentication abstraction is solid
   - No major API changes expected

2. ✅ **Community adoption** (if released publicly)
   - Real-world usage patterns identified
   - API is validated by external users
   - Performance is acceptable

3. ✅ **Feature completeness**
   - Streaming support
   - Tool use / function calling
   - All critical features implemented

### Potential Integration Approach (For Reference Only)

When/if we eventually integrate with manager:

1. **Gradual migration strategy**:
   - Add `nocodo-llm-sdk` as optional dependency
   - Implement new SDK-based client alongside existing `llm_client`
   - Run both in parallel for testing
   - Feature flag to switch between implementations
   - Eventually deprecate old `llm_client` module

2. **Key considerations**:
   - Manager may have specific requirements not covered by SDK
   - SDK should remain independent (no manager-specific code)
   - Manager should adapt to SDK, not vice versa

**No timeline or version number assigned** - this will be decided when v0.2 is stable.

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

- Claude Messages API: `external-docs/claude_message_api.md`
- Claude Errors: `external-docs/claude_api_errors.md`
- xAI API Reference: `external-docs/xai_api_reference.md`
- LLM Integration Guide: `specs/LLM_INTEGRATION.md`
- Official Anthropic Python SDK (external reference)
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
