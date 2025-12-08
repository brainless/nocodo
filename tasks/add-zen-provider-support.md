# Task: Add Zen Provider Support for Grok and GLM Models

**Status**: Not Started
**Priority**: High
**Created**: 2025-12-08
**Estimated Effort**: 6-8 hours
**Related**: tasks/nocodo-llm-sdk-creation.md (v0.2 Multi-Provider Architecture)

---

## Overview

Add OpenCode Zen as a second provider for existing Grok and GLM models in nocodo-llm-sdk. This validates the v0.2 multi-provider architecture with real, free-to-test providers.

**Models to Support:**
- **Grok Code Fast 1** via Zen (model ID: `grok-code`, free, no auth required)
- **Big Pickle** via Zen (model ID: `big-pickle`, free, routes to GLM 4.6, no auth required)

**Reference Documentation**: `external-docs/opencode_zen_docs.md`

---

## Current State

### Existing Providers
- **GLM 4.6**: Via Cerebras API (`https://api.cerebras.ai`)
  - Client: `GlmClient` in `nocodo-llm-sdk/src/glm/client.rs`
  - Model ID: `glm-4.6`
  - Auth: Required (API key)

- **Grok**: Via xAI API (`https://api.x.ai`)
  - Client: `GrokClient` in `nocodo-llm-sdk/src/grok/client.rs`
  - Model ID: `grok-code-fast-1`
  - Auth: Required (API key)

### Current File Structure
```
nocodo-llm-sdk/src/
├── grok/
│   ├── mod.rs
│   ├── client.rs        # GrokClient (xAI)
│   ├── types.rs
│   └── builder.rs
├── glm/
│   ├── mod.rs
│   ├── client.rs        # GlmClient (Cerebras)
│   ├── types.rs
│   └── builder.rs
├── types.rs
└── client.rs
```

---

## Target State

### New Multi-Provider Structure
```
nocodo-llm-sdk/src/
├── grok/
│   ├── mod.rs                    # Re-exports both providers
│   ├── types.rs                  # Shared Grok types
│   ├── builder.rs                # Shared builder
│   ├── xai/                      # xAI provider (existing, refactored)
│   │   └── client.rs             # XaiGrokClient
│   └── zen/                      # NEW: Zen provider
│       └── client.rs             # ZenGrokClient
├── glm/
│   ├── mod.rs                    # Re-exports both providers
│   ├── types.rs                  # Shared GLM types
│   ├── builder.rs                # Shared builder
│   ├── cerebras/                 # Cerebras provider (existing, refactored)
│   │   └── client.rs             # CerebrasGlmClient
│   └── zen/                      # NEW: Zen provider (Big Pickle)
│       └── client.rs             # ZenGlmClient
├── types.rs
└── client.rs
```

---

## Key Differences: Zen vs Native Providers

| Aspect | Native (xAI/Cerebras) | Zen (OpenCode) |
|--------|----------------------|----------------|
| **Base URL** | `https://api.x.ai`<br/>`https://api.cerebras.ai` | `https://opencode.ai/zen` |
| **Model Names** | `grok-code-fast-1`<br/>`glm-4.6` | `grok-code`<br/>`big-pickle` |
| **Authentication** | Required (API key) | Optional (free models) |
| **Auth Header** | `Authorization: Bearer {key}` | `Authorization: Bearer {key}` (if used) |
| **Endpoint** | `/v1/chat/completions` | `/v1/chat/completions` |
| **Request Format** | OpenAI-compatible | OpenAI-compatible (same) |
| **Response Format** | OpenAI-compatible | OpenAI-compatible (same) |
| **Cost** | Paid | Free during beta |

---

## Implementation Plan

### Phase 1: Refactor Existing Clients (2 hours)

#### 1.1 Refactor Grok Module

**Create provider-specific structure:**

1. **Create `nocodo-llm-sdk/src/grok/xai/` directory**

2. **Move and rename client:**
   - Move `grok/client.rs` → `grok/xai/client.rs`
   - Rename `GrokClient` → `XaiGrokClient`
   - Update base URL (already `https://api.x.ai`)
   - Update model reference (already `grok-code-fast-1`)

