use nocodo_llm_sdk::openai::OpenAIClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment variable
    let api_key = std::env::var("OPENAI_API_KEY")
        .map_err(|_| "OPENAI_API_KEY environment variable not set")?;
    let client = OpenAIClient::new(api_key)?;

    // Build and send a response request
    let response = client
        .response_builder()
        .model("gpt-5.1")
        .input("Write a Python function to check if a number is prime.")
        .send()
        .await?;

    // Extract text content from response
    let text_content: String = response
        .output
        .iter()
        .filter(|item| item.item_type == "message")
        .filter_map(|item| item.content.as_ref())
        .flatten()
        .filter(|block| block.content_type == "output_text")
        .map(|block| block.text.clone())
        .collect();

    println!("GPT: {}", text_content);
    println!(
        "Usage: {} input tokens, {} output tokens",
        response.usage.input_tokens.unwrap_or(0),
        response.usage.output_tokens.unwrap_or(0)
    );
    Ok(())
}
