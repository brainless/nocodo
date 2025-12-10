# Task: Implement Tool/Function Calling for nocodo-llm-sdk

**Status**: Not Started
**Priority**: High
**Created**: 2025-12-09
**Estimated Effort**: 18-23 hours
**Target Version**: v0.2
**Related Tasks**: tasks/nocodo-llm-sdk-creation.md (v0.2 Multi-Provider Architecture)

---

## Overview

Add type-safe tool/function calling support to nocodo-llm-sdk using a derive-based approach with JSON Schema generation. This enables agentic workflows where LLMs can call external functions to gather information, perform actions, and make decisions.

**Scope**: All 4 current providers (Claude, OpenAI, Grok, Cerebras)

**Design Approach**: Derive-based with `schemars` for automatic JSON Schema generation from Rust types.

---

## Why Tool Calling?

### Use Cases

1. **Agentic Workflows**: LLMs can autonomously use tools to complete complex tasks
2. **Data Retrieval**: Fetch real-time information (weather, stock prices, database queries)
3. **Action Execution**: Perform operations (send emails, create tickets, update databases)
4. **Structured Output**: Enforce specific response formats via tools
5. **Function Orchestration**: Chain multiple function calls to solve problems

### Provider Support

All providers support tool/function calling with similar patterns:

| Provider | Support | Format | Special Features |
|----------|---------|--------|------------------|
| **Claude** | ✅ Full | `{name, description, input_schema}` | Server-side tools (web_search, bash) |
| **OpenAI** | ✅ Full | `{type: "function", function: {...}}` | Parallel calls, max 128 tools |
| **Grok (xAI)** | ✅ Full | Same as OpenAI | Server tools (web_search, code_execution, x_search) |
| **Cerebras (GLM)** | ✅ Full | Same as OpenAI | Parallel calls |

**Key Differences**:
- **Schema Format**: Claude uses `input_schema`, others use `function.parameters`
- **Tool Choice**: Different parameter names and values
- **Parallel Calls**: All support, with different opt-out mechanisms
- **Server Tools**: Claude and Grok have built-in autonomous tools

---

## Design: Derive-Based with schemars

### Core Concept

Use Rust's type system + JSON Schema generation for compile-time safety:

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use nocodo_llm_sdk::tools::Tool;

// 1. Define parameter struct with derive macros
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct WeatherParams {
    /// City name (e.g., "New York", "London")
    location: String,
    /// Temperature unit
    #[serde(default)]
    unit: TempUnit,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
enum TempUnit {
    Celsius,
    Fahrenheit,
}

impl Default for TempUnit {
    fn default() -> Self {
        Self::Celsius
    }
}

// 2. Create tool from type (auto-generates JSON Schema)
let weather_tool = Tool::from_type::<WeatherParams>()
    .name("get_weather")
    .description("Get current weather for a location")
    .build();

// 3. Use tool in request
let response = client
    .message_builder()
    .model("gpt-4o")
    .user_message("What's the weather in Paris?")
    .tool(weather_tool)
    .send()
    .await?;

// 4. Handle tool calls with type safety
if let Some(tool_calls) = response.tool_calls {
    for call in tool_calls {
        // Type-safe deserialization!
        let params: WeatherParams = call.parse_arguments()?;

        // Execute function
        let weather_data = fetch_weather(&params.location, params.unit).await?;

        // Return result to LLM
        let result = ToolResult::new(call.id(), weather_data)?;

        let final_response = client
            .message_builder()
            .continue_from(&response)
            .tool_result(result)
            .send()
            .await?;
    }
}
```

### Benefits

1. **Type Safety**: Parameters validated at compile time
2. **Auto JSON Schema**: Generated from Rust types via schemars
3. **Doc Comments**: Become schema descriptions automatically
4. **Serde Integration**: Respects `#[serde(...)]` attributes
5. **Idiomatic Rust**: Feels natural to Rust developers
6. **Future-Proof**: FFI-ready for Python/Node bindings

---

## Implementation Plan

### Phase 1: Core Types (3-4 hours)

**Goal**: Create unified tool types that work across all providers

#### 1.1 Add Dependency

```toml
# nocodo-llm-sdk/Cargo.toml
[dependencies]
schemars = { version = "0.8", features = ["preserve_order"] }
# Existing: serde, serde_json, tokio, reqwest, etc.
```

#### 1.2 Create Core Types

**File**: `nocodo-llm-sdk/src/tools/mod.rs`

```rust
use schemars::schema::RootSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::marker::PhantomData;

/// A tool that can be called by an LLM
#[derive(Debug, Clone)]
pub struct Tool {
    name: String,
    description: String,
    parameters: RootSchema,
}

impl Tool {
    /// Create a tool from a type that implements JsonSchema
    ///
    /// # Example
    /// ```rust
    /// #[derive(JsonSchema, Serialize, Deserialize)]
    /// struct MyParams {
    ///     query: String,
    ///     limit: u32,
    /// }
    ///
    /// let tool = Tool::from_type::<MyParams>()
    ///     .name("search")
    ///     .description("Search the database")
    ///     .build();
    /// ```
    pub fn from_type<T: schemars::JsonSchema>() -> ToolBuilder<T> {
        ToolBuilder {
            name: None,
            description: None,
            _phantom: PhantomData,
        }
    }

    /// Create a tool from raw JSON Schema (for FFI/future language bindings)
    #[cfg(feature = "ffi")]
    pub fn from_json_schema(
        name: String,
        description: String,
        schema: Value,
    ) -> Result<Self, crate::error::LlmError> {
        let parameters: RootSchema = serde_json::from_value(schema)
            .map_err(|e| crate::error::LlmError::InvalidToolSchema {
                message: e.to_string(),
            })?;

        Ok(Tool {
            name,
            description,
            parameters,
        })
    }

    // Getters
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn parameters(&self) -> &RootSchema {
        &self.parameters
    }
}

