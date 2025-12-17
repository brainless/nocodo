use nocodo_llm_sdk::openai::client::OpenAIClient;
use std::fs;

// Integration tests require OPENAI_API_KEY environment variable
// Run with: OPENAI_API_KEY=sk-... cargo test --test gpt_integration -- --ignored
// Or create a 'keys' file with the API key

fn get_api_key() -> Option<String> {
    // First try environment variable
    if let Ok(key) = std::env::var("OPENAI_API_KEY") {
        return Some(key);
    }

    // Then try reading from 'keys' file
    if let Ok(content) = fs::read_to_string("keys") {
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("OPENAI_API_KEY=") {
                // Extract the value between quotes
                if let Some(start) = line.find('"') {
                    if let Some(end) = line.rfind('"') {
                        if start < end {
                            let key = &line[start + 1..end];
                            if !key.is_empty() {
                                return Some(key.to_string());
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

fn skip_if_no_api_key() {
    if get_api_key().is_none() {
        panic!("Skipping integration test - OPENAI_API_KEY not set");
    }
}

#[tokio::test]
#[ignore] // Run manually with API key
async fn test_openai_real_api_call() {
    skip_if_no_api_key();
    let api_key = get_api_key().unwrap();

    let client = OpenAIClient::new(api_key).unwrap();
    let response = client
        .message_builder()
        .model("gpt-5.1")
        .max_completion_tokens(100)
        .user_message("Say 'Hello, World!' and nothing else.")
        .send()
        .await;

    assert!(response.is_ok());
    let response = response.unwrap();
    assert_eq!(response.object, "chat.completion");
    assert!(!response.choices.is_empty());
    assert!(!response.choices[0].message.content.is_empty());
    assert!(response.choices[0]
        .message
        .content
        .to_lowercase()
        .contains("hello"));
}

#[tokio::test]
#[ignore] // Run manually with API key
async fn test_openai_invalid_api_key() {
    let client = OpenAIClient::new("invalid-key").unwrap();
    let response = client
        .message_builder()
        .model("gpt-5.1")
        .max_completion_tokens(100)
        .user_message("Hello")
        .send()
        .await;

    assert!(response.is_err());
    // Should be an authentication error
    match response.unwrap_err() {
        nocodo_llm_sdk::error::LlmError::Authentication { .. } => {}
        other => panic!("Expected authentication error, got: {:?}", other),
    }
}

#[tokio::test]
#[ignore] // Run manually with API key
async fn test_openai_responses_api_gpt51_codex() {
    skip_if_no_api_key();
    let api_key = get_api_key().unwrap();

    let client = OpenAIClient::new(api_key).unwrap();
    let response = client
        .response_builder()
        .model("gpt-5.1-codex")
        .input("Write a simple Python function to calculate fibonacci numbers")
        .send()
        .await;

    assert!(response.is_ok());
    let response = response.unwrap();
    assert_eq!(response.object, "response");
    assert_eq!(response.status, "completed");
    assert!(!response.output.is_empty());

    // Check that we have at least one message with text content
    let has_text_content = response.output.iter().any(|item| {
        item.item_type == "message"
            && item.content.as_ref().map_or(false, |blocks| {
                blocks
                    .iter()
                    .any(|block| block.content_type == "output_text" && !block.text.is_empty())
            })
    });
    assert!(has_text_content);
}
