//! Debug Gemini 3 Pro response structure
//!
//! Run with: GEMINI_API_KEY="..." cargo run --example gemini_debug

use nocodo_llm_sdk::gemini::GeminiClient;
use nocodo_llm_sdk::models::gemini::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("GEMINI_API_KEY")?;
    let client = GeminiClient::new(api_key)?;

    println!("=== Testing Gemini 3 Pro ===\n");

    let response = client
        .message_builder()
        .model(GEMINI_3_PRO)
        .user_message("What is 2+2? Answer in one word.")
        .max_output_tokens(100) // Increased from 50
        .send()
        .await?;

    println!("Model version: {}", response.model_version);
    println!("Candidates count: {}", response.candidates.len());

    if !response.candidates.is_empty() {
        let candidate = &response.candidates[0];
        println!("\nCandidate:");
        println!("  Finish reason: {}", candidate.finish_reason);
        println!("  Index: {}", candidate.index);

        let content = &candidate.content;
        println!("\nContent structure:");
        println!("  Role: {:?}", content.role);
        println!("  Has parts: {}", content.parts.is_some());
        println!("  Has text: {}", content.text.is_some());

        if let Some(parts) = &content.parts {
            println!("\n  Parts count: {}", parts.len());
            for (i, part) in parts.iter().enumerate() {
                println!("\n  Part {}:", i);
                println!("    Has text: {}", part.text.is_some());
                println!("    Has inline_data: {}", part.inline_data.is_some());
                println!("    Has function_call: {}", part.function_call.is_some());
                println!(
                    "    Has function_response: {}",
                    part.function_response.is_some()
                );
                println!(
                    "    Has thought_signature: {}",
                    part.thought_signature.is_some()
                );

                if let Some(text) = &part.text {
                    println!("    Text content: '{}'", text);
                }
            }
        }

        if let Some(text) = &content.text {
            println!("\n  Direct text: '{}'", text);
        }
    }

    if let Some(usage) = response.usage_metadata {
        println!("\n=== Token Usage ===");
        println!("Prompt: {:?}", usage.prompt_token_count);
        println!("Response: {:?}", usage.candidates_token_count);
        println!("Total: {:?}", usage.total_token_count);
    }

    Ok(())
}
