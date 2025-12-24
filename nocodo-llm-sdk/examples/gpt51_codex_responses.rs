use nocodo_llm_sdk::openai::OpenAIClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get API key from environment variable
    let api_key = std::env::var("OPENAI_API_KEY")
        .map_err(|_| "OPENAI_API_KEY environment variable not set")?;
    let client = OpenAIClient::new(api_key)?;

    // Build and send a response using the Responses API (GPT-5.1-Codex)
    let response = client
        .response_builder()
        .model("gpt-5.1-codex")
        .input("Write a Python function to calculate fibonacci numbers recursively")
        .send()
        .await?;

    println!("GPT-5.1-Codex Response:");

    // Extract and print the text content from the response
    for item in &response.output {
        if item.item_type == "message" {
            if let Some(content_blocks) = &item.content {
                for block in content_blocks {
                    if block.content_type == "output_text" {
                        println!("{}", block.text);
                    }
                }
            }
        }
    }

    println!(
        "Usage: {} input tokens, {} output tokens",
        response.usage.input_tokens.unwrap_or(0),
        response.usage.output_tokens.unwrap_or(0)
    );
    Ok(())
}