/// Builder for type-safe tools
pub struct ToolBuilder<T> {
    name: Option<String>,
    description: Option<String>,
    _phantom: PhantomData<T>,
}

impl<T: schemars::JsonSchema> ToolBuilder<T> {
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn build(self) -> Tool {
        let schema = schemars::schema_for!(T);
        Tool {
            name: self.name.expect("Tool name is required"),
            description: self.description.unwrap_or_default(),
            parameters: schema,
        }
    }
}

/// A tool call from the LLM
#[derive(Debug, Clone)]
pub struct ToolCall {
    id: String,
    name: String,
    arguments: Value,
}

impl ToolCall {
    pub fn new(id: String, name: String, arguments: Value) -> Self {
        Self { id, name, arguments }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Parse arguments into a strongly-typed struct
    ///
    /// # Example
    /// ```rust
    /// let params: WeatherParams = call.parse_arguments()?;
    /// println!("Location: {}", params.location);
    /// ```
    pub fn parse_arguments<T>(&self) -> Result<T, crate::error::LlmError>
    where
        T: for<'de> Deserialize<'de>,
    {
        serde_json::from_value(self.arguments.clone())
            .map_err(|e| crate::error::LlmError::ToolArgumentParse {
                tool_name: self.name.clone(),
                source: e,
            })
    }

    /// Get raw JSON arguments
    pub fn raw_arguments(&self) -> &Value {
        &self.arguments
    }
}

/// Tool execution result to send back to the LLM
#[derive(Debug, Clone)]
pub struct ToolResult {
    tool_call_id: String,
    content: String,
}

impl ToolResult {
    /// Create a tool result from any serializable value
    pub fn new<T: Serialize>(
        tool_call_id: impl Into<String>,
        content: T,
    ) -> Result<Self, crate::error::LlmError> {
        let content = serde_json::to_string(&content)
            .map_err(|e| crate::error::LlmError::Serialization { source: e })?;
        Ok(Self {
            tool_call_id: tool_call_id.into(),
            content,
        })
    }

    /// Create a tool result from a plain text string
    pub fn text(tool_call_id: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            tool_call_id: tool_call_id.into(),
            content: text.into(),
        }
    }

    pub fn tool_call_id(&self) -> &str {
        &self.tool_call_id
    }

    pub fn content(&self) -> &str {
        &self.content
    }
}

/// Tool choice strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ToolChoice {
    /// Let the model decide whether to use tools
    Auto,
    /// Force the model to use at least one tool
    Required,
    /// Disable tool use
    None,
    /// Force a specific tool by name
    Specific { name: String },
}

