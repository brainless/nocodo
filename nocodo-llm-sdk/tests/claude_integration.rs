use nocodo_llm_sdk::claude::client::ClaudeClient;

// Integration tests require ANTHROPIC_API_KEY environment variable
// Run with: ANTHROPIC_API_KEY=sk-... cargo test --test claude_integration -- --ignored

fn get_api_key() -> Option<String> {
    std::env::var("ANTHROPIC_API_KEY").ok()
}

fn skip_if_no_api_key() {
    if get_api_key().is_none() {
        panic!("Skipping integration test - ANTHROPIC_API_KEY not set");
    }
}

#[tokio::test]
#[ignore] // Run manually with API key
async fn test_real_api_call() {
    skip_if_no_api_key();
    let api_key = get_api_key().unwrap();

    let client = ClaudeClient::new(api_key).unwrap();
    let response = client
        .message_builder()
        .model("claude-sonnet-4-5-20250929")
        .max_tokens(100)
        .user_message("Say 'Hello, World!' and nothing else.")
        .send()
        .await;

    assert!(response.is_ok());
    let response = response.unwrap();
    assert_eq!(response.role, nocodo_llm_sdk::claude::types::ClaudeRole::Assistant);
    assert!(!response.content.is_empty());
    match &response.content[0] {
        nocodo_llm_sdk::claude::types::ClaudeContentBlock::Text { text } => {
            assert!(text.contains("Hello, World!"));
        }
    }
}

#[tokio::test]
#[ignore] // Run manually with API key
async fn test_invalid_api_key() {
    let client = ClaudeClient::new("invalid-key").unwrap();
    let response = client
        .message_builder()
        .model("claude-sonnet-4-5-20250929")
        .max_tokens(100)
        .user_message("Hello")
        .send()
        .await;

    assert!(response.is_err());
    // Should be an authentication error
    match response.unwrap_err() {
        nocodo_llm_sdk::error::LlmError::Authentication { .. } => {},
        _ => panic!("Expected authentication error"),
    }
}