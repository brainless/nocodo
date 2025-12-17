use nocodo_llm_sdk::glm::{
    types::{GlmChatCompletionRequest, GlmMessage},
    zen::ZenGlmClient,
};

#[tokio::test]
async fn test_zen_glm_big_pickle_free_model() {
    // No API key required for free Big Pickle model!
    let client = ZenGlmClient::new().expect("Failed to create Zen GLM client");

    let request = GlmChatCompletionRequest {
        model: "big-pickle".to_string(),
        messages: vec![GlmMessage::user("Say 'Hello, World!' and nothing else.")],
        max_completion_tokens: Some(50),
        temperature: Some(0.7),
        top_p: None,
        stop: None,
        stream: None,
        seed: None,
        tools: None,
        tool_choice: None,
    };

    let response = client
        .create_chat_completion(request)
        .await
        .expect("Failed to get response from Zen GLM (Big Pickle)");

    assert!(!response.choices.is_empty());
    let response_text = response.choices[0].message.get_text();
    assert!(!response_text.trim().is_empty());
    assert!(
        response_text.contains("Hello") || response_text.contains("hello"),
        "Response should contain greeting: {}",
        response_text
    );
    println!("Big Pickle response: {:?}", response);
}

#[tokio::test]
async fn test_zen_glm_with_api_key() {
    // Test with API key (for paid models)
    let api_key = std::env::var("ZEN_API_KEY").ok();

    if api_key.is_none() {
        println!("ZEN_API_KEY not set, skipping authenticated test");
        return;
    }

    let client = ZenGlmClient::with_api_key(api_key.unwrap())
        .expect("Failed to create Zen GLM client with API key");

    let request = GlmChatCompletionRequest {
        model: "big-pickle".to_string(),
        messages: vec![GlmMessage::user("Say 'Hello, World!' and nothing else.")],
        max_completion_tokens: Some(100),
        temperature: None,
        top_p: None,
        stop: None,
        stream: None,
        seed: None,
        tools: None,
        tool_choice: None,
    };

    let response = client
        .create_chat_completion(request)
        .await
        .expect("Failed to get response");

    assert!(!response.choices.is_empty());
    let response_text = response.choices[0].message.get_text();
    assert!(
        response_text.contains("Hello") || response_text.contains("hello"),
        "Response should contain greeting: {}",
        response_text
    );
    println!("Authenticated GLM response: {:?}", response);
}
