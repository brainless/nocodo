use nocodo_llm_sdk::gemini::{types::*, GeminiClient};
use nocodo_llm_sdk::models::gemini::*;

#[tokio::test]
#[ignore]
async fn test_gemini_3_pro_simple_completion() {
    let api_key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY required");

    let client = GeminiClient::new(api_key).expect("Failed to create Gemini client");

    let response = client
        .message_builder()
        .model(GEMINI_3_PRO)
        .user_message("What is 2+2? Answer in one word.")
        .max_output_tokens(50)
        .send()
        .await
        .expect("Failed to get response");

    assert!(!response.candidates.is_empty());
    let content = &response.candidates[0].content;

    let text = if let Some(parts) = &content.parts {
        parts[0].text.as_ref().expect("Expected text response")
    } else if let Some(text) = &content.text {
        text
    } else {
        panic!(
            "No text found in response. This might indicate API access issues with Gemini 3 Pro."
        );
    };

    assert!(text.contains("4") || text.to_lowercase().contains("four"));
}

#[tokio::test]
#[ignore]
async fn test_gemini_3_flash_with_thinking_level() {
    let api_key = std::env::var("GEMINI_API_KEY").expect("GEMINI_API_KEY required");

    let client = GeminiClient::new(api_key).unwrap();

    let response = client
        .message_builder()
        .model(GEMINI_3_FLASH)
        .thinking_level("low")
        .user_message("Hello, Gemini!")
        .max_output_tokens(100)
        .send()
        .await
        .expect("Failed to get response");

    assert!(!response.candidates.is_empty());
}

#[tokio::test]
#[ignore]
async fn test_gemini_with_system_instruction() {
    let api_key = std::env::var("GEMINI_API_KEY").unwrap();
    let client = GeminiClient::new(api_key).unwrap();

    let response = client
        .message_builder()
        .model(GEMINI_3_PRO)
        .system("You are a helpful coding assistant. Always respond concisely.")
        .user_message("Write a hello world function in Python")
        .max_output_tokens(500)
        .send()
        .await
        .expect("Failed");

    assert!(!response.candidates.is_empty());
}

#[tokio::test]
#[ignore]
async fn test_gemini_multi_turn_conversation() {
    let api_key = std::env::var("GEMINI_API_KEY").unwrap();
    let client = GeminiClient::new(api_key).unwrap();

    let response = client
        .message_builder()
        .model(GEMINI_3_FLASH)
        .user_message("Hi, what's your name?")
        .model_message("I'm Gemini, a large language model from Google.")
        .user_message("What can you help me with?")
        .max_output_tokens(200)
        .send()
        .await
        .expect("Failed");

    assert!(!response.candidates.is_empty());
}
