# Nocodo LLM SDK

A general-purpose LLM SDK for Rust, starting with Claude (Anthropic) support.

## Features

- **Type-safe**: Leverages Rust's type system for compile-time guarantees
- **Async-first**: Built with Tokio for high-performance async operations
- **Ergonomic**: Builder pattern for easy request construction
- **Comprehensive error handling**: Detailed error types with context
- **Claude support**: Full Messages API implementation
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

## Advanced Usage

### Conversation with Multiple Messages

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

### Custom Parameters

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
- `message_builder() -> MessageBuilder`: Start building a message request

### MessageBuilder

- `model(model: impl Into<String>) -> Self`: Set the model
- `max_tokens(tokens: u32) -> Self`: Set maximum tokens
- `message(role: impl Into<String>, content: impl Into<String>) -> Self`: Add a message
- `user_message(content: impl Into<String>) -> Self`: Add a user message
- `assistant_message(content: impl Into<String>) -> Self`: Add an assistant message
- `system(content: impl Into<String>) -> Self`: Set system prompt
- `temperature(temp: f32) -> Self`: Set temperature
- `top_p(top_p: f32) -> Self`: Set top-p
- `stop_sequences(sequences: Vec<String>) -> Self`: Set stop sequences
- `send() -> Result<ClaudeMessageResponse>`: Send the request

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

Integration tests require a valid Anthropic API key:

```bash
ANTHROPIC_API_KEY=your-key-here cargo test --test integration
```

## Examples

See the `examples/` directory for complete working examples:

- `simple_completion.rs`: Basic usage
- More examples coming in future versions

## Development

This SDK is in active development. v0.1 focuses on Claude support with a clean, standalone API.

### Future Plans (v0.2+)

- Multi-provider support (OpenAI, xAI, etc.)
- Streaming responses
- Tool use / function calling
- Advanced features

## License

Licensed under the same terms as the nocodo project.

## Contributing

Contributions welcome! Please see the main nocodo repository for contribution guidelines.