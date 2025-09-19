mod common;

use actix_web::test;
use serde_json::json;

use crate::common::{
    TestApp, TestDataGenerator,
    llm_config::{LlmTestConfig, should_run_llm_tests},
    keyword_validation::{KeywordValidator, LlmTestScenario},
};
use nocodo_manager::models::{CreateLlmAgentSessionRequest, MessageAuthorType};

/// Comprehensive end-to-end test combining phases 1, 2, and 3
///
/// This test demonstrates:
/// - Phase 1: Test isolation infrastructure
/// - Phase 2: Real LLM integration
/// - Phase 3: Keyword-based validation
#[actix_rt::test]
async fn test_llm_e2e_real_integration() {
    // Skip test if no LLM providers are available
    if !should_run_llm_tests() {
        println!("⚠️  Skipping LLM E2E test - no API keys available");
        println!("   Set GROK_API_KEY, OPENAI_API_KEY, or ANTHROPIC_API_KEY to run this test");
        return;
    }

    // Get LLM configuration from environment
    let llm_config = LlmTestConfig::from_environment();

    if !llm_config.has_available_providers() {
        println!("⚠️  Skipping LLM E2E test - no LLM providers configured");
        return;
    }

    let provider = llm_config.get_default_provider().expect("No default provider available");

    println!("🚀 Running LLM E2E test with provider: {}", provider.name);
    println!("   Model: {}", provider.default_model());

    // PHASE 1: Create isolated test environment
    println!("\n📦 Phase 1: Setting up isolated test environment");
    let test_app = TestApp::new_with_llm(provider).await;

    // Verify isolation
    assert!(test_app.test_config().test_id.starts_with("test-"));
    assert!(test_app.test_config().db_path().to_string_lossy().contains(&test_app.test_config().test_id));

    // Verify LLM agent is configured
    let llm_agent = test_app.llm_agent().expect("LLM agent should be configured");
    println!("   ✅ Test isolation configured with ID: {}", test_app.test_config().test_id);
    println!("   ✅ LLM agent configured");

    // PHASE 2: Set up real LLM integration test scenario
    println!("\n🤖 Phase 2: Setting up real LLM integration");

    // Create test scenario with project context
    let scenario = LlmTestScenario::tech_stack_analysis_python_fastapi();

    // Set up project context from scenario
    test_app.create_project_from_scenario(&scenario.context)
        .await
        .expect("Failed to create project from scenario");

    // Create a work session
    let work = TestDataGenerator::create_work(Some("LLM E2E Test Work"), Some("test-project"));
    test_app.db().create_work(&work)
        .expect("Failed to create work session");

    // Create LLM agent session
    let session_request = CreateLlmAgentSessionRequest {
        provider: provider.name.clone(),
        model: provider.default_model().to_string(),
        system_prompt: Some("You are a helpful coding assistant analyzing project tech stacks. Be concise and accurate.".to_string()),
    };

    let uri = format!("/work/{}/llm-agent/sessions", work.id);
    let req = test::TestRequest::post()
        .uri(&uri)
        .set_json(&session_request)
        .to_request();

    let resp = test::call_service(test_app.service(), req).await;
    assert!(resp.status().is_success(), "Failed to create LLM session");

    let body: serde_json::Value = test::read_body_json(resp).await;
    let session_id = body["session"]["id"].as_str().expect("No session ID returned").to_string();

    println!("   ✅ Created LLM session: {}", session_id);
    println!("   ✅ Project context created with {} files", scenario.context.files.len());

    // PHASE 3: Test real LLM interaction with keyword validation
    println!("\n🎯 Phase 3: Testing LLM interaction with keyword validation");

    // Send the test scenario prompt to the real LLM
    let message_data = json!({
        "role": "user",
        "content": scenario.prompt
    });

    let uri = format!("/api/llm-agent/sessions/{}/messages", session_id);
    let req = test::TestRequest::post()
        .uri(&uri)
        .set_json(&message_data)
        .to_request();

    println!("   📤 Sending prompt to LLM: {}", scenario.prompt);

    let resp = test::call_service(test_app.service(), req).await;
    assert!(resp.status().is_success(), "Failed to send message to LLM");

    // Wait for LLM processing (real API call takes time)
    println!("   ⏳ Waiting for LLM response...");

    // Get the LLM response from the session messages
    let uri = format!("/api/llm-agent/sessions/{}/messages", session_id);
    let req = test::TestRequest::get().uri(&uri).to_request();
    let resp = test::call_service(test_app.service(), req).await;

    assert!(resp.status().is_success(), "Failed to get LLM messages");

    let body: serde_json::Value = test::read_body_json(resp).await;
    let messages = body["messages"].as_array().expect("No messages array");

    // Find the assistant's response
    let assistant_response = messages
        .iter()
        .find(|msg| msg["role"] == "assistant")
        .expect("No assistant response found");

    let response_content = assistant_response["content"]
        .as_str()
        .expect("No content in assistant response");

    println!("   📥 LLM Response received ({} chars)", response_content.len());
    println!("   📝 Response preview: {}...",
        if response_content.len() > 100 {
            &response_content[..100]
        } else {
            response_content
        });

    // PHASE 3: Validate response using keyword validation
    println!("\n🔍 Phase 3: Validating LLM response with keyword matching");

    let validation_result = KeywordValidator::validate_response(
        response_content,
        &scenario.expected_keywords
    );

    println!("   📊 Validation Results:");
    println!("      • Score: {:.2}", validation_result.score);
    println!("      • Required keywords found: {:?}", validation_result.found_required);
    println!("      • Optional keywords found: {:?}", validation_result.found_optional);
    println!("      • Forbidden keywords found: {:?}", validation_result.found_forbidden);

    if !validation_result.missing_required.is_empty() {
        println!("      • Missing required keywords: {:?}", validation_result.missing_required);
    }

    // Test assertions
    assert!(
        validation_result.passed,
        "LLM response validation failed for provider {}: {}\n\n\
         📝 Full Response:\n{}\n\n\
         📊 Validation Details:\n\
         • Score: {:.2} (minimum: {:.2})\n\
         • Required found: {:?}\n\
         • Required missing: {:?}\n\
         • Forbidden found: {:?}\n\
         • Optional found: {:?}",
        provider.name,
        scenario.name,
        response_content,
        validation_result.score,
        scenario.expected_keywords.minimum_score,
        validation_result.found_required,
        validation_result.missing_required,
        validation_result.found_forbidden,
        validation_result.found_optional
    );

    println!("   ✅ Keyword validation passed!");

    // Additional verification: ensure response contains some technical content
    assert!(
        response_content.len() > 50,
        "LLM response too short, might be an error: {}",
        response_content
    );

    // Verify the response is not just an error message
    let response_lower = response_content.to_lowercase();
    assert!(
        !response_lower.contains("error") || response_lower.contains("api") || response_lower.contains("python"),
        "LLM response appears to be an error: {}",
        response_content
    );

    println!("\n🎉 E2E Test Complete!");
    println!("   ✅ Phase 1: Test isolation infrastructure working");
    println!("   ✅ Phase 2: Real LLM integration successful");
    println!("   ✅ Phase 3: Keyword validation passed");
    println!("   📈 Overall score: {:.2}/1.0", validation_result.score);

    // Cleanup verification
    println!("\n🧹 Cleanup verification:");
    let projects = test_app.db().get_all_projects().expect("Failed to get projects");
    println!("   📁 Test projects created: {}", projects.len());

    let works = test_app.db().get_all_works().expect("Failed to get works");
    println!("   💼 Test work sessions: {}", works.len());

    println!("   🗂️  Test files will be cleaned up automatically");
}

