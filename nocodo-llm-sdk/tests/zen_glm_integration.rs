use nocodo_llm_sdk::glm::{
    zen::ZenGlmClient,
    types::{GlmChatCompletionRequest, GlmMessage},
};

#[tokio::test]
async fn test_zen_glm_big_pickle_free_model() {
    // No API key required for free Big Pickle model!
    let client = ZenGlmClient::new().expect("Failed to create Zen GLM client");

    let request = GlmChatCompletionRequest {
        model: "big-pickle".to_string(),
        messages: vec![GlmMessage::user("What is 2+2? Answer in one word.")],
        max_completion_tokens: Some(50),
        temperature: Some(0.7),
        top_p: None,
        stop: None,
        stream: None,
        seed: None,
    };

    let response = client
        .create_chat_completion(request)
        .await
        .expect("Failed to get response from Zen GLM (Big Pickle)");

    assert!(!response.choices.is_empty());
    assert!(response.choices[0].message.get_text().contains("4"));
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
        messages: vec![GlmMessage::user("Hello from Zen GLM!")],
        max_completion_tokens: Some(100),
        temperature: None,
        top_p: None,
        stop: None,
        stream: None,
        seed: None,
    };

    let response = client
        .create_chat_completion(request)
        .await
        .expect("Failed to get response");

    assert!(!response.choices.is_empty());
    println!("Authenticated GLM response: {:?}", response);
}