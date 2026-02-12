//! Simple tool calling example: Weather lookup
//!
//! Run with: OPENAI_API_KEY="..." cargo run --example tool_calling_weather
//!
//! Note: This example uses the Responses API. Tool calling with Responses API
//! is still being implemented.

use nocodo_llm_sdk::openai::OpenAIClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("OPENAI_API_KEY")?;
    let client = OpenAIClient::new(api_key)?;

    println!("🤖 Asking: What's the weather in Paris and Tokyo?");

    // Send request using Responses API
    let response = client
        .response_builder()
        .model("gpt-5-mini")
        .input("What's the typical weather in Paris and Tokyo?")
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

    println!("\n✅ Response:\n{}", text_content);

    Ok(())
}