impl Default for ToolChoice {
    fn default() -> Self {
        Self::Auto
    }
}
```

#### 1.3 Update Error Types

**File**: `nocodo-llm-sdk/src/error.rs`

Add new error variants:

```rust
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    // ... existing variants ...

    #[error("Invalid tool schema: {message}")]
    InvalidToolSchema { message: String },

    #[error("Failed to parse tool arguments for {tool_name}: {source}")]
    ToolArgumentParse {
        tool_name: String,
        source: serde_json::Error,
    },

    #[error("Tool execution failed: {message}")]
    ToolExecutionFailed { message: String },
}
```

#### 1.4 Update Library Exports

**File**: `nocodo-llm-sdk/src/lib.rs`

```rust
pub mod tools;

// Re-export for convenience
pub use tools::{Tool, ToolCall, ToolChoice, ToolResult};
```

---

### Phase 2: Provider Integration (8-10 hours)

**Goal**: Add tool support to all 4 providers

#### 2.1 Provider Conversion Traits

**File**: `nocodo-llm-sdk/src/tools/provider.rs`

```rust
use serde::Serialize;
use serde_json::Value;
use super::Tool;

/// Convert unified Tool to provider-specific format
pub trait ProviderToolFormat {
    type ProviderTool: Serialize;

    fn to_provider_tool(tool: &Tool) -> Self::ProviderTool;
    fn to_provider_tool_choice(choice: &super::ToolChoice) -> Value;
}
```

#### 2.2 OpenAI Implementation (2 hours)

**Update**: `nocodo-llm-sdk/src/openai/types.rs`

```rust
use schemars::schema::RootSchema;
use serde::{Deserialize, Serialize};

/// OpenAI tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAITool {
    pub r#type: String,  // Always "function"
    pub function: OpenAIFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIFunction {
    pub name: String,
    pub description: String,
    pub parameters: RootSchema,
}

/// Tool call in OpenAI response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIToolCall {
    pub id: String,
    pub r#type: String,  // "function"
    pub function: OpenAIFunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIFunctionCall {
    pub name: String,
    pub arguments: String,  // JSON string
}

// Update OpenAIChatCompletionRequest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIChatCompletionRequest {
    pub model: String,
    pub messages: Vec<OpenAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    // NEW: Tool fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<OpenAITool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_tool_calls: Option<bool>,
}