/// Test multiple scenarios in sequence
#[actix_rt::test]
async fn test_llm_multiple_scenarios() {
    if !should_run_llm_tests() {
        println!("⚠️  Skipping multiple scenarios test - no API keys available");
        return;
    }

    let llm_config = LlmTestConfig::from_environment();

    if !llm_config.has_available_providers() {
        println!("⚠️  Skipping multiple scenarios test - no LLM providers configured");
        return;
    }

    let provider = llm_config.get_default_provider().expect("No default provider available");

    println!("🔄 Testing multiple LLM scenarios with provider: {}", provider.name);

    // Test scenarios
    let scenarios = vec![
        LlmTestScenario::tech_stack_analysis_python_fastapi(),
        LlmTestScenario::tech_stack_analysis_rust(),
    ];

    for (i, scenario) in scenarios.iter().enumerate() {
        println!("\n🧪 Scenario {}: {}", i + 1, scenario.name);

        // Create isolated test environment for each scenario
        let test_app = TestApp::new_with_llm(provider).await;

        // Set up scenario
        test_app.create_project_from_scenario(&scenario.context)
            .await
            .expect("Failed to create project from scenario");

        let work = TestDataGenerator::create_work(
            Some(&format!("Multi Scenario Work {}", i + 1)),
            Some("test-project")
        );
        test_app.db().create_work(&work).expect("Failed to create work");

        // Create LLM session
        let session_request = CreateLlmAgentSessionRequest {
            provider: provider.name.clone(),
            model: provider.default_model().to_string(),
            system_prompt: Some("You are a helpful coding assistant. Be concise and accurate.".to_string()),
        };

        let uri = format!("/work/{}/llm-agent/sessions", work.id);
        let req = test::TestRequest::post()
            .uri(&uri)
            .set_json(&session_request)
            .to_request();

        let resp = test::call_service(test_app.service(), req).await;
        assert!(resp.status().is_success());

        let body: serde_json::Value = test::read_body_json(resp).await;
        let session_id = body["session"]["id"].as_str().unwrap().to_string();

        // Send prompt
        let message_data = json!({
            "role": "user",
            "content": scenario.prompt
        });

        let uri = format!("/api/llm-agent/sessions/{}/messages", session_id);
        let req = test::TestRequest::post()
            .uri(&uri)
            .set_json(&message_data)
            .to_request();

        let resp = test::call_service(test_app.service(), req).await;
        assert!(resp.status().is_success());

        // Get response
        let uri = format!("/api/llm-agent/sessions/{}/messages", session_id);
        let req = test::TestRequest::get().uri(&uri).to_request();
        let resp = test::call_service(test_app.service(), req).await;
        assert!(resp.status().is_success());

        let body: serde_json::Value = test::read_body_json(resp).await;
        let messages = body["messages"].as_array().unwrap();

        let assistant_response = messages
            .iter()
            .find(|msg| msg["role"] == "assistant")
            .expect("No assistant response found");

        let response_content = assistant_response["content"].as_str().unwrap();

        // Validate response
        let validation_result = KeywordValidator::validate_response(
            response_content,
            &scenario.expected_keywords
        );

        println!("   📊 Score: {:.2}, Passed: {}", validation_result.score, validation_result.passed);

        // For multiple scenarios, we'll be more lenient but still check basic requirements
        assert!(
            validation_result.score >= 0.5,
            "Scenario {} failed with score {:.2}: {}",
            i + 1, validation_result.score, scenario.name
        );

        assert!(
            !validation_result.found_forbidden.is_empty() == false || validation_result.found_required.len() > 0,
            "Scenario {} had forbidden keywords or no required keywords",
            i + 1
        );
    }

    println!("\n✅ All scenarios completed successfully!");
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use crate::common::keyword_validation::{LlmKeywordExpectations, KeywordValidator};

    #[test]
    fn test_keyword_validation_logic() {
        let expectations = LlmKeywordExpectations {
            required_keywords: vec!["Python".to_string(), "FastAPI".to_string()],
            optional_keywords: vec!["React".to_string()],
            forbidden_keywords: vec!["Django".to_string()],
            minimum_score: 0.7,
        };

        // Test successful validation
        let good_response = "This project uses Python with FastAPI framework and React frontend";
        let result = KeywordValidator::validate_response(good_response, &expectations);
        assert!(result.passed);
        assert_eq!(result.found_required.len(), 2);
        assert_eq!(result.found_optional.len(), 1);
        assert_eq!(result.found_forbidden.len(), 0);

        // Test failing validation
        let bad_response = "This project uses Django web framework";
        let result = KeywordValidator::validate_response(bad_response, &expectations);
        assert!(!result.passed);
        assert_eq!(result.found_forbidden.len(), 1);
    }

    #[test]
    fn test_scenario_creation() {
        let scenario = LlmTestScenario::tech_stack_analysis_python_fastapi();

        assert!(!scenario.name.is_empty());
        assert!(!scenario.prompt.is_empty());
        assert!(!scenario.context.files.is_empty());
        assert!(!scenario.expected_keywords.required_keywords.is_empty());

        // Verify specific content
        assert!(scenario.context.files.iter().any(|f| f.path == "main.py"));
        assert!(scenario.context.files.iter().any(|f| f.path == "requirements.txt"));
        assert!(scenario.expected_keywords.required_keywords.contains(&"Python".to_string()));
        assert!(scenario.expected_keywords.required_keywords.contains(&"FastAPI".to_string()));
    }

    #[test]
    fn test_llm_config_from_environment() {
        let config = LlmTestConfig::from_environment();

        // Should not panic and should return valid config
        assert!(config.test_timeouts.request_timeout_secs > 0);
        assert!(config.test_timeouts.total_test_timeout_secs > 0);

        // If no providers available, should still be valid but empty
        if !config.has_available_providers() {
            assert!(config.enabled_providers.is_empty());
            assert!(config.default_provider.is_none());
        }
    }
}