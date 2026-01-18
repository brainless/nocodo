use nocodo_llm_sdk::glm::{cerebras::CerebrasGlmClient, types::GlmChatCompletionRequest};

mod json_mode_helper;
use json_mode_helper::{expected_values, json_mode_prompt, validate_person_info_json};

#[tokio::test]
#[ignore] // Requires CEREBRAS_API_KEY environment variable
async fn test_glm_real_api_call() {
    let api_key = std::env::var("CEREBRAS_API_KEY").expect("CEREBRAS_API_KEY not set");
    let client = CerebrasGlmClient::new(api_key).unwrap();

    let request = GlmChatCompletionRequest {
        model: "zai-glm-4.6".to_string(),
        messages: vec![nocodo_llm_sdk::glm::types::GlmMessage::user(
            "Say 'Hello, World!' and nothing else.",
        )],
        max_completion_tokens: Some(100),
        temperature: Some(0.0),
        top_p: None,
        stop: None,
        stream: Some(false),
        seed: Some(42),
        tools: None,
        tool_choice: None,
        response_format: None,
    };

    let response = client.create_chat_completion(request).await;
    if let Err(ref e) = response {
        eprintln!("Error: {:?}", e);
    }
    assert!(response.is_ok());
    let response = response.unwrap();

    assert_eq!(response.object, Some("chat.completion".to_string()));
    assert!(!response.choices.is_empty());
    let message_text = response.choices[0].message.get_text();
    assert!(!message_text.is_empty());
    // GLM may return "Hello, World!" in content or reasoning
    assert!(
        message_text.contains("Hello") || message_text.contains("hello"),
        "Response should contain greeting: {}",
        message_text
    );
}

#[tokio::test]
#[ignore] // Requires CEREBRAS_API_KEY environment variable
async fn test_glm_invalid_api_key() {
    let client = CerebrasGlmClient::new("invalid-key").unwrap();

    let request = GlmChatCompletionRequest {
        model: "zai-glm-4.6".to_string(),
        messages: vec![nocodo_llm_sdk::glm::types::GlmMessage::user("Hello")],
        max_completion_tokens: Some(10),
        temperature: None,
        top_p: None,
        stop: None,
        stream: Some(false),
        seed: None,
        tools: None,
        tool_choice: None,
        response_format: None,
    };

    let response = client.create_chat_completion(request).await;
    assert!(response.is_err());
    // Should be an authentication error
    match response.unwrap_err() {
        nocodo_llm_sdk::error::LlmError::Authentication { .. } => {}
        other => panic!("Expected authentication error, got: {:?}", other),
    }
}

#[tokio::test]
#[ignore]
async fn test_glm_json_mode() {
    let api_key = std::env::var("CEREBRAS_API_KEY").expect("CEREBRAS_API_KEY not set");

    let client = nocodo_llm_sdk::glm::cerebras::CerebrasGlmClient::new(api_key).unwrap();
    let response = client
        .message_builder()
        .model("llama-3.3-70b")
        .max_tokens(100)
        .response_format(nocodo_llm_sdk::glm::types::GlmResponseFormat::json_object())
        .user_message(&json_mode_prompt())
        .send()
        .await;

    assert!(response.is_ok());
    let response = response.unwrap();
    let content = response.choices[0].message.get_text();

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
