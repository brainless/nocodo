use nocodo_llm_sdk::openai::OpenAIClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment variable
    let api_key = std::env::var("OPENAI_API_KEY")
        .map_err(|_| "OPENAI_API_KEY environment variable not set")?;
    let client = OpenAIClient::new(api_key)?;

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
        response.usage.prompt_tokens.unwrap_or(0), response.usage.completion_tokens.unwrap_or(0)
    );
    Ok(())
}