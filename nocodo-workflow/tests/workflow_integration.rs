mod helpers;

use nocodo_workflow::WorkflowResponse;

// Helper function to build system prompt with JSON schema
fn build_system_prompt() -> String {
    let schema = WorkflowResponse::json_schema();
    let schema_str = serde_json::to_string_pretty(&schema).unwrap();

    format!(
        r#"You are a helpful assistant and are helping the user clearly define an agent. You can respond only in JSON conforming to the given type. You can ask questions for clarification, ask for data access (API, URL, DB, etc.).

You must respond with valid JSON that matches this schema:

{}

Important:
- Your response MUST be valid JSON only
- Do not include any text before or after the JSON
- The response must match the WorkflowResponse schema exactly"#,
        schema_str
    )
}

#[tokio::test]
#[ignore] // Run via test-runner
async fn test_workflow_agent_definition() {
    let system_prompt = build_system_prompt();
    let user_prompt = "I would like an agent to collect available information about a company before any call with anyone in that company that is scheduled in my calendar";

    // Get response from selected provider
    let response_text = helpers::get_llm_response(&system_prompt, user_prompt)
        .await
        .expect("Failed to get LLM response");

    let provider = std::env::var("WORKFLOW_PROVIDER").unwrap_or_else(|_| "zen_glm".to_string());
    println!("Provider: {}", provider);
    println!("Raw response:\n{}", response_text);

    // Extract JSON from response (handles markdown code blocks)
    let json_text = helpers::extract_json(&response_text);

    // Validate that response is valid JSON
    let parsed_json: serde_json::Value = serde_json::from_str(json_text).unwrap_or_else(|e| {
        panic!(
            "Response is not valid JSON: {}. Response was: {}",
            e, json_text
        )
    });

    println!(
        "Parsed JSON:\n{}",
        serde_json::to_string_pretty(&parsed_json).unwrap()
    );

    // Validate that response matches WorkflowResponse schema
    let workflow_response: WorkflowResponse = serde_json::from_value(parsed_json.clone())
        .unwrap_or_else(|e| {
            panic!(
                "Response does not match WorkflowResponse schema: {}. JSON was: {}",
                e,
                serde_json::to_string_pretty(&parsed_json).unwrap()
            )
        });

    println!("Validated WorkflowResponse:");
    println!("  Questions: {:?}", workflow_response.questions);
    println!("  Inputs: {:?}", workflow_response.inputs);

    // Basic sanity checks
    assert!(
        !workflow_response.questions.is_empty() || !workflow_response.inputs.is_empty(),
        "Response should have either questions or inputs"
    );
}

#[tokio::test]
#[ignore] // Run via test-runner
async fn test_workflow_response_has_questions() {
    let system_prompt = build_system_prompt();
    let user_prompt = "I want an agent that will show the most important tasks of the week from my calendar and emails, but I am not sure how this will work. Please help";

    // Get response from selected provider
    let response_text = helpers::get_llm_response(&system_prompt, user_prompt)
        .await
        .expect("Failed to get LLM response");

    let provider = std::env::var("WORKFLOW_PROVIDER").unwrap_or_else(|_| "zen_glm".to_string());
    println!("Provider: {}", provider);
    println!("Raw response:\n{}", response_text);

    // Extract JSON from response
    let json_text = helpers::extract_json(&response_text);

    let workflow_response: WorkflowResponse = serde_json::from_str(json_text).unwrap_or_else(|e| {
        panic!(
            "Failed to parse response: {}. Response was: {}",
            e, json_text
        )
    });

    // Since the user is unsure, the LLM should ask clarifying questions
    assert!(
        !workflow_response.questions.is_empty(),
        "Response should have questions when user is unsure"
    );

    println!("Questions asked: {:?}", workflow_response.questions);
}
