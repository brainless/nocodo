//! Multi-tool agent example: Search and calculate
//!
//! Run with: OPENAI_API_KEY="..." cargo run --example tool_calling_agent
//!
//! Note: This example uses the Responses API. Tool calling with Responses API
//! is still being implemented.

use nocodo_llm_sdk::openai::OpenAIClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("OPENAI_API_KEY")?;
    let client = OpenAIClient::new(api_key)?;

    println!("🤖 Agent starting...\n");

    let response = client
        .response_builder()
        .model("gpt-5-mini")
        .input("Tell me about Rust programming and what is 123 * 456?")
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

    println!("\n✅ Agent completed:\n{}", text_content);

    Ok(())
}
