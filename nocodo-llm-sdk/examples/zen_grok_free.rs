//! Example: Using Zen provider for free Grok Code access
//!
//! Run with: cargo run --example zen_grok_free

use nocodo_llm_sdk::grok::{
    zen::ZenGrokClient,
    types::{GrokChatCompletionRequest, GrokMessage, GrokRole},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create Zen Grok client (no API key required for free model!)
    let client = ZenGrokClient::new()?;

    println!("Using Zen provider for free Grok Code access");
    println!("Provider: {}", client.provider_name());
    println!("Default model: {}\n", ZenGrokClient::default_model());

    // Create a simple completion request
    let request = GrokChatCompletionRequest {
        model: "grok-code".to_string(),
        messages: vec![GrokMessage {
            role: GrokRole::User,
            content: "Write a simple 'Hello, World!' program in Rust.".to_string(),
        }],
        max_tokens: Some(500),
        temperature: Some(0.7),
        top_p: None,
        stop: None,
        stream: None,
    };

    println!("Sending request to Zen Grok...");
    let response = client.create_chat_completion(request).await?;

    println!("\n=== Response ===");
    println!("Model: {}", response.model);
    println!("Content:\n{}", response.choices[0].message.content);

    println!("\n=== Token Usage ===");
    println!("Prompt tokens: {}", response.usage.prompt_tokens);
    println!("Completion tokens: {}", response.usage.completion_tokens);
    println!("Total tokens: {}", response.usage.total_tokens);

    Ok(())
}