3. **Create `grok/xai/mod.rs`:**
   ```rust
   mod client;
   pub use client::XaiGrokClient;
   ```

4. **Update `grok/mod.rs`:**
   ```rust
   pub mod xai;
   pub mod types;
   pub mod builder;

   // Re-export for convenience
   pub use types::*;
   pub use builder::*;

   // Type alias for backwards compatibility
   #[deprecated(since = "0.2.0", note = "Use xai::XaiGrokClient explicitly")]
   pub type GrokClient = xai::XaiGrokClient;
   ```

5. **Keep shared modules:**
   - `grok/types.rs` - Shared request/response types
   - `grok/builder.rs` - Shared builder (works with any provider)

#### 1.2 Refactor GLM Module

**Same structure as Grok:**

1. **Create `nocodo-llm-sdk/src/glm/cerebras/` directory**

2. **Move and rename client:**
   - Move `glm/client.rs` → `glm/cerebras/client.rs`
   - Rename `GlmClient` → `CerebrasGlmClient`
   - Update base URL (already `https://api.cerebras.ai`)
   - Update model reference (already `glm-4.6`)

3. **Create `glm/cerebras/mod.rs`:**
   ```rust
   mod client;
   pub use client::CerebrasGlmClient;
   ```

4. **Update `glm/mod.rs`:**
   ```rust
   pub mod cerebras;
   pub mod types;
   pub mod builder;

   // Re-export for convenience
   pub use types::*;
   pub use builder::*;

   // Type alias for backwards compatibility
   #[deprecated(since = "0.2.0", note = "Use cerebras::CerebrasGlmClient explicitly")]
   pub type GlmClient = cerebras::CerebrasGlmClient;
   ```

5. **Keep shared modules:**
   - `glm/types.rs` - Shared request/response types
   - `glm/builder.rs` - Shared builder (works with any provider)

#### 1.3 Update Main Library

**Update `nocodo-llm-sdk/src/lib.rs`:**
```rust
// Update re-exports
pub use grok::{xai::XaiGrokClient, types::*, builder::*};
pub use glm::{cerebras::CerebrasGlmClient, types::*, builder::*};
```

#### 1.4 Verify Refactoring

**Test that existing code still compiles:**
```bash
cd nocodo-llm-sdk
cargo build
cargo test
```

**Run existing integration tests:**
```bash
XAI_API_KEY="..." cargo test --test grok_integration -- --ignored
CEREBRAS_API_KEY="..." cargo test --test glm_integration -- --ignored
```

---

### Phase 2: Implement Zen Providers (3 hours)

#### 2.1 Zen Grok Client

**Create `nocodo-llm-sdk/src/grok/zen/client.rs`:**

```rust
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use crate::{
    error::LlmError,
    grok::types::{GrokChatCompletionRequest, GrokChatCompletionResponse, GrokErrorResponse},
};

/// Zen provider for Grok (OpenCode Zen)
///
/// Free during beta, no authentication required for `grok-code` model.
pub struct ZenGrokClient {
    api_key: Option<String>,
    base_url: String,
    http_client: reqwest::Client,
}

impl ZenGrokClient {
    /// Create a new Zen Grok client (no API key required for free models)
    pub fn new() -> Result<Self, LlmError> {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300)) // 5 minute timeout
            .build()
            .map_err(|e| LlmError::Network { source: e })?;

        Ok(Self {
            api_key: None,
            base_url: "https://opencode.ai/zen".to_string(),
            http_client,
        })
    }

    /// Create a client with API key (for paid Zen models)
    pub fn with_api_key(api_key: impl Into<String>) -> Result<Self, LlmError> {
        let api_key = api_key.into();
        if api_key.is_empty() {
            return Err(LlmError::authentication("API key cannot be empty"));
        }

        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .map_err(|e| LlmError::Network { source: e })?;

        Ok(Self {
            api_key: Some(api_key),
            base_url: "https://opencode.ai/zen".to_string(),
            http_client,
        })
    }

    /// Set a custom base URL for the API
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Create a chat completion using the Zen Chat Completions API
    ///
    /// Default model is "grok-code" (free during beta)
    pub async fn create_chat_completion(
        &self,
        request: GrokChatCompletionRequest,
    ) -> Result<GrokChatCompletionResponse, LlmError> {
        let url = format!("{}/v1/chat/completions", self.base_url);

        let mut headers = HeaderMap::new();

        // Add authorization header if API key is provided
        if let Some(ref api_key) = self.api_key {
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {}", api_key))
                    .map_err(|e| LlmError::authentication(format!("Invalid API key format: {}", e)))?,
            );
        }

        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );

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

            // Try to parse as Grok error format
            if let Ok(error_response) = serde_json::from_str::<GrokErrorResponse>(&error_body) {
                return Err(LlmError::api(
                    status.as_u16(),
                    error_response.error.message,
                ));
            }

            return Err(LlmError::api(status.as_u16(), error_body));
        }

        let completion_response = response
            .json::<GrokChatCompletionResponse>()
            .await
            .map_err(|e| LlmError::Parse {
                message: format!("Failed to parse response: {}", e),
            })?;

        Ok(completion_response)
    }

    /// Get the provider name
    pub fn provider_name(&self) -> &str {
        "Zen (OpenCode)"
    }

    /// Get the default model for free access
    pub fn default_model() -> &'static str {
        "grok-code"
    }
}

impl Default for ZenGrokClient {
    fn default() -> Self {
        Self::new().expect("Failed to create default ZenGrokClient")
    }
}
```

