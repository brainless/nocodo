use nocodo_llm_sdk::openai::client::OpenAIClient;
use std::fs;

mod json_mode_helper;
use json_mode_helper::{expected_values, json_mode_prompt, validate_person_info_json};

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
        .response_builder()
        .model("gpt-5.1")
        .input("Say 'Hello, World!' and nothing else.")
        .send()
        .await;

    assert!(response.is_ok());
    let response = response.unwrap();
    assert_eq!(response.object, "response");
    assert_eq!(response.status, "completed");
    assert!(!response.output.is_empty());

    // Check that we have text content
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

#[tokio::test]
#[ignore] // Run manually with API key
async fn test_openai_invalid_api_key() {
    let client = OpenAIClient::new("invalid-key").unwrap();
    let response = client
        .response_builder()
        .model("gpt-5.1")
        .input("Hello")
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

#[tokio::test]
#[ignore]
async fn test_openai_json_mode() {
    skip_if_no_api_key();
    let api_key = get_api_key().unwrap();

    let client = OpenAIClient::new(api_key).unwrap();
    let response = client
        .response_builder()
        .model("gpt-5-mini")
        .input(&json_mode_prompt())
        .send()
        .await;

    assert!(response.is_ok());
    let response = response.unwrap();

    // Extract text content from response
    let content = response
        .output
        .iter()
        .filter(|item| item.item_type == "message")
        .flat_map(|item| item.content.as_ref())
        .flat_map(|blocks| blocks.iter())
        .filter(|block| block.content_type == "output_text")
        .map(|block| block.text.clone())
        .collect::<String>();

    // Validate JSON structure
    validate_person_info_json(&content).expect("Response should be valid JSON");

    // Parse and check expected values
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    let (exp_name, exp_age, exp_city, exp_occupation) = expected_values();

    assert_eq!(json["name"].as_str(), Some(exp_name.as_str()));
    assert_eq!(json["age"].as_i64(), Some(exp_age));
    assert_eq!(json["city"].as_str(), Some(exp_city.as_str()));
    assert_eq!(json["occupation"].as_str(), Some(exp_occupation.as_str()));
}
