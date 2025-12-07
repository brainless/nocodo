use nocodo_llm_sdk::grok::client::GrokClient;

// Integration tests require XAI_API_KEY environment variable
// Run with: XAI_API_KEY=sk-... cargo test --test grok_integration -- --ignored

fn get_api_key() -> Option<String> {
    std::env::var("XAI_API_KEY").ok()
}

fn skip_if_no_api_key() {
    if get_api_key().is_none() {
        panic!("Skipping integration test - XAI_API_KEY not set");
    }
}

#[tokio::test]
#[ignore] // Run manually with API key
async fn test_grok_real_api_call() {
    skip_if_no_api_key();
    let api_key = get_api_key().unwrap();

    let client = GrokClient::new(api_key).unwrap();
    let response = client
        .message_builder()
        .model("grok-code-fast-1")
        .max_tokens(100)
        .user_message("Say 'Hello, World!' and nothing else.")
        .send()
        .await;

    assert!(response.is_ok());
    let response = response.unwrap();
    assert_eq!(response.object, "chat.completion");
    assert!(!response.choices.is_empty());
    assert!(!response.choices[0].message.content.is_empty());
    assert!(response.choices[0].message.content.contains("Hello, World!"));
}

#[tokio::test]
#[ignore] // Run manually with API key
async fn test_grok_invalid_api_key() {
    let client = GrokClient::new("invalid-key").unwrap();
    let response = client
        .message_builder()
        .model("grok-code-fast-1")
        .max_tokens(100)
        .user_message("Hello")
        .send()
        .await;

    assert!(response.is_err());
    // Should be an authentication error
    match response.unwrap_err() {
        nocodo_llm_sdk::error::LlmError::Authentication { .. } => {},
        other => panic!("Expected authentication error, got: {:?}", other),
    }
}