// Update OpenAIChatCompletionResponse
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIChoice {
    pub index: u32,
    pub message: OpenAIMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIMessage {
    pub role: OpenAIRole,
    pub content: String,
    // NEW: Tool call fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<OpenAIToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}
```

**Create**: `nocodo-llm-sdk/src/openai/tools.rs`

```rust
use crate::tools::{provider::ProviderToolFormat, Tool, ToolChoice};
use super::types::{OpenAITool, OpenAIFunction};
use serde_json::{json, Value};

pub struct OpenAIToolFormat;

impl ProviderToolFormat for OpenAIToolFormat {
    type ProviderTool = OpenAITool;

    fn to_provider_tool(tool: &Tool) -> Self::ProviderTool {
        OpenAITool {
            r#type: "function".to_string(),
            function: OpenAIFunction {
                name: tool.name().to_string(),
                description: tool.description().to_string(),
                parameters: tool.parameters().clone(),
            },
        }
    }

    fn to_provider_tool_choice(choice: &ToolChoice) -> Value {
        match choice {
            ToolChoice::Auto => json!("auto"),
            ToolChoice::Required => json!("required"),
            ToolChoice::None => json!("none"),
            ToolChoice::Specific { name } => json!({
                "type": "function",
                "function": { "name": name }
            }),
        }
    }
}
```

**Update**: `nocodo-llm-sdk/src/openai/builder.rs`

```rust
use crate::tools::{Tool, ToolChoice, ToolResult};
use super::tools::OpenAIToolFormat;
use crate::tools::provider::ProviderToolFormat;

impl OpenAIMessageBuilder {
    /// Add a tool to the request
    pub fn tool(mut self, tool: Tool) -> Self {
        let tools = self.tools.get_or_insert_with(Vec::new);
        tools.push(OpenAIToolFormat::to_provider_tool(&tool));
        self
    }

    /// Add multiple tools
    pub fn tools(mut self, tools: Vec<Tool>) -> Self {
        for tool in tools {
            self = self.tool(tool);
        }
        self
    }

    /// Set tool choice strategy
    pub fn tool_choice(mut self, choice: ToolChoice) -> Self {
        self.tool_choice = Some(OpenAIToolFormat::to_provider_tool_choice(&choice));
        self
    }

    /// Enable or disable parallel tool calls (default: true)
    pub fn parallel_tool_calls(mut self, enabled: bool) -> Self {
        self.parallel_tool_calls = Some(enabled);
        self
    }

    /// Add a tool result to continue the conversation
    pub fn tool_result(mut self, result: ToolResult) -> Self {
        self.messages.push(OpenAIMessage {
            role: OpenAIRole::Tool,
            content: result.content().to_string(),
            tool_call_id: Some(result.tool_call_id().to_string()),
            tool_calls: None,
        });
        self
    }
}
```

**Update**: `nocodo-llm-sdk/src/openai/client.rs`

Add method to extract tool calls from response:

```rust
impl OpenAIChatCompletionResponse {
    /// Extract tool calls from the response
    pub fn tool_calls(&self) -> Option<Vec<crate::tools::ToolCall>> {
        self.choices.first()?.message.tool_calls.as_ref().map(|calls| {
            calls.iter().map(|call| {
                let arguments: serde_json::Value = serde_json::from_str(&call.function.arguments)
                    .unwrap_or(serde_json::Value::Null);

                crate::tools::ToolCall::new(
                    call.id.clone(),
                    call.function.name.clone(),
                    arguments,
                )
            }).collect()
        })
    }
}
```

#### 2.3 Claude Implementation (2.5 hours)

**Update**: `nocodo-llm-sdk/src/claude/types.rs`

```rust
use schemars::schema::RootSchema;

/// Claude tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeTool {
    pub name: String,
    pub description: String,
    pub input_schema: RootSchema,  // Note: Different field name than OpenAI
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControl>,
}

/// Tool use block in Claude response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClaudeContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

// Update ClaudeMessageRequest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeMessageRequest {
    pub model: String,
    pub max_tokens: u32,
    pub messages: Vec<ClaudeMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    // NEW: Tool fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ClaudeTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<serde_json::Value>,
}
```

**Create**: `nocodo-llm-sdk/src/claude/tools.rs`

```rust
use crate::tools::{provider::ProviderToolFormat, Tool, ToolChoice};
use super::types::ClaudeTool;
use serde_json::{json, Value};

pub struct ClaudeToolFormat;

impl ProviderToolFormat for ClaudeToolFormat {
    type ProviderTool = ClaudeTool;

    fn to_provider_tool(tool: &Tool) -> Self::ProviderTool {
        ClaudeTool {
            name: tool.name().to_string(),
            description: tool.description().to_string(),
            input_schema: tool.parameters().clone(),
            cache_control: None,
        }
    }

    fn to_provider_tool_choice(choice: &ToolChoice) -> Value {
        match choice {
            ToolChoice::Auto => json!({"type": "auto"}),
            ToolChoice::Required => json!({"type": "any"}),
            ToolChoice::None => json!({"type": "none"}),  // Disable tools
            ToolChoice::Specific { name } => json!({
                "type": "tool",
                "name": name
            }),
        }
    }
}
```

**Update builder and client** similar to OpenAI pattern.

#### 2.4 Grok Implementation (2 hours)

Grok uses OpenAI-compatible API, so most code can be shared:

```rust
// src/grok/tools.rs
pub use crate::openai::tools::OpenAIToolFormat as GrokToolFormat;
```

Add server-side tool support:

```rust
/// Built-in server-side tools for Grok
#[derive(Debug, Clone, Serialize)]
pub enum GrokServerTool {
    WebSearch,
    XSearch,
    CodeExecution,
    FileSearch,
}