**Create `nocodo-llm-sdk/src/grok/zen/mod.rs`:**
```rust
mod client;
pub use client::ZenGrokClient;
```

**Update `nocodo-llm-sdk/src/grok/mod.rs`:**
```rust
pub mod xai;
pub mod zen;  // NEW
pub mod types;
pub mod builder;

// Re-export for convenience
pub use types::*;
pub use builder::*;

// Type alias for backwards compatibility
#[deprecated(since = "0.2.0", note = "Use xai::XaiGrokClient explicitly")]
pub type GrokClient = xai::XaiGrokClient;
```

#### 2.2 Zen GLM Client (Big Pickle)

**Create `nocodo-llm-sdk/src/glm/zen/client.rs`:**

```rust
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use crate::{
    error::LlmError,
    glm::types::{GlmChatCompletionRequest, GlmChatCompletionResponse, GlmErrorResponse},
};

/// Zen provider for GLM (OpenCode Zen - "Big Pickle")
///
/// Free for limited time, no authentication required for `big-pickle` model.
/// Note: "Big Pickle" routes to GLM 4.6 on the backend.
pub struct ZenGlmClient {
    api_key: Option<String>,
    base_url: String,
    http_client: reqwest::Client,
}

impl ZenGlmClient {
    /// Create a new Zen GLM client (no API key required for free models)
    pub fn new() -> Result<Self, LlmError> {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300)) // 5 minute timeout
            .build()
            .map_err(|e| LlmError::Network { source: e })?;

        Ok(Self {
            api_key: None,
            base_url: "https://opencode.ai/zen".to_string(),
            http_client,
        })
    }

    /// Create a client with API key (for paid Zen models)
    pub fn with_api_key(api_key: impl Into<String>) -> Result<Self, LlmError> {
        let api_key = api_key.into();
        if api_key.is_empty() {
            return Err(LlmError::authentication("API key cannot be empty"));
        }

        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .map_err(|e| LlmError::Network { source: e })?;

        Ok(Self {
            api_key: Some(api_key),
            base_url: "https://opencode.ai/zen".to_string(),
            http_client,
        })
    }

    /// Set a custom base URL for the API
    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Create a chat completion using the Zen Chat Completions API
    ///
    /// Default model is "big-pickle" (free, limited time, routes to GLM 4.6)
    pub async fn create_chat_completion(
        &self,
        request: GlmChatCompletionRequest,
    ) -> Result<GlmChatCompletionResponse, LlmError> {
        let url = format!("{}/v1/chat/completions", self.base_url);

        let mut headers = HeaderMap::new();

        // Add authorization header if API key is provided
        if let Some(ref api_key) = self.api_key {
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&format!("Bearer {}", api_key))
                    .map_err(|e| LlmError::authentication(format!("Invalid API key format: {}", e)))?,
            );
        }

        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );

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

            // Try to parse as GLM error format
            if let Ok(error_response) = serde_json::from_str::<GlmErrorResponse>(&error_body) {
                return Err(LlmError::api(
                    status.as_u16(),
                    error_response.error.message,
                ));
            }

            return Err(LlmError::api(status.as_u16(), error_body));
        }

        let completion_response = response
            .json::<GlmChatCompletionResponse>()
            .await
            .map_err(|e| LlmError::Parse {
                message: format!("Failed to parse response: {}", e),
            })?;

        Ok(completion_response)
    }

    /// Get the provider name
    pub fn provider_name(&self) -> &str {
        "Zen (OpenCode)"
    }

    /// Get the default model for free access
    pub fn default_model() -> &'static str {
        "big-pickle"
    }
}

impl Default for ZenGlmClient {
    fn default() -> Self {
        Self::new().expect("Failed to create default ZenGlmClient")
    }
}
```

