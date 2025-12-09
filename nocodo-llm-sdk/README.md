# Nocodo LLM SDK

A general-purpose LLM SDK for Rust with support for multiple LLM providers.

## Features

- **Type-safe**: Leverages Rust's type system for compile-time guarantees
- **Async-first**: Built with Tokio for high-performance async operations
- **Ergonomic**: Builder pattern for easy request construction
- **Comprehensive error handling**: Detailed error types with context
- **Tool/Function Calling**: Type-safe tool calling with automatic JSON Schema generation
- **Claude support**: Full Messages API implementation
- **Grok support**: xAI and Zen (free) Grok integration with OpenAI-compatible API
- **GLM support**: Cerebras GLM models with OpenAI-compatible API
- **Zen provider**: Free access to select models during beta
- **OpenAI support**: GPT models including GPT-5 with Chat Completions API
- **Multi-provider**: Same models available from different providers
- **Extensible**: Designed for easy addition of other LLM providers

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
nocodo-llm-sdk = { path = "../nocodo-llm-sdk" }  # For local development
# OR
nocodo-llm-sdk = "0.1"
```

## Quick Start

### Claude (Anthropic)

```rust
use nocodo_llm_sdk::claude::ClaudeClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client with your Anthropic API key
    let client = ClaudeClient::new("your-anthropic-api-key")?;

    // Build and send a message
    let response = client
        .message_builder()
        .model("claude-sonnet-4-5-20250929")
        .max_tokens(1024)
        .user_message("Hello, Claude! How are you today?")
        .send()
        .await?;

    println!("Response: {}", response.content[0].text);
    Ok(())
}
```

### Grok (xAI)

```rust
use nocodo_llm_sdk::grok::GrokClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client with your xAI API key
    let client = GrokClient::new("your-xai-api-key")?;

    // Build and send a message
    let response = client
        .message_builder()
        .model("grok-code-fast-1")
        .max_tokens(1024)
        .user_message("Write a Rust function to reverse a string.")
        .send()
        .await?;

    println!("Grok: {}", response.choices[0].message.content);
    println!(
        "Usage: {} input tokens, {} output tokens",
        response.usage.prompt_tokens, response.usage.completion_tokens
    );
    Ok(())
}
```

### GLM (Cerebras)

```rust
use nocodo_llm_sdk::glm::GlmClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client with your Cerebras API key
    let client = GlmClient::new("your-cerebras-api-key")?;

    // Build and send a message
    let response = client
        .message_builder()
        .model("zai-glm-4.6")
        .max_tokens(1024)
        .user_message("Explain quantum computing in simple terms.")
        .send()
        .await?;

    // GLM models may return both content and reasoning
    println!("GLM: {}", response.choices[0].message.get_text());
    println!(
        "Usage: {} input tokens, {} output tokens",
        response.usage.prompt_tokens, response.usage.completion_tokens
    );
    Ok(())
}
```

## Multi-Provider Support

nocodo-llm-sdk supports accessing the same models via different providers, giving you flexibility in cost, performance, and availability.

### Grok Models

Access Grok via different providers:

#### Zen (Free)
```rust
use nocodo_llm_sdk::grok::zen::ZenGrokClient;

// No API key required for free model!
let client = ZenGrokClient::new()?;
let response = client
    .message_builder()
    .model("grok-code")
    .max_tokens(1024)
    .user_message("Hello, Grok!")
    .send()
    .await?;

println!("Response: {}", response.choices[0].message.content);
```

#### xAI (Paid)
```rust
use nocodo_llm_sdk::grok::xai::XaiGrokClient;

let client = XaiGrokClient::new("your-xai-api-key")?;
let response = client
    .message_builder()
    .model("grok-code-fast-1")
    .max_tokens(1024)
    .user_message("Hello, Grok!")
    .send()
    .await?;

println!("Response: {}", response.choices[0].message.content);
```

### GLM 4.6 Models

Access GLM 4.6 via different providers:

#### Cerebras (Paid)
```rust
use nocodo_llm_sdk::glm::cerebras::CerebrasGlmClient;

