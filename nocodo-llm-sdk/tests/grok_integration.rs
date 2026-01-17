use nocodo_llm_sdk::grok::xai::XaiGrokClient;

mod json_mode_helper;
use json_mode_helper::{expected_values, json_mode_prompt, validate_person_info_json};

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

    let client = XaiGrokClient::new(api_key).unwrap();
    let response = client
        .message_builder()
        .model("grok-code-fast-1")
        .max_tokens(100)
        .user_message("Say 'Hello, World!' and nothing else.")
        .send()
        .await;

    assert!(response.is_ok());
    let response = response.unwrap();
    assert_eq!(response.object, Some("chat.completion".to_string()));
    assert!(!response.choices.is_empty());
    assert!(!response.choices[0].message.content.is_empty());
    assert!(response.choices[0]
        .message
        .content
        .contains("Hello, World!"));
}

#[tokio::test]
#[ignore] // Run manually with API key
async fn test_grok_invalid_api_key() {
    let client = XaiGrokClient::new("invalid-key").unwrap();
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
        nocodo_llm_sdk::error::LlmError::Authentication { .. } => {}
        other => panic!("Expected authentication error, got: {:?}", other),
    }
}

#[tokio::test]
#[ignore]
async fn test_grok_json_mode() {
    skip_if_no_api_key();
    let api_key = get_api_key().unwrap();

    let client = nocodo_llm_sdk::grok::xai::XaiGrokClient::new(api_key).unwrap();
    let response = client
        .message_builder()
        .model("grok-beta")
        .max_tokens(100)
        .response_format(nocodo_llm_sdk::grok::types::GrokResponseFormat::json_object())
        .user_message(&json_mode_prompt())
        .send()
        .await;

    assert!(response.is_ok());
    let response = response.unwrap();
    let content = &response.choices[0].message.content;

    // Validate JSON structure
    validate_person_info_json(content).expect("Response should be valid JSON");

    // Parse and check expected values
    let json: serde_json::Value = serde_json::from_str(content).unwrap();
    let (exp_name, exp_age, exp_city, exp_occupation) = expected_values();

    assert_eq!(json["name"].as_str(), Some(exp_name.as_str()));
    assert_eq!(json["age"].as_i64(), Some(exp_age));
    assert_eq!(json["city"].as_str(), Some(exp_city.as_str()));
    assert_eq!(json["occupation"].as_str(), Some(exp_occupation.as_str()));
}