**Create `nocodo-llm-sdk/src/glm/zen/mod.rs`:**
```rust
mod client;
pub use client::ZenGlmClient;
```

**Update `nocodo-llm-sdk/src/glm/mod.rs`:**
```rust
pub mod cerebras;
pub mod zen;  // NEW
pub mod types;
pub mod builder;

// Re-export for convenience
pub use types::*;
pub use builder::*;

// Type alias for backwards compatibility
#[deprecated(since = "0.2.0", note = "Use cerebras::CerebrasGlmClient explicitly")]
pub type GlmClient = cerebras::CerebrasGlmClient;
```

#### 2.3 Update Main Library Exports

**Update `nocodo-llm-sdk/src/lib.rs`:**
```rust
// Add Zen provider exports
pub mod grok {
    pub use crate::grok::*;
}

pub mod glm {
    pub use crate::glm::*;
}

// Convenience re-exports
pub use grok::{
    xai::XaiGrokClient,
    zen::ZenGrokClient,
};

pub use glm::{
    cerebras::CerebrasGlmClient,
    zen::ZenGlmClient,
};
```

---

### Phase 3: Testing (2-3 hours)

#### 3.1 Integration Tests

**Create `nocodo-llm-sdk/tests/zen_grok_integration.rs`:**

```rust
use nocodo_llm_sdk::grok::{
    zen::ZenGrokClient,
    types::{GrokChatCompletionRequest, GrokMessage},
};

#[tokio::test]
async fn test_zen_grok_free_model() {
    // No API key required for free model!
    let client = ZenGrokClient::new().expect("Failed to create Zen Grok client");

    let request = GrokChatCompletionRequest {
        model: "grok-code".to_string(),
        messages: vec![GrokMessage {
            role: "user".to_string(),
            content: "What is 2+2? Answer in one word.".to_string(),
        }],
        max_tokens: Some(50),
        temperature: Some(0.7),
        ..Default::default()
    };

    let response = client
        .create_chat_completion(request)
        .await
        .expect("Failed to get response from Zen Grok");

    assert!(!response.choices.is_empty());
    assert!(response.choices[0].message.content.contains("4"));
    assert_eq!(response.model, "grok-code");
    println!("Response: {:?}", response);
}

#[tokio::test]
async fn test_zen_grok_with_api_key() {
    // Test with API key (for paid models in the future)
    let api_key = std::env::var("ZEN_API_KEY").ok();

    if api_key.is_none() {
        println!("ZEN_API_KEY not set, skipping authenticated test");
        return;
    }

    let client = ZenGrokClient::with_api_key(api_key.unwrap())
        .expect("Failed to create Zen Grok client with API key");

    let request = GrokChatCompletionRequest {
        model: "grok-code".to_string(),
        messages: vec![GrokMessage {
            role: "user".to_string(),
            content: "Hello from Zen!".to_string(),
        }],
        max_tokens: Some(100),
        ..Default::default()
    };

    let response = client
        .create_chat_completion(request)
        .await
        .expect("Failed to get response");

    assert!(!response.choices.is_empty());
    println!("Authenticated response: {:?}", response);
}
```

**Create `nocodo-llm-sdk/tests/zen_glm_integration.rs`:**