impl GrokServerTool {
    pub fn to_tool_definition(&self) -> serde_json::Value {
        match self {
            Self::WebSearch => json!({"type": "web_search"}),
            Self::XSearch => json!({"type": "x_search"}),
            Self::CodeExecution => json!({"type": "code_execution"}),
            Self::FileSearch => json!({"type": "file_search"}),
        }
    }
}
```

#### 2.5 Cerebras (GLM) Implementation (1.5 hours)

Also OpenAI-compatible:

```rust
// src/glm/tools.rs
pub use crate::openai::tools::OpenAIToolFormat as GlmToolFormat;
```

---

### Phase 3: Testing (3-4 hours)

**Goal**: Comprehensive tests for all providers

#### 3.1 Unit Tests

**File**: `nocodo-llm-sdk/src/tools/tests.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, JsonSchema)]
    struct TestParams {
        query: String,
        limit: u32,
    }

    #[test]
    fn test_tool_creation() {
        let tool = Tool::from_type::<TestParams>()
            .name("search")
            .description("Search database")
            .build();

        assert_eq!(tool.name(), "search");
        assert_eq!(tool.description(), "Search database");
    }

    #[test]
    fn test_tool_call_parsing() {
        let args = serde_json::json!({
            "query": "rust",
            "limit": 10
        });

        let call = ToolCall::new(
            "call_123".to_string(),
            "search".to_string(),
            args,
        );

        let params: TestParams = call.parse_arguments().unwrap();
        assert_eq!(params.query, "rust");
        assert_eq!(params.limit, 10);
    }

    #[test]
    fn test_tool_result_creation() {
        let result = ToolResult::text("call_123", "Found 10 results");
        assert_eq!(result.tool_call_id(), "call_123");
        assert_eq!(result.content(), "Found 10 results");
    }
}
```

#### 3.2 Integration Tests

**File**: `nocodo-llm-sdk/tests/tool_calling_integration.rs`

```rust
use nocodo_llm_sdk::{
    openai::OpenAIClient,
    tools::{Tool, ToolChoice, ToolResult},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct WeatherParams {
    /// City name
    location: String,
}

#[tokio::test]
#[ignore] // Requires API key
async fn test_openai_tool_calling() {
    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");
    let client = OpenAIClient::new(api_key).unwrap();

    let weather_tool = Tool::from_type::<WeatherParams>()
        .name("get_weather")
        .description("Get current weather for a location")
        .build();

    let response = client
        .message_builder()
        .model("gpt-4o")
        .user_message("What's the weather in Tokyo?")
        .tool(weather_tool)
        .tool_choice(ToolChoice::Auto)
        .send()
        .await
        .unwrap();

    // Should trigger tool call
    assert!(response.tool_calls().is_some());

    let tool_calls = response.tool_calls().unwrap();
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0].name(), "get_weather");

    // Parse arguments
    let params: WeatherParams = tool_calls[0].parse_arguments().unwrap();
    assert!(params.location.to_lowercase().contains("tokyo"));

    // Return result
    let result = ToolResult::text(
        tool_calls[0].id(),
        "Sunny, 22°C"
    );

    let final_response = client
        .message_builder()
        .continue_from(&response)
        .tool_result(result)
        .send()
        .await
        .unwrap();

    assert!(!final_response.content().is_empty());
}
```

Similar tests for Claude, Grok, Cerebras.

#### 3.3 Test Runner Script

**File**: `nocodo-llm-sdk/scripts/test_tools.sh`

```bash
#!/bin/bash
set -e

echo "Running unit tests..."
cargo test

echo -e "\nRunning OpenAI integration tests..."
OPENAI_API_KEY="${OPENAI_API_KEY}" cargo test --test tool_calling_integration test_openai -- --ignored --nocapture

echo -e "\nRunning Claude integration tests..."
ANTHROPIC_API_KEY="${ANTHROPIC_API_KEY}" cargo test --test tool_calling_integration test_claude -- --ignored --nocapture

echo -e "\nRunning Grok integration tests..."
XAI_API_KEY="${XAI_API_KEY}" cargo test --test tool_calling_integration test_grok -- --ignored --nocapture

echo -e "\nRunning Cerebras integration tests..."
CEREBRAS_API_KEY="${CEREBRAS_API_KEY}" cargo test --test tool_calling_integration test_cerebras -- --ignored --nocapture

echo -e "\n✅ All tests passed!"
```

---

### Phase 4: Examples & Documentation (2-3 hours)

#### 4.1 Simple Weather Example

**File**: `nocodo-llm-sdk/examples/tool_calling_weather.rs`

```rust
//! Simple tool calling example: Weather lookup
//!
//! Run with: OPENAI_API_KEY="..." cargo run --example tool_calling_weather

