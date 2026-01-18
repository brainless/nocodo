//! Simple Gemini 3 Pro example
//!
//! Run with: GEMINI_API_KEY="..." cargo run --example gemini_simple

use nocodo_llm_sdk::gemini::GeminiClient;
use nocodo_llm_sdk::models::gemini::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("GEMINI_API_KEY")?;
    let client = GeminiClient::new(api_key)?;

    println!("=== Gemini 3 Pro Example ===\n");

    let response = client
        .message_builder()
        .model(GEMINI_3_PRO)
        .system("You are a helpful coding assistant")
        .user_message("Write a simple Rust function to calculate fibonacci numbers")
        .thinking_level("high")
        .temperature(1.0)
        .max_output_tokens(1024)
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

    if let Some(usage) = response.usage_metadata {
        println!("\n=== Token Usage ===");
        if let Some(prompt) = usage.prompt_token_count {
            println!("Prompt: {}", prompt);
        }
        if let Some(response) = usage.candidates_token_count {
            println!("Response: {}", response);
        }
        if let Some(total) = usage.total_token_count {
            println!("Total: {}", total);
        }
    }

    Ok(())
}