```rust
use nocodo_llm_sdk::glm::{
    zen::ZenGlmClient,
    types::{GlmChatCompletionRequest, GlmMessage},
};

#[tokio::test]
async fn test_zen_glm_big_pickle_free_model() {
    // No API key required for free Big Pickle model!
    let client = ZenGlmClient::new().expect("Failed to create Zen GLM client");

    let request = GlmChatCompletionRequest {
        model: "big-pickle".to_string(),
        messages: vec![GlmMessage {
            role: "user".to_string(),
            content: "What is 2+2? Answer in one word.".to_string(),
        }],
        max_tokens: Some(50),
        temperature: Some(0.7),
        ..Default::default()
    };

    let response = client
        .create_chat_completion(request)
        .await
        .expect("Failed to get response from Zen GLM (Big Pickle)");

    assert!(!response.choices.is_empty());
    assert!(response.choices[0].message.content.contains("4"));
    println!("Big Pickle response: {:?}", response);
}

#[tokio::test]
async fn test_zen_glm_with_api_key() {
    // Test with API key (for paid models)
    let api_key = std::env::var("ZEN_API_KEY").ok();

    if api_key.is_none() {
        println!("ZEN_API_KEY not set, skipping authenticated test");
        return;
    }

    let client = ZenGlmClient::with_api_key(api_key.unwrap())
        .expect("Failed to create Zen GLM client with API key");

    let request = GlmChatCompletionRequest {
        model: "big-pickle".to_string(),
        messages: vec![GlmMessage {
            role: "user".to_string(),
            content: "Hello from Zen GLM!".to_string(),
        }],
        max_tokens: Some(100),
        ..Default::default()
    };

    let response = client
        .create_chat_completion(request)
        .await
        .expect("Failed to get response");

    assert!(!response.choices.is_empty());
    println!("Authenticated GLM response: {:?}", response);
}
```

#### 3.2 Run Tests

**Run free model tests (no API key required!):**
```bash
cd nocodo-llm-sdk

# Test Zen Grok (free)
cargo test --test zen_grok_integration -- --nocapture

# Test Zen GLM Big Pickle (free)
cargo test --test zen_glm_integration -- --nocapture
```

**Run authenticated tests (optional):**
```bash
ZEN_API_KEY="your-zen-api-key" cargo test --test zen_grok_integration -- --nocapture
ZEN_API_KEY="your-zen-api-key" cargo test --test zen_glm_integration -- --nocapture
```

**Verify existing tests still pass:**
```bash
cargo test
XAI_API_KEY="..." cargo test --test grok_integration -- --ignored
CEREBRAS_API_KEY="..." cargo test --test glm_integration -- --ignored
```

---

### Phase 4: Documentation & Examples (1-2 hours)

#### 4.1 Create Examples

**Create `nocodo-llm-sdk/examples/zen_grok_free.rs`:**

```rust
//! Example: Using Zen provider for free Grok Code access
//!
//! Run with: cargo run --example zen_grok_free

use nocodo_llm_sdk::grok::{
    zen::ZenGrokClient,
    types::{GrokChatCompletionRequest, GrokMessage},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create Zen Grok client (no API key required for free model!)
    let client = ZenGrokClient::new()?;

    println!("Using Zen provider for free Grok Code access");
    println!("Provider: {}", client.provider_name());
    println!("Default model: {}\n", ZenGrokClient::default_model());

    // Create a simple completion request
    let request = GrokChatCompletionRequest {
        model: "grok-code".to_string(),
        messages: vec![GrokMessage {
            role: "user".to_string(),
            content: "Write a simple 'Hello, World!' program in Rust.".to_string(),
        }],
        max_tokens: Some(500),
        temperature: Some(0.7),
        ..Default::default()
    };

    println!("Sending request to Zen Grok...");
    let response = client.create_chat_completion(request).await?;

    println!("\n=== Response ===");
    println!("Model: {}", response.model);
    println!("Content:\n{}", response.choices[0].message.content);

    if let Some(usage) = response.usage {
        println!("\n=== Token Usage ===");
        println!("Prompt tokens: {}", usage.prompt_tokens);
        println!("Completion tokens: {}", usage.completion_tokens);
        println!("Total tokens: {}", usage.total_tokens);
    }

    Ok(())
}
```

**Create `nocodo-llm-sdk/examples/zen_big_pickle_free.rs`:**