let client = CerebrasGlmClient::new("your-cerebras-api-key")?;
let response = client
    .message_builder()
    .model("zai-glm-4.6")
    .max_tokens(1024)
    .user_message("Hello, GLM!")
    .send()
    .await?;

println!("Response: {}", response.choices[0].message.get_text());
```

### Provider Comparison

| Model | Zen (OpenCode) | Native Provider |
|-------|----------------|-----------------|
| **Grok** | `grok-code` (free) | `grok-code-fast-1` (xAI, paid) |
| **GLM 4.6** | `big-pickle` (free, limited time) | `zai-glm-4.6` (Cerebras, paid) |

### OpenAI (GPT-5)

```rust
use nocodo_llm_sdk::openai::OpenAIClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client with your OpenAI API key
    let client = OpenAIClient::new("your-openai-api-key")?;

    // Build and send a message
    let response = client
        .message_builder()
        .model("gpt-5.1")
        .max_completion_tokens(1024)
        .reasoning_effort("medium")  // For GPT-5 models
        .user_message("Write a Python function to check if a number is prime.")
        .send()
        .await?;

    println!("GPT: {}", response.choices[0].message.content);
    println!(
        "Usage: {} input tokens, {} output tokens",
        response.usage.prompt_tokens, response.usage.completion_tokens
    );
    Ok(())
}
```

## Advanced Usage

### Claude: Conversation with Multiple Messages

```rust
let response = client
    .message_builder()
    .model("claude-sonnet-4-5-20250929")
    .max_tokens(1024)
    .message("system", "You are a helpful assistant.")
    .message("user", "What's the capital of France?")
    .message("assistant", "The capital of France is Paris.")
    .message("user", "What's its population?")
    .send()
    .await?;
```

### Claude: Custom Parameters

```rust
let response = client
    .message_builder()
    .model("claude-sonnet-4-5-20250929")
    .max_tokens(2048)
    .temperature(0.7)
    .top_p(0.9)
    .system("You are an expert programmer.")
    .user_message("Write a Rust function to calculate fibonacci numbers.")
    .send()
    .await?;
```

### Grok: Multi-turn Conversation

```rust
let response = client
    .message_builder()
    .model("grok-code-fast-1")
    .max_tokens(1024)
    .system_message("You are an expert Rust developer.")
    .user_message("What's the best way to handle errors in Rust?")
    .assistant_message("The best way is to use Result<T, E> for recoverable errors and panic! for unrecoverable errors.")
    .user_message("Can you show me an example?")
    .send()
    .await?;
```

### Error Handling

```rust
match client
    .message_builder()
    .model("claude-sonnet-4-5-20250929")
    .max_tokens(100)
    .user_message("Hello")
    .send()
    .await
{
    Ok(response) => println!("Success: {}", response.content[0].text),
    Err(nocodo_llm_sdk::error::LlmError::AuthenticationError { message }) => {
        eprintln!("Authentication failed: {}", message);
    }
    Err(nocodo_llm_sdk::error::LlmError::RateLimitError { message, retry_after }) => {
        eprintln!("Rate limited: {} - retry after {:?}", message, retry_after);
    }
    Err(e) => eprintln!("Other error: {:?}", e),
}
```

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

### Advanced: Multi-Tool Agent

```rust
// Define multiple tools
let search_tool = Tool::from_type::<SearchParams>()
    .name("search")
    .description("Search the knowledge base")
    .build();

let calc_tool = Tool::from_type::<CalculateParams>()
    .name("calculate")
    .description("Evaluate mathematical expressions")
    .build();

// Use multiple tools with parallel execution
let response = client
    .message_builder()
    .user_message("Search for 'Rust' and calculate 123 * 456")
    .tools(vec![search_tool, calc_tool])
    .tool_choice(ToolChoice::Auto)
    .parallel_tool_calls(true)
    .send()
    .await?;
