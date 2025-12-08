use nocodo_llm_sdk::glm::{client::GlmClient, types::GlmChatCompletionRequest};

#[tokio::test]
#[ignore] // Requires CEREBRAS_API_KEY environment variable
async fn test_glm_real_api_call() {
    let api_key = std::env::var("CEREBRAS_API_KEY").expect("CEREBRAS_API_KEY not set");
    let client = GlmClient::new(api_key).unwrap();

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
    };

    let response = client.create_chat_completion(request).await;
    if let Err(ref e) = response {
        eprintln!("Error: {:?}", e);
    }
    assert!(response.is_ok());
    let response = response.unwrap();

    assert_eq!(response.object, "chat.completion");
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
    let client = GlmClient::new("invalid-key").unwrap();

    let request = GlmChatCompletionRequest {
        model: "zai-glm-4.6".to_string(),
        messages: vec![nocodo_llm_sdk::glm::types::GlmMessage::user("Hello")],
        max_completion_tokens: Some(10),
        temperature: None,
        top_p: None,
        stop: None,
        stream: Some(false),
        seed: None,
    };

    let response = client.create_chat_completion(request).await;
    assert!(response.is_err());
    // Should be an authentication error
    match response.unwrap_err() {
        nocodo_llm_sdk::error::LlmError::Authentication { .. } => {}
        other => panic!("Expected authentication error, got: {:?}", other),
    }
}