```rust
//! Example: Using Zen provider for free GLM 4.6 access (Big Pickle)
//!
//! Run with: cargo run --example zen_big_pickle_free

use nocodo_llm_sdk::glm::{
    zen::ZenGlmClient,
    types::{GlmChatCompletionRequest, GlmMessage},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create Zen GLM client (no API key required for Big Pickle!)
    let client = ZenGlmClient::new()?;

    println!("Using Zen provider for free GLM 4.6 access (Big Pickle)");
    println!("Provider: {}", client.provider_name());
    println!("Default model: {}\n", ZenGlmClient::default_model());

    // Create a simple completion request
    let request = GlmChatCompletionRequest {
        model: "big-pickle".to_string(),
        messages: vec![GlmMessage {
            role: "user".to_string(),
            content: "Explain what a pickle is in programming, in 2 sentences.".to_string(),
        }],
        max_tokens: Some(200),
        temperature: Some(0.7),
        ..Default::default()
    };

    println!("Sending request to Zen GLM (Big Pickle)...");
    let response = client.create_chat_completion(request).await?;

    println!("\n=== Response ===");
    println!("Model: {}", response.model);
    println!("Content:\n{}", response.choices[0].message.content);

    if let Some(usage) = response.usage {
        println!("\n=== Token Usage ===");
        println!("Prompt tokens: {}", usage.prompt_tokens);
        println!("Completion tokens: {}", usage.completion_tokens);
        println!("Total tokens: {}", usage.total_tokens);
    }

    Ok(())
}
```

**Create `nocodo-llm-sdk/examples/provider_comparison.rs`:**

```rust
//! Example: Comparing different providers for the same models
//!
//! Run with:
//! XAI_API_KEY="..." CEREBRAS_API_KEY="..." cargo run --example provider_comparison

use nocodo_llm_sdk::{
    grok::{
        xai::XaiGrokClient,
        zen::ZenGrokClient,
        types::{GrokChatCompletionRequest, GrokMessage},
    },
    glm::{
        cerebras::CerebrasGlmClient,
        zen::ZenGlmClient,
        types::{GlmChatCompletionRequest, GlmMessage},
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Provider Comparison Demo ===\n");

    // Test Grok via different providers
    test_grok_providers().await?;

    println!("\n{}\n", "=".repeat(60));

    // Test GLM via different providers
    test_glm_providers().await?;

    Ok(())
}

async fn test_grok_providers() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- Grok Model via Different Providers ---\n");

    let prompt = "What is 2+2? Answer in one word.";

    // 1. Zen Grok (free)
    println!("1. Zen Provider (FREE):");
    let zen_client = ZenGrokClient::new()?;
    let request = GrokChatCompletionRequest {
        model: "grok-code".to_string(),
        messages: vec![GrokMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        }],
        max_tokens: Some(50),
        ..Default::default()
    };

    match zen_client.create_chat_completion(request).await {
        Ok(response) => {
            println!("   Model: {}", response.model);
            println!("   Response: {}", response.choices[0].message.content);
        }
        Err(e) => println!("   Error: {}", e),
    }

    // 2. xAI Grok (paid, requires API key)
    println!("\n2. xAI Provider (PAID):");
    if let Ok(api_key) = std::env::var("XAI_API_KEY") {
        let xai_client = XaiGrokClient::new(api_key)?;
        let request = GrokChatCompletionRequest {
            model: "grok-code-fast-1".to_string(),
            messages: vec![GrokMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            max_tokens: Some(50),
            ..Default::default()
        };

        match xai_client.create_chat_completion(request).await {
            Ok(response) => {
                println!("   Model: {}", response.model);
                println!("   Response: {}", response.choices[0].message.content);
            }
            Err(e) => println!("   Error: {}", e),
        }
    } else {
        println!("   Skipped (XAI_API_KEY not set)");
    }

    Ok(())
}

async fn test_glm_providers() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- GLM 4.6 via Different Providers ---\n");

    let prompt = "What is 2+2? Answer in one word.";

    // 1. Zen GLM (Big Pickle, free)
    println!("1. Zen Provider (Big Pickle, FREE):");
    let zen_client = ZenGlmClient::new()?;
    let request = GlmChatCompletionRequest {
        model: "big-pickle".to_string(),
        messages: vec![GlmMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        }],
        max_tokens: Some(50),
        ..Default::default()
    };

    match zen_client.create_chat_completion(request).await {
        Ok(response) => {
            println!("   Model: {}", response.model);
            println!("   Response: {}", response.choices[0].message.content);
        }
        Err(e) => println!("   Error: {}", e),
    }

    // 2. Cerebras GLM (paid, requires API key)
    println!("\n2. Cerebras Provider (PAID):");
    if let Ok(api_key) = std::env::var("CEREBRAS_API_KEY") {
        let cerebras_client = CerebrasGlmClient::new(api_key)?;
        let request = GlmChatCompletionRequest {
            model: "glm-4.6".to_string(),
            messages: vec![GlmMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            max_tokens: Some(50),
            ..Default::default()
        };

        match cerebras_client.create_chat_completion(request).await {
            Ok(response) => {
                println!("   Model: {}", response.model);
                println!("   Response: {}", response.choices[0].message.content);
            }
            Err(e) => println!("   Error: {}", e),
        }
    } else {
        println!("   Skipped (CEREBRAS_API_KEY not set)");
    }

    Ok(())
}
```

