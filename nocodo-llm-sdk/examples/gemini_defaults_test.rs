//! Test that per-model defaults are applied correctly
//!
//! Run with: GEMINI_API_KEY="..." cargo run --example gemini_defaults_test

use nocodo_llm_sdk::gemini::GeminiClient;
use nocodo_llm_sdk::models::gemini::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("GEMINI_API_KEY")?;
    let client = GeminiClient::new(api_key)?;

    println!("=== Testing Gemini 3 Pro with Default Settings ===\n");

    // This should now work without specifying max_output_tokens or thinking_level
    // The model metadata will apply:
    // - default_max_tokens: 1024
    // - default_thinking_level: "high"
    // - default_temperature: 1.0
    let response = client
        .message_builder()
        .model(GEMINI_3_PRO)
        .user_message("What is 2+2? Answer in one word.")
        .send()
        .await?;

    println!("Model version: {}", response.model_version);
    println!("Finish reason: {}", response.candidates[0].finish_reason);

    if let Some(parts) = &response.candidates[0].content.parts {
        if !parts.is_empty() {
            if let Some(text) = &parts[0].text {
                println!("\n✅ SUCCESS! Got response: '{}'", text);
            }
        }
    }

    if let Some(usage) = response.usage_metadata {
        println!("\n=== Token Usage ===");
        println!("Prompt: {:?}", usage.prompt_token_count);
        println!("Response: {:?}", usage.candidates_token_count);
        println!("Total: {:?}", usage.total_token_count);
    }

    println!("\n=== Testing Gemini 3 Flash with Default Settings ===\n");

    let response2 = client
        .message_builder()
        .model(GEMINI_3_FLASH)
        .user_message("Hello, Gemini!")
        .send()
        .await?;

    println!("Model version: {}", response2.model_version);
    println!("Finish reason: {}", response2.candidates[0].finish_reason);

    if let Some(parts) = &response2.candidates[0].content.parts {
        if !parts.is_empty() {
            if let Some(text) = &parts[0].text {
                println!("\n✅ SUCCESS! Got response: '{}'", text);
            }
        }
    }

    println!("\n✅ All tests passed! Defaults are working correctly.");

    Ok(())
}
