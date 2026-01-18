//! Gemini 3 Flash example with fast responses
//!
//! Run with: GEMINI_API_KEY="..." cargo run --example gemini_flash

use nocodo_llm_sdk::gemini::GeminiClient;
use nocodo_llm_sdk::models::gemini::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("GEMINI_API_KEY")?;
    let client = GeminiClient::new(api_key)?;

    println!("=== Gemini 3 Flash (Fast Mode) ===\n");

    let response = client
        .message_builder()
        .model(GEMINI_3_FLASH)
        .thinking_level("low")
        .user_message("Explain what a REST API is in one sentence")
        .max_output_tokens(200)
        .send()
        .await?;

    println!("Model: {}", response.model_version);
    println!("\nResponse:");

    for candidate in &response.candidates {
        if let Some(parts) = &candidate.content.parts {
            for part in parts {
                if let Some(text) = &part.text {
                    println!("{}", text);
                }
            }
        }
    }

    Ok(())
}
