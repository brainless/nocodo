# Nocodo LLM SDK

A general-purpose LLM SDK for Rust with support for multiple LLM providers.

## Features

- **Type-safe**: Leverages Rust's type system for compile-time guarantees
- **Async-first**: Built with Tokio for high-performance async operations
- **Ergonomic**: Builder pattern for easy request construction
- **Comprehensive error handling**: Detailed error types with context
- **Claude support**: Full Messages API implementation
- **Grok support**: xAI Grok integration with OpenAI-compatible API
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

## API Reference

### ClaudeClient

- `new(api_key: impl Into<String>) -> Result<Self>`: Create a new client
- `with_base_url(url: impl Into<String>) -> Self`: Set custom API base URL
- `message_builder() -> ClaudeMessageBuilder`: Start building a message request

### GrokClient

- `new(api_key: impl Into<String>) -> Result<Self>`: Create a new client
- `with_base_url(url: impl Into<String>) -> Self`: Set custom API base URL
- `message_builder() -> GrokMessageBuilder`: Start building a message request

### MessageBuilder (Claude & Grok)

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
```

## Examples

See the `examples/` directory for complete working examples:

- `simple_completion.rs`: Basic Claude usage
- `grok_completion.rs`: Basic Grok usage
- More examples coming in future versions

## Development

This SDK is in active development. v0.1 provides Claude and Grok support with a clean, standalone API.

### Supported Providers

- **Claude** (Anthropic): Full Messages API implementation
  - Models: claude-sonnet-4-5, claude-opus-4-5, etc.
- **Grok** (xAI): OpenAI-compatible API
  - Models: grok-code-fast-1 (optimized for coding tasks)

### Future Plans (v0.2+)

- OpenAI support
- Streaming responses
- Tool use / function calling
- Advanced features (vision, extended thinking)

## License

Licensed under the same terms as the nocodo project.

## Contributing

Contributions welcome! Please see the main nocodo repository for contribution guidelines.