use nocodo_llm_sdk::{
    openai::OpenAIClient,
    tools::{Tool, ToolChoice, ToolResult},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct WeatherParams {
    /// City name (e.g., "New York", "London", "Tokyo")
    location: String,
    /// Temperature unit
    #[serde(default)]
    unit: TempUnit,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
enum TempUnit {
    Celsius,
    Fahrenheit,
}

impl Default for TempUnit {
    fn default() -> Self {
        Self::Celsius
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("OPENAI_API_KEY")?;
    let client = OpenAIClient::new(api_key)?;

    // Define tool
    let weather_tool = Tool::from_type::<WeatherParams>()
        .name("get_weather")
        .description("Get current weather for a city")
        .build();

    println!("🤖 Asking: What's the weather in Paris and Tokyo?");

    // First request with tool
    let response = client
        .message_builder()
        .model("gpt-4o")
        .user_message("What's the weather in Paris and Tokyo?")
        .tool(weather_tool)
        .tool_choice(ToolChoice::Auto)
        .parallel_tool_calls(true)
        .send()
        .await?;

    // Handle tool calls
    if let Some(tool_calls) = response.tool_calls() {
        println!("\n📞 LLM requested {} tool call(s):", tool_calls.len());

        let mut results = Vec::new();

        for call in tool_calls {
            println!("\n  Tool: {}", call.name());

            // Type-safe parameter extraction
            let params: WeatherParams = call.parse_arguments()?;
            println!("  Location: {}", params.location);
            println!("  Unit: {:?}", params.unit);

            // Simulate weather API call
            let weather_data = format!(
                "{{\"temperature\": 22, \"condition\": \"Sunny\", \"location\": \"{}\"}}",
                params.location
            );

            results.push(ToolResult::text(call.id(), weather_data));
        }

        // Continue conversation with results
        let mut builder = client.message_builder().continue_from(&response);
        for result in results {
            builder = builder.tool_result(result);
        }

        let final_response = builder.send().await?;
        println!("\n✅ Final response:\n{}", final_response.content());
    } else {
        println!("\n✅ Direct response:\n{}", response.content());
    }

    Ok(())
}
```

#### 4.2 Multi-Tool Agent Example

**File**: `nocodo-llm-sdk/examples/tool_calling_agent.rs`

```rust
//! Multi-tool agent example: Search and calculate
//!
//! Run with: OPENAI_API_KEY="..." cargo run --example tool_calling_agent

use nocodo_llm_sdk::{
    openai::OpenAIClient,
    tools::{Tool, ToolChoice, ToolResult},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct SearchParams {
    /// Search query
    query: String,
    /// Maximum results (default: 10)
    #[serde(default = "default_limit")]
    limit: u32,
}

fn default_limit() -> u32 {
    10
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct CalculateParams {
    /// Mathematical expression to evaluate
    expression: String,
}

async fn execute_tool(call: &nocodo_llm_sdk::tools::ToolCall) -> Result<ToolResult, Box<dyn std::error::Error>> {
    match call.name() {
        "search" => {
            let params: SearchParams = call.parse_arguments()?;
            println!("🔍 Searching for: {} (limit: {})", params.query, params.limit);

            // Simulate search results
            let results = format!(
                "Found {} results for '{}':\n1. Result A\n2. Result B\n3. Result C",
                params.limit.min(3),
                params.query
            );
            Ok(ToolResult::text(call.id(), results))
        }
        "calculate" => {
            let params: CalculateParams = call.parse_arguments()?;
            println!("🧮 Calculating: {}", params.expression);

            // Simulate calculation (in reality, use a safe eval library)
            let result = "42"; // Placeholder
            Ok(ToolResult::text(call.id(), format!("Result: {}", result)))
        }
        _ => Err(format!("Unknown tool: {}", call.name()).into()),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("OPENAI_API_KEY")?;
    let client = OpenAIClient::new(api_key)?;

    // Define tools
    let search_tool = Tool::from_type::<SearchParams>()
        .name("search")
        .description("Search the knowledge base")
        .build();

    let calc_tool = Tool::from_type::<CalculateParams>()
        .name("calculate")
        .description("Evaluate mathematical expressions")
        .build();

    println!("🤖 Multi-tool agent starting...\n");

    let response = client
        .message_builder()
        .model("gpt-4o")
        .user_message("Search for 'Rust programming' and calculate 123 * 456")
        .tools(vec![search_tool, calc_tool])
        .tool_choice(ToolChoice::Auto)
        .parallel_tool_calls(true)
        .send()
        .await?;

    // Handle tool calls
    if let Some(tool_calls) = response.tool_calls() {
        println!("📞 Executing {} tool(s)...\n", tool_calls.len());

        let mut results = Vec::new();
        for call in tool_calls {
            let result = execute_tool(&call).await?;
            results.push(result);
        }

        // Continue conversation
        let mut builder = client.message_builder().continue_from(&response);
        for result in results {
            builder = builder.tool_result(result);
        }

        let final_response = builder.send().await?;
        println!("\n✅ Agent completed:\n{}", final_response.content());
    }

    Ok(())
}
```

#### 4.3 Documentation

**Update**: `nocodo-llm-sdk/README.md`

Add section:

```markdown
## Tool Calling (Function Calling)

Enable LLMs to call external functions with type-safe parameter extraction.

### Basic Example

```rust
use nocodo_llm_sdk::{openai::OpenAIClient, tools::Tool};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// Define parameter schema
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct WeatherParams {
    location: String,
    unit: String,
}

// Create tool
let tool = Tool::from_type::<WeatherParams>()
    .name("get_weather")
    .description("Get weather for a location")
    .build();

// Use tool
let response = client
    .message_builder()
    .user_message("What's the weather in NYC?")
    .tool(tool)
    .send()
    .await?;

// Handle tool calls
if let Some(calls) = response.tool_calls() {
    for call in calls {
        let params: WeatherParams = call.parse_arguments()?;
        // Execute your function...
    }
}
```

See `examples/tool_calling_*.rs` for complete examples.
```

---

## Future: Multi-Language Support

**Note**: This is planned for future versions but doesn't affect current Rust implementation.

### Python Bindings (v0.3+)

Users will be able to use Pydantic models:

```python
from pydantic import BaseModel
from nocodo_llm_sdk import OpenAIClient, tool_from_pydantic

class WeatherParams(BaseModel):
    location: str
    unit: Literal["celsius", "fahrenheit"] = "celsius"

# Converts Pydantic → JSON Schema → Rust Tool
tool = tool_from_pydantic(WeatherParams, name="get_weather", description="...")

client = OpenAIClient(api_key="...")
response = await client.chat.create(
    model="gpt-4o",
    messages=[...],
    tools=[tool]
)

# Type-safe parsing
params = WeatherParams.model_validate_json(response.tool_calls[0].arguments)
```

### Node.js/TypeScript Bindings (v0.4+)

Users will be able to use Zod schemas:

```typescript
import { z } from 'zod';
import { OpenAIClient, toolFromZod } from '@nocodo/llm-sdk';

const WeatherParamsSchema = z.object({
  location: z.string(),
  unit: z.enum(['celsius', 'fahrenheit']).default('celsius')
});

// Converts Zod → JSON Schema → Rust Tool
const tool = toolFromZod(WeatherParamsSchema, {
  name: 'get_weather',
  description: '...'
});

const client = new OpenAIClient({ apiKey: '...' });
const response = await client.chat.create({
  model: 'gpt-4o',
  messages: [...],
  tools: [tool]
});

// Type-safe parsing
const params = WeatherParamsSchema.parse(JSON.parse(response.toolCalls[0].arguments));
```

### Implementation Strategy

The current Rust implementation includes `Tool::from_json_schema()` method (behind `ffi` feature flag) to support future language bindings. Each language will:

1. Use its native schema generation (Pydantic, Zod, dry-schema)
2. Convert to JSON Schema
3. Pass to Rust core via FFI
4. Rust handles all API communication

This approach gives:
- ✅ Type safety in every language
- ✅ Idiomatic APIs per language
- ✅ Single source of truth (Rust core)
- ✅ Shared HTTP/retry/auth logic

---

## Success Criteria

### Phase 1 Complete When:

1. ✅ **All tests pass**:
   ```bash
   cargo test
   cargo clippy -- -D warnings
   cargo fmt -- --check
   ```

2. ✅ **All providers work**:
   ```bash
   OPENAI_API_KEY="..." cargo test --test tool_calling_integration test_openai -- --ignored
   ANTHROPIC_API_KEY="..." cargo test --test tool_calling_integration test_claude -- --ignored
   XAI_API_KEY="..." cargo test --test tool_calling_integration test_grok -- --ignored
   CEREBRAS_API_KEY="..." cargo test --test tool_calling_integration test_cerebras -- --ignored
   ```

3. ✅ **Examples run successfully**:
   ```bash
   cargo run --example tool_calling_weather
   cargo run --example tool_calling_agent
   ```

4. ✅ **Documentation is complete**:
   - API docs: `cargo doc --no-deps --open`
   - README updated with tool calling section
   - Examples are well-commented

5. ✅ **Type-safe API works**:
   ```rust
   let tool = Tool::from_type::<MyParams>().name("...").build();
   let params: MyParams = call.parse_arguments()?;
   ```

---

## Timeline Estimate

- **Phase 1**: Core types (3-4 hours)
- **Phase 2**: Provider integration (8-10 hours)
  - OpenAI: 2 hours
  - Claude: 2.5 hours
  - Grok: 2 hours
  - Cerebras: 1.5 hours
  - Buffer: 2 hours
- **Phase 3**: Testing (3-4 hours)
- **Phase 4**: Examples & docs (2-3 hours)

**Total**: 18-23 hours

---

## Dependencies

```toml
[dependencies]
# NEW
schemars = { version = "0.8", features = ["preserve_order"] }

# Existing
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
reqwest = { version = "0.11", features = ["json"] }
thiserror = "1.0"

# For FFI (future)
[features]
ffi = []
```

---

## Key Design Decisions

### 1. Use schemars for JSON Schema Generation

**Why**: Automatic, respects serde attributes, JSON Schema 2020-12 compliant (Claude requirement)

### 2. Derive-Based Approach (Not Manual or Proc Macro)

**Why**: Balance of type safety, boilerplate, and implementation complexity

### 3. Unified Tool Type Across Providers

**Why**: Users learn one API, we handle provider-specific conversions internally

### 4. Builder Pattern for Tools

**Why**: Ergonomic, discoverable via IDE autocomplete, validates at build time

### 5. FFI-Ready Design

**Why**: Future Python/Node bindings need `Tool::from_json_schema()` method

---

## Notes

### Provider-Specific Quirks

**Claude**:
- Uses `input_schema` instead of `parameters`
- `tool_choice.type` values: `auto`, `any`, `tool`, `none`
- Supports server-side tools (web_search, bash)

**OpenAI**:
- Wraps in `{type: "function", function: {...}}`
- Max 128 tools per request
- `parallel_tool_calls` parameter

**Grok**:
- OpenAI-compatible format
- Has server-side tools: web_search, x_search, code_execution
- `parallel_function_calling` parameter

**Cerebras**:
- OpenAI-compatible format
- Standard parallel tool calls

### Testing Strategy

1. **Unit tests**: Type conversions, schema generation
2. **Integration tests**: Real API calls per provider
3. **Example-based tests**: Run examples as part of CI

### Documentation Strategy

1. **Inline docs**: Comprehensive rustdoc comments
2. **Examples**: Simple (weather) → Complex (agent)
3. **README**: Quick start + link to examples
4. **Provider comparison**: Document differences

---

## Next Steps

1. **Review and approve this task**
2. **Phase 1**: Add schemars, create core types
3. **Phase 2**: Implement per provider (start with OpenAI)
4. **Phase 3**: Add integration tests
5. **Phase 4**: Write examples and docs
6. **Mark v0.2 stable** with tool calling support

---

## References

- Claude Messages API: `external-docs/claude_message_api.md`
- OpenAI Chat Completions API: `external-docs/openai_chat_completions_api_reference.md`
- xAI API: `external-docs/xai_api_reference.md`
- Cerebras API: `external-docs/cerebras_api_reference.md`
- schemars documentation: https://docs.rs/schemars/
- JSON Schema specification: https://json-schema.org/
