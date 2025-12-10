use nocodo_llm_sdk::XaiGrokClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment
    let api_key =
        std::env::var("XAI_API_KEY").expect("XAI_API_KEY environment variable must be set");

    // Create client
    let client = XaiGrokClient::new(api_key)?;

    // Build and send request
    let response = client
        .message_builder()
        .model("grok-code-fast-1")
        .max_tokens(1024)
        .user_message("Write a Rust function that reverses a string in place.")
        .send()
        .await?;

    // Print response
    println!("Grok: {}", response.choices[0].message.content);
    println!(
        "Usage: {} input tokens, {} output tokens (total: {})",
        response.usage.prompt_tokens, response.usage.completion_tokens, response.usage.total_tokens
    );
    println!("Finish reason: {:?}", response.choices[0].finish_reason);

    Ok(())
}