#### 4.2 Update README

**Update `nocodo-llm-sdk/README.md`** to include Zen provider usage:

Add section:

```markdown
## Multi-Provider Support

nocodo-llm-sdk supports accessing the same models via different providers, giving you flexibility in cost, performance, and availability.

### Grok Models

Access Grok via different providers:

#### Zen (Free)
```rust
use nocodo_llm_sdk::grok::zen::ZenGrokClient;

// No API key required for free model!
let client = ZenGrokClient::new()?;
let response = client.create_chat_completion(request).await?;
```

#### xAI (Paid)
```rust
use nocodo_llm_sdk::grok::xai::XaiGrokClient;

let client = XaiGrokClient::new("your-xai-api-key")?;
let response = client.create_chat_completion(request).await?;
```

### GLM 4.6 Models

Access GLM 4.6 via different providers:

#### Zen (Free - "Big Pickle")
```rust
use nocodo_llm_sdk::glm::zen::ZenGlmClient;

// No API key required for Big Pickle!
let client = ZenGlmClient::new()?;
let response = client.create_chat_completion(request).await?;
```

#### Cerebras (Paid)
```rust
use nocodo_llm_sdk::glm::cerebras::CerebrasGlmClient;

let client = CerebrasGlmClient::new("your-cerebras-api-key")?;
let response = client.create_chat_completion(request).await?;
```

### Provider Comparison

| Model | Zen (OpenCode) | Native Provider |
|-------|----------------|-----------------|
| **Grok** | `grok-code` (free) | `grok-code-fast-1` (xAI, paid) |
| **GLM 4.6** | `big-pickle` (free) | `glm-4.6` (Cerebras, paid) |
```

#### 4.3 Update Documentation Comments

Add provider comparison documentation to each client module's `mod.rs`.

---

## Success Criteria

### Implementation Complete When:

1. ✅ **Code compiles and all tests pass**
   ```bash
   cargo build
   cargo test
   cargo clippy
   ```

2. ✅ **Multi-provider structure works**
   - Existing xAI and Cerebras clients work (refactored with provider namespace)
   - New Zen clients work for both Grok and GLM
   - Backwards compatibility maintained via type aliases

3. ✅ **Free model access works (no API key required)**
   ```bash
   cargo test --test zen_grok_integration -- --nocapture
   cargo test --test zen_glm_integration -- --nocapture
   ```

4. ✅ **Examples run successfully**
   ```bash
   cargo run --example zen_grok_free
   cargo run --example zen_big_pickle_free
   cargo run --example provider_comparison
   ```

5. ✅ **Documentation is complete**
   - README updated with Zen provider usage
   - Examples demonstrate all providers
   - API docs explain provider selection

6. ✅ **Code quality maintained**
   - No clippy warnings
   - Consistent code style across providers
   - Well-tested (integration tests with real APIs)

---

## Provider Selection Guide

### When to Use Zen (OpenCode)

