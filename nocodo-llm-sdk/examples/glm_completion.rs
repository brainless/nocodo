use nocodo_llm_sdk::glm::GlmClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment
    let api_key = std::env::var("CEREBRAS_API_KEY")
        .expect("CEREBRAS_API_KEY environment variable must be set");

    // Create client
    let client = GlmClient::new(api_key)?;

    // Build and send request
    let response = client
        .message_builder()
        .model("zai-glm-4.6")
        .max_tokens(1024)
        .user_message("Hello, GLM! Can you tell me about yourself?")
        .send()
        .await?;

    // Print response
    println!("GLM: {}", response.choices[0].message.get_text());
    println!(
        "Usage: {} input tokens, {} output tokens (total: {})",
        response.usage.prompt_tokens, response.usage.completion_tokens, response.usage.total_tokens
    );
    println!("Finish reason: {:?}", response.choices[0].finish_reason);

    Ok(())
}
