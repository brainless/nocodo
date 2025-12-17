use nocodo_llm_sdk::glm::zai::ZaiGlmClient;

#[tokio::test]
#[ignore] // Requires ZAI_API_KEY environment variable
async fn test_zai_glm_regular_mode() {
    let api_key = std::env::var("ZAI_API_KEY").expect("ZAI_API_KEY not set");
    let client = ZaiGlmClient::new(api_key).expect("Failed to create Z.AI GLM client");

    let result = client
        .message_builder()
        .model("glm-4.6")
        .max_tokens(100)
        .user_message("Say 'Hello, World!' and nothing else.")
        .temperature(0.0)
        .send()
        .await;

    // Check if authentication/balance error - skip test if account has no balance
    if let Err(e) = &result {
        let error_str = e.to_string();
        if error_str.contains("Insufficient balance") || error_str.contains("1113") {
            println!(
                "Skipping test - Z.AI account has insufficient balance: {}",
                error_str
            );
            return;
        }
    }

    let response = result.expect("Failed to get response from Z.AI GLM (regular mode)");

    assert!(!response.choices.is_empty());
    let response_text = response.choices[0].message.get_text();
    assert!(!response_text.trim().is_empty());
    assert!(
        response_text.contains("Hello") || response_text.contains("hello"),
        "Response should contain greeting: {}",
        response_text
    );
    println!("Z.AI GLM regular mode response: {:?}", response);
}

#[tokio::test]
#[ignore] // Requires ZAI_API_KEY environment variable
async fn test_zai_glm_coding_plan_mode() {
    let api_key = std::env::var("ZAI_API_KEY").expect("ZAI_API_KEY not set");
    let client = ZaiGlmClient::with_coding_plan(api_key, true)
        .expect("Failed to create Z.AI GLM client with coding plan");

    assert!(
        client.is_coding_plan(),
        "Client should be in coding plan mode"
    );

    let result = client
        .message_builder()
        .model("glm-4.6")
        .max_tokens(100)
        .user_message("Write a Python function to reverse a string.")
        .temperature(0.0)
        .send()
        .await;

    // Check if authentication/balance error - skip test if account has no balance
    if let Err(e) = &result {
        let error_str = e.to_string();
        if error_str.contains("Insufficient balance") || error_str.contains("1113") {
            println!(
                "Skipping test - Z.AI account has insufficient balance: {}",
                error_str
            );
            return;
        }
    }

    let response = result.expect("Failed to get response from Z.AI GLM (coding plan mode)");

    assert!(!response.choices.is_empty());
    let response_text = response.choices[0].message.get_text();
    assert!(!response_text.trim().is_empty());
    // Should contain Python code for reversing a string
    assert!(
        response_text.to_lowercase().contains("def")
            || response_text.to_lowercase().contains("function"),
        "Response should contain function definition: {}",
        response_text
    );
    println!("Z.AI GLM coding plan mode response: {:?}", response);
}

#[tokio::test]
#[ignore] // Requires ZAI_API_KEY environment variable
async fn test_zai_glm_json_response_format() {
    let api_key = std::env::var("ZAI_API_KEY").expect("ZAI_API_KEY not set");
    let client = ZaiGlmClient::new(api_key).expect("Failed to create Z.AI GLM client");

    let result = client
        .message_builder()
        .model("glm-4.6")
        .max_tokens(200)
        .user_message("Respond with a JSON object containing your name and version.")
        .temperature(0.0)
        .send()
        .await;

    // Check if authentication/balance error - skip test if account has no balance
    if let Err(e) = &result {
        let error_str = e.to_string();
        if error_str.contains("Insufficient balance") || error_str.contains("1113") {
            println!(
                "Skipping test - Z.AI account has insufficient balance: {}",
                error_str
            );
            return;
        }
    }

    let response = result.expect("Failed to get response from Z.AI GLM");

    assert!(!response.choices.is_empty());
    let response_text = response.choices[0].message.get_text();
    assert!(!response_text.trim().is_empty());

    // Try to parse as JSON (may not be perfect JSON, but should contain JSON-like structure)
    let trimmed = response_text.trim();
    let looks_like_json = trimmed.starts_with("{") && trimmed.ends_with("}");

    if looks_like_json {
        println!("Successfully got JSON response: {}", trimmed);
    } else {
        println!("Got text response that should contain JSON: {}", trimmed);
    }

    println!("Z.AI GLM JSON response format test: {:?}", response);
}

#[tokio::test]
async fn test_zai_glm_invalid_api_key() {
    let client = ZaiGlmClient::new("invalid-key").expect("Failed to create Z.AI GLM client");

    let result = client
        .message_builder()
        .model("glm-4.6")
        .max_tokens(10)
        .user_message("Hello")
        .send()
        .await;

    assert!(result.is_err());
    // Should be an authentication error
    match result.unwrap_err() {
        nocodo_llm_sdk::error::LlmError::Authentication { .. } => {
            // Expected
        }
        other => panic!("Expected authentication error, got: {:?}", other),
    }
}

#[tokio::test]
#[ignore] // Requires ZAI_API_KEY environment variable
async fn test_zai_glm_thinking_mode() {
    let api_key = std::env::var("ZAI_API_KEY").expect("ZAI_API_KEY not set");
    let client = ZaiGlmClient::new(api_key).expect("Failed to create Z.AI GLM client");

    // Test with a complex reasoning task
    let result = client
        .message_builder()
        .model("glm-4.6")
        .max_tokens(500)
        .system_message("You are a helpful assistant that thinks step by step.")
        .user_message("Explain how to solve classic 'Tower of Hanoi' problem with 3 disks.")
        .temperature(0.7)
        .send()
        .await;

    // Check if authentication/balance error - skip test if account has no balance
    if let Err(e) = &result {
        let error_str = e.to_string();
        if error_str.contains("Insufficient balance") || error_str.contains("1113") {
            println!(
                "Skipping test - Z.AI account has insufficient balance: {}",
                error_str
            );
            return;
        }
    }

    let response = result.expect("Failed to get response from Z.AI GLM");

    assert!(!response.choices.is_empty());
    let response_text = response.choices[0].message.get_text();
    assert!(!response_text.trim().is_empty());

    // Should contain explanation of Tower of Hanoi solution
    assert!(
        response_text.to_lowercase().contains("hanoi")
            || response_text.to_lowercase().contains("tower"),
        "Response should contain Tower of Hanoi explanation"
    );

    println!("Z.AI GLM thinking mode response: {}", response_text);
}
