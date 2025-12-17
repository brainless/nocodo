//! Example: Using Zen provider for free Grok Code access
//!
//! Run with: cargo run --example zen_grok_free

use nocodo_llm_sdk::grok::{
    types::{GrokChatCompletionRequest, GrokMessage, GrokRole},
    zen::ZenGrokClient,
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
            tool_calls: None,
            tool_call_id: None,
        }],
        max_tokens: Some(500),
        temperature: Some(0.7),
        top_p: None,
        stop: None,
        stream: None,
        tools: None,
        tool_choice: None,
    };

    println!("Sending request to Zen Grok...");
    let response = client.create_chat_completion(request).await?;

    println!("\n=== Response ===");
    println!("Model: {}", response.model);
    println!("Content:\n{}", response.choices[0].message.content);

    println!("\n=== Token Usage ===");
    if let Some(usage) = &response.usage {
        println!("Prompt tokens: {}", usage.prompt_tokens);
        println!("Completion tokens: {}", usage.completion_tokens);
        println!("Total tokens: {}", usage.total_tokens);
    } else {
        println!("(Usage information not available)");
    }

    Ok(())
}