```

See `examples/tool_calling_*.rs` for complete examples.

### Tool Choice Options

- `ToolChoice::Auto`: Let the model decide whether to use tools
- `ToolChoice::Required`: Force the model to use at least one tool
- `ToolChoice::None`: Disable tool use
- `ToolChoice::Specific { name }`: Force a specific tool by name

## API Reference

### ClaudeClient

- `new(api_key: impl Into<String>) -> Result<Self>`: Create a new client
- `with_base_url(url: impl Into<String>) -> Self`: Set custom API base URL
- `message_builder() -> ClaudeMessageBuilder`: Start building a message request

### GrokClient

- `new(api_key: impl Into<String>) -> Result<Self>`: Create a new client
- `with_base_url(url: impl Into<String>) -> Self`: Set custom API base URL
- `message_builder() -> GrokMessageBuilder`: Start building a message request

### GlmClient

- `new(api_key: impl Into<String>) -> Result<Self>`: Create a new client
- `with_base_url(url: impl Into<String>) -> Self`: Set custom API base URL
- `message_builder() -> GlmMessageBuilder`: Start building a message request

### OpenAIClient

- `new(api_key: impl Into<String>) -> Result<Self>`: Create a new client
- `with_base_url(url: impl Into<String>) -> Self`: Set custom API base URL
- `message_builder() -> OpenAIMessageBuilder`: Start building a message request

### MessageBuilder (Claude, Grok & GLM)

- `model(model: impl Into<String>) -> Self`: Set the model
- `max_tokens(tokens: u32) -> Self`: Set maximum tokens
- `message(role: impl Into<String>, content: impl Into<String>) -> Self`: Add a message
- `user_message(content: impl Into<String>) -> Self`: Add a user message
- `assistant_message(content: impl Into<String>) -> Self`: Add an assistant message
- `system_message(content: impl Into<String>) -> Self`: Add a system message
- `temperature(temp: f32) -> Self`: Set temperature (0.0-2.0)
- `top_p(top_p: f32) -> Self`: Set top-p sampling
- `stop_sequences(sequences: Vec<String>) -> Self`: Set stop sequences
- `send() -> Result<Response>`: Send the request

Note: Claude also supports `system()` for system prompts, while Grok uses `system_message()`.

## Error Types

- `AuthenticationError`: Invalid API key
- `RateLimitError`: Rate limit exceeded (includes retry_after info)
- `InvalidRequestError`: Malformed request
- `ApiError`: API error with status code
- `NetworkError`: Network/connection issues
- `ParseError`: JSON parsing errors
- `InternalError`: Unexpected internal errors

## Testing

### Unit Tests

```bash
cargo test
```

### Integration Tests

Integration tests require valid API keys:

```bash
# Claude integration tests
ANTHROPIC_API_KEY=your-key-here cargo test --test claude_integration -- --ignored

# Grok integration tests
XAI_API_KEY=your-key-here cargo test --test grok_integration -- --ignored

# GLM integration tests
CEREBRAS_API_KEY=your-key-here cargo test --test glm_integration -- --ignored
```

## Examples

See the `examples/` directory for complete working examples:

- `simple_completion.rs`: Basic Claude usage
- `grok_completion.rs`: Basic Grok usage
- `glm_completion.rs`: Basic GLM usage
- `tool_calling_weather.rs`: Simple tool calling example
- `tool_calling_agent.rs`: Multi-tool agent example

## Development

This SDK is in active development. v0.1 provides Claude and Grok support with a clean, standalone API.

### Supported Providers

- **Claude** (Anthropic): Full Messages API implementation
  - Models: claude-sonnet-4-5, claude-opus-4-5, etc.
- **Grok** (xAI): OpenAI-compatible API
  - Models: grok-code-fast-1 (optimized for coding tasks)
- **GLM** (Cerebras): OpenAI-compatible API
  - Models: zai-glm-4.6 (GLM 4.6 model)

### Future Plans (v0.2+)

- Streaming responses
- Advanced features (vision, extended thinking)
- Multi-language bindings (Python, Node.js)

## License

Licensed under the same terms as the nocodo project.

## Contributing

Contributions welcome! Please see the main nocodo repository for contribution guidelines.