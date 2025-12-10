use nocodo_llm_sdk::claude::ClaudeClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .expect("ANTHROPIC_API_KEY environment variable must be set");

    // Create client
    let client = ClaudeClient::new(api_key)?;

    // Build and send request
    let response = client
        .message_builder()
        .model("claude-sonnet-4-5-20250929")
        .max_tokens(1024)
        .user_message("Explain quantum computing in simple terms.")
        .send()
        .await?;

    // Print response
    match &response.content[0] {
        nocodo_llm_sdk::claude::types::ClaudeContentBlock::Text { text } => {
            println!("Claude: {}", text);
        }
        nocodo_llm_sdk::claude::types::ClaudeContentBlock::ToolUse { .. } => {
            println!("Claude: [Tool use content]");
        }
    }
    println!(
        "Usage: {} input tokens, {} output tokens",
        response.usage.input_tokens, response.usage.output_tokens
    );

    Ok(())
}