**Pros:**
- ✅ Free during beta (no API key required!)
- ✅ Easy to test (no account setup needed)
- ✅ Same OpenAI-compatible API
- ✅ Access to multiple models via single gateway

**Cons:**
- ⚠️ Beta service (may have rate limits)
- ⚠️ Free tier may be limited time
- ⚠️ Different model names than native providers

**Best for:**
- Testing and development
- Prototyping
- Learning the SDK
- Low-volume production use

### When to Use Native Providers (xAI, Cerebras)

**Pros:**
- ✅ Direct access to model providers
- ✅ Production-ready SLAs
- ✅ Native model names
- ✅ Full feature support

**Cons:**
- ⚠️ Requires API key and account
- ⚠️ Paid usage
- ⚠️ Separate accounts for each provider

**Best for:**
- Production deployments
- High-volume usage
- Enterprise applications
- When you need SLAs

---

## Migration Impact

### Breaking Changes

**None for existing users** - backwards compatibility maintained via type aliases:

```rust
// Old code still works (with deprecation warning)
use nocodo_llm_sdk::grok::GrokClient;
let client = GrokClient::new(api_key)?;

// New explicit provider usage (recommended)
use nocodo_llm_sdk::grok::xai::XaiGrokClient;
let client = XaiGrokClient::new(api_key)?;
```

### New Capabilities

Users can now:
1. Choose between providers for the same model
2. Use free Zen models for testing (no API key!)
3. Switch providers by changing client type (same API)
4. Compare provider responses side-by-side

---

## Future Enhancements

After Zen provider support is complete:

1. **More Zen models**
   - GPT models via Zen (`gpt-5.1`, `gpt-5.1-codex`)
   - Claude models via Zen (`claude-sonnet-4-5`)
   - Other Zen-supported models

2. **Provider abstraction**
   - Unified client that can switch providers at runtime
   - Provider fallback/retry logic

3. **Authentication abstraction**
   - Support for Zen authenticated models
   - Team API keys
   - Bring-your-own-key (BYOK)

4. **Provider-specific features**
   - Model listing via `/v1/models` endpoint
   - Usage tracking
   - Cost estimation

---

## Notes

### Key Design Decisions

1. **Provider-specific clients** (not adapter pattern)
   - Simpler implementation
   - Clear provider selection
   - Each client can have provider-specific optimizations

2. **Optional authentication** for Zen
   - `new()` - No API key (free models)
   - `with_api_key()` - With API key (paid models)
   - Future-proof for when Zen adds paid tiers

3. **Shared types and builders**
   - Request/response types are provider-agnostic
   - Builders work with any provider client
   - Minimizes code duplication

4. **Backwards compatibility**
   - Type aliases for old client names
   - Deprecation warnings guide users to new structure
   - No breaking changes in v0.2

### Testing Strategy

1. **Free model tests** (no API key needed!)
   - Can run in CI/CD
   - Great for integration testing
   - Validates multi-provider architecture

2. **Provider comparison tests**
   - Ensure same model works across providers
   - Verify consistent behavior
   - Validate error handling

3. **Existing test suite**
   - All existing tests must still pass
   - Validates backwards compatibility

---

## Timeline Estimate

- **Phase 1** (Refactoring): 2 hours
- **Phase 2** (Zen Implementation): 3 hours
- **Phase 3** (Testing): 2-3 hours
- **Phase 4** (Documentation): 1-2 hours

**Total**: 8-10 hours for complete implementation

**Fast track** (MVP): 4-5 hours
- Skip provider comparison example
- Minimal documentation updates
- Focus on working implementation

---

## References

- **Zen Documentation**: `external-docs/opencode_zen_docs.md`
- **Task Plan**: `tasks/nocodo-llm-sdk-creation.md` (v0.2 Multi-Provider Architecture)
- **Existing Implementations**:
  - `nocodo-llm-sdk/src/grok/client.rs` (xAI)
  - `nocodo-llm-sdk/src/glm/client.rs` (Cerebras)

---

## Next Steps

1. Review and approve this plan
2. Start Phase 1: Refactor existing clients
3. Implement Phase 2: Zen providers
4. Test with free models (Phase 3)
5. Add examples and docs (Phase 4)
6. Celebrate free LLM access! 🎉
