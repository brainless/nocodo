use nocodo_llm_sdk::grok::{
    zen::ZenGrokClient,
    types::{GrokChatCompletionRequest, GrokMessage, GrokRole},
};

#[tokio::test]
async fn test_zen_grok_free_model() {
    // No API key required for free model!
    let client = ZenGrokClient::new().expect("Failed to create Zen Grok client");

    let request = GrokChatCompletionRequest {
        model: "grok-code".to_string(),
        messages: vec![GrokMessage {
            role: GrokRole::User,
            content: "What is 2+2? Answer in one word.".to_string(),
            tool_calls: None,
            tool_call_id: None,
        }],
        max_tokens: Some(50),
        temperature: Some(0.7),
        top_p: None,
        stop: None,
        stream: None,
        tools: None,
        tool_choice: None,
    };

    let response = client
        .create_chat_completion(request)
        .await
        .expect("Failed to get response from Zen Grok");

    assert!(!response.choices.is_empty());
    assert!(response.choices[0].message.content.contains("4"));
    assert_eq!(response.model, "grok-code");
    println!("Response: {:?}", response);
}

#[tokio::test]
async fn test_zen_grok_with_api_key() {
    // Test with API key (for paid models in the future)
    let api_key = std::env::var("ZEN_API_KEY").ok();

    if api_key.is_none() {
        println!("ZEN_API_KEY not set, skipping authenticated test");
        return;
    }

    let client = ZenGrokClient::with_api_key(api_key.unwrap())
        .expect("Failed to create Zen Grok client with API key");

    let request = GrokChatCompletionRequest {
        model: "grok-code".to_string(),
        messages: vec![GrokMessage {
            role: GrokRole::User,
            content: "Hello from Zen!".to_string(),
            tool_calls: None,
            tool_call_id: None,
        }],
        max_tokens: Some(100),
        temperature: None,
        top_p: None,
        stop: None,
        stream: None,
        tools: None,
        tool_choice: None,
    };

    let response = client
        .create_chat_completion(request)
        .await
        .expect("Failed to get response");

    assert!(!response.choices.is_empty());
    println!("Authenticated response: {:?}", response);
}