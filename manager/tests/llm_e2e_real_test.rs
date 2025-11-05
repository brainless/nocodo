mod common;

use actix_web::{test, web, App, HttpMessage};
use std::env;

use crate::common::{
    keyword_validation::{KeywordValidator, LlmTestScenario},
    llm_config::LlmTestConfig,
    TestApp,
};
use nocodo_manager::handlers;
use nocodo_manager::models::{
    AddMessageRequest, CreateAiSessionRequest, CreateWorkRequest, MessageAuthorType,
    MessageContentType,
};

/// Comprehensive end-to-end test combining phases 1, 2, and 3
///
/// This test demonstrates:
/// - Phase 1: Test isolation infrastructure
/// - Phase 2: Real LLM integration
/// - Phase 3: Keyword-based validation
#[actix_rt::test]
async fn test_llm_e2e_saleor() {
    // Get LLM configuration from environment and skip if no providers available
    let llm_config = LlmTestConfig::from_environment();
    if !llm_config.has_available_providers() {
        println!("⚠️  Skipping LLM E2E test - no API keys available");
        println!("   Set GROK_API_KEY, OPENAI_API_KEY, or ANTHROPIC_API_KEY to run this test");
        return;
    }

    let provider = llm_config
        .get_default_provider()
        .expect("No default provider available");

    // Get the requested model from environment or use the first available model
    let model = env::var("MODEL").unwrap_or_else(|_| provider.default_model().to_string());

    // Validate that the requested model is available for this provider
    if !provider.models.contains(&model) {
        println!(
            "❌ Error: Model '{}' not available for provider '{}'",
            model, provider.name
        );
        println!("   Available models: {:?}", provider.models);
        return;
    }

    println!("🚀 Running LLM E2E test with provider: {}", provider.name);
    println!("   Model: {}", model);

    // PHASE 1: Create isolated test environment
    println!("\n📦 Phase 1: Setting up isolated test environment");
    let test_app = TestApp::new_with_llm(provider).await;

    // Verify isolation
    assert!(test_app.test_config().test_id.starts_with("test-"));
    assert!(test_app
        .test_config()
        .db_path()
        .to_string_lossy()
        .contains(&test_app.test_config().test_id));

    // Verify LLM agent is configured
    let _llm_agent = test_app
        .llm_agent()
        .expect("LLM agent should be configured");
    println!(
        "   ✅ Test isolation configured with ID: {}",
        test_app.test_config().test_id
    );
    println!("   ✅ LLM agent configured");

    // PHASE 2: Set up real LLM integration test scenario
    println!("\n🤖 Phase 2: Setting up real LLM integration");

    // Create test scenario with project context
    let scenario = LlmTestScenario::tech_stack_analysis_saleor();

    // Set up project context from scenario
    let project_id = test_app
        .create_project_from_scenario(&scenario.context)
        .await
        .expect("Failed to create project from scenario");

    // Verify project was created
    let projects = test_app.db().get_all_projects().unwrap();
    println!("   📁 Found {} projects in database", projects.len());
    for p in &projects {
        println!("     - Project {}: {} (path: {})", p.id, p.name, p.path);
    }
    assert!(projects.iter().any(|p| p.id == project_id), "Project {} not found in database", project_id);

    // Create a test user in the database
    let test_user = nocodo_manager::models::User {
        id: 1,
        username: "test_user".to_string(),
        email: "test@example.com".to_string(),
        password_hash: "test_hash".to_string(),
        is_active: true,
        created_at: chrono::Utc::now().timestamp(),
        updated_at: chrono::Utc::now().timestamp(),
    };
    test_app.db().create_user(&test_user).unwrap();
    println!("   👤 Created test user with ID: {}", test_user.id);

    // Read work title from prompts configuration
    let work_title = crate::common::keyword_validation::PromptsConfig::load_from_file(
        std::path::Path::new("prompts/default.toml")
    )
    .map(|config| config.tech_stack_analysis.prompt.clone())
    .unwrap_or_else(|_| "Tech Stack Analysis".to_string());

    // Follow the exact manager-web homepage form flow:
    // 1. Create the work with auto_start=true (this creates work, message, and AI session automatically)
    let work_request = CreateWorkRequest {
        title: work_title,
        project_id: Some(project_id),
        model: Some(model.clone()),
        auto_start: true,  // Auto-start creates work, message, and AI session
        tool_name: Some("llm-agent".to_string()),
    };

    let req = test::TestRequest::post()
        .uri("/api/work")
        .set_json(&work_request)
        .to_request();

    // Add mock user authentication for testing
    let mock_user = nocodo_manager::models::UserInfo {
        id: 1,
        username: "test_user".to_string(),
        email: "test@example.com".to_string(),
    };
    req.extensions_mut().insert(mock_user.clone());

    let service = test::init_service(App::new().app_data(test_app.app_state.clone()).route(
        "/api/work",
        web::post().to(nocodo_manager::handlers::create_work),
    ))
    .await;
    let resp = test::call_service(&service, req).await;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = test::read_body(resp).await;
        let body_str = String::from_utf8_lossy(&body);
        panic!("Failed to create work session. Status: {}, Body: {}", status, body_str);
    }

    let body: serde_json::Value = test::read_body_json(resp).await;
    let work_id = body["work"]["id"].as_i64().expect("No work ID returned");

    println!("   ✅ Created work session: {}", work_id);
    println!(
        "   ✅ Project context created from git repository: {}",
        scenario.context.git_repo
    );

    // Record start time for timeout calculation
    let start_time = std::time::Instant::now();

    // PHASE 3: Test real LLM interaction with keyword validation
    println!("\n🎯 Phase 3: Testing LLM interaction with keyword validation");
    println!("   📤 Prompt sent to AI session: {}", scenario.prompt);

    // Wait for AI session processing (real API call takes time)
    println!("   ⏳ Waiting for AI session response...");

    // Give the AI session some time to process (background task + real API call takes time)
    // In real scenarios this would be done via WebSocket, but for testing we poll the database directly
    let mut response_content = String::new();
    let mut printed_output_ids = std::collections::HashSet::new(); // Track which outputs we've printed

    loop {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        // Check for timeout (300 seconds from work creation)
        let elapsed = start_time.elapsed();
        if elapsed.as_secs() >= 300 {
            println!(
                "   ❌ Timeout waiting for AI response after {} seconds - no final text response received",
                elapsed.as_secs()
            );
            panic!("Test failed: AI did not provide a final text response within 300 seconds");
        }

        // Check AI session outputs using the manager API
        let ai_outputs = get_ai_outputs_for_work(&test_app, work_id)
            .await
            .expect("Failed to get AI session outputs");

        // Print any new outputs that haven't been printed yet
        let mut has_new_outputs = false;
        for output in &ai_outputs {
            if !printed_output_ids.contains(&output.id) && !output.content.is_empty() {
                let preview = if output.content.len() > 100 {
                    format!("{}...", &output.content[..100])
                } else {
                    output.content.clone()
                };
                println!("   📝 New AI output: {}", preview);
                printed_output_ids.insert(output.id);
                has_new_outputs = true;
            }
        }

        // Debug: Print all outputs for analysis after 25 seconds
        if elapsed.as_secs() >= 25 && !printed_output_ids.contains(&-1) {
            // Use -1 as a flag to indicate we've printed debug info
            printed_output_ids.insert(-1);
            println!(
                "   🔍 DEBUG: Total AI outputs after {} seconds: {}",
                elapsed.as_secs(),
                ai_outputs.len()
            );

        }

        // Check if we have a text response (not just tool calls)
        if let Some(output) = ai_outputs.iter().rev().find(|output| {
            !output.content.is_empty() &&
            !output.content.trim().starts_with("{\"text") && // Not structured assistant message with tool calls
            !output.content.trim().starts_with("{\"type") && // Not a tool call (with or without colon)
            !output.content.trim().starts_with("{\"files") && // Not a tool response
            !output.content.trim().starts_with("{\"content") && // Not a file content response
            !output.content.trim().starts_with("type") && // Not a malformed tool call
            // Only filter out if it's clearly a tool call/response, not just mentioning these words
            !(output.content.trim().contains("\"tool_call\"") || output.content.trim().contains("\"tool_use_id\""))
        }) {
            response_content = output.content.clone();
            println!(
                "   ✅ AI text response received after {} seconds",
                elapsed.as_secs()
            );
            break;
        }

        // Continue waiting for a final text response
        if has_new_outputs {
            println!(
                "   🔧 Found {} total outputs, waiting for final text response...",
                ai_outputs.len()
            );
        }

        // Only print waiting message if we didn't just print new outputs
        if !has_new_outputs {
            println!(
                "   ⏳ Waiting for AI response... ({}s elapsed)",
                elapsed.as_secs()
            );
        }
    }

    // Get the AI session outputs using the manager API
    let ai_outputs = get_ai_outputs_for_work(&test_app, work_id)
        .await
        .expect("Failed to get AI session outputs");

    println!("   🔍 Found {} AI session outputs:", ai_outputs.len());
    for (i, output) in ai_outputs.iter().enumerate() {
        println!(
            "      Output {}: content_preview={}",
            i + 1,
            output.content.chars().take(50).collect::<String>()
        );
    }

    // Find the response content from the outputs
    if response_content.is_empty() {
        // Look for the final assistant message (not tool calls)
        if let Some(output) = ai_outputs.iter().rev().find(|output| {
            !output.content.is_empty() &&
            !output.content.trim().starts_with("{\"type") && // Not a tool call (with or without colon)
            !output.content.trim().starts_with("{\"files") && // Not a tool response
            !output.content.trim().starts_with("{\"content") && // Not a file content response
            !output.content.trim().starts_with("type") && // Not a malformed tool call
            !output.content.trim().contains("\"type") && // Not containing tool call syntax
            !output.content.trim().contains("read_file") && // Not containing tool names
            !output.content.trim().contains("list_files") // Not containing tool names
        }) {
            response_content = output.content.clone();
        } else {
            panic!("Test failed: No final text response from AI found - only tool calls or responses were received");
        }
    }

    println!(
        "   📥 LLM Response received ({} chars)",
        response_content.len()
    );
    println!(
        "   📝 Response preview: {}...",
        if response_content.len() > 100 {
            &response_content[..100]
        } else {
            &response_content
        }
    );

    // PHASE 3: Validate response using keyword validation
    println!("\n🔍 Phase 3: Validating LLM response with keyword matching");

    let validation_result =
        KeywordValidator::validate_response(&response_content, &scenario.expected_keywords);

    println!("   📊 Validation Results:");
    println!("      • Score: {:.2}", validation_result.score);
    println!(
        "      • Required keywords found: {:?}",
        validation_result.found_required
    );
    println!(
        "      • Optional keywords found: {:?}",
        validation_result.found_optional
    );
    println!(
        "      • Forbidden keywords found: {:?}",
        validation_result.found_forbidden
    );

    if !validation_result.missing_required.is_empty() {
        println!(
            "      • Missing required keywords: {:?}",
            validation_result.missing_required
        );
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
        !response_lower.contains("error")
            || response_lower.contains("api")
            || response_lower.contains("python"),
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
    let projects = test_app
        .db()
        .get_all_projects()
        .expect("Failed to get projects");
    println!("   📁 Test projects created: {}", projects.len());

    let works = test_app.db().get_all_works().expect("Failed to get works");
    println!("   💼 Test work sessions: {}", works.len());

    println!("   🗂️  Test files will be cleaned up automatically");
}

/// Test multiple scenarios in sequence
#[actix_rt::test]
async fn test_llm_multiple_scenarios() {
    let llm_config = LlmTestConfig::from_environment();
    if !llm_config.has_available_providers() {
        println!("⚠️  Skipping multiple scenarios test - no API keys available");
        println!("   Set GROK_API_KEY, OPENAI_API_KEY, or ANTHROPIC_API_KEY to run this test");
        return;
    }

    let provider = llm_config
        .get_default_provider()
        .expect("No default provider available");

    println!(
        "🔄 Testing multiple LLM scenarios with provider: {}",
        provider.name
    );

    // Test scenarios
    let scenarios = [LlmTestScenario::tech_stack_analysis_saleor()];

    for (i, scenario) in scenarios.iter().enumerate() {
        println!("\n🧪 Scenario {}: {}", i + 1, scenario.name);

        // Create isolated test environment for each scenario
        let test_app = TestApp::new_with_llm(provider).await;

        // Set up scenario
        let project_id = test_app
            .create_project_from_scenario(&scenario.context)
            .await
            .expect("Failed to create project from scenario");

        // Follow the exact manager-web homepage form flow:
        // 1. Create the work
        let model = env::var("MODEL").unwrap_or_else(|_| provider.default_model().to_string());
        let work_request = CreateWorkRequest {
            title: format!("Multi Scenario Work {}", i + 1),
            project_id: Some(project_id),
            model: Some(model),
            auto_start: true,
            tool_name: Some("llm-agent".to_string()),
        };

        let req = test::TestRequest::post()
            .uri("/work")
            .set_json(&work_request)
            .to_request();

        let service = test::init_service(App::new().app_data(test_app.app_state.clone()).route(
            "/work",
        web::post().to(handlers::create_work),
        ))
        .await;
        let resp = test::call_service(&service, req).await;
        assert!(resp.status().is_success());

        let body: serde_json::Value = test::read_body_json(resp).await;
        let work_id = body["work"]["id"].as_i64().unwrap();

        // 2. Add the initial message
        let message_request = AddMessageRequest {
            content: scenario.prompt.clone(),
            content_type: MessageContentType::Text,
            author_type: MessageAuthorType::User,
            author_id: None,
        };

        let uri = format!("/work/{}/messages", work_id);
        let req = test::TestRequest::post()
            .uri(&uri)
            .set_json(&message_request)
            .to_request();

        let service = test::init_service(App::new().app_data(test_app.app_state.clone()).route(
            "/work/{work_id}/messages",
            web::post().to(nocodo_manager::handlers::add_message_to_work),
        ))
        .await;
        let resp = test::call_service(&service, req).await;
        assert!(resp.status().is_success());

        let body: serde_json::Value = test::read_body_json(resp).await;
        let message_id = body["message"]["id"].as_i64().unwrap();

        // 3. Create the AI session with llm-agent tool
        let ai_session_request = CreateAiSessionRequest {
            message_id: message_id.to_string(),
            tool_name: "llm-agent".to_string(),
        };

        let uri = format!("/work/{}/sessions", work_id);
        let req = test::TestRequest::post()
            .uri(&uri)
            .set_json(&ai_session_request)
            .to_request();

        let service = test::init_service(App::new().app_data(test_app.app_state.clone()).route(
            "/work/{work_id}/sessions",
            web::post().to(nocodo_manager::handlers::create_ai_session),
        ))
        .await;
        let resp = test::call_service(&service, req).await;
        assert!(resp.status().is_success());

        let body: serde_json::Value = test::read_body_json(resp).await;
        let _ai_session_id = body["session"]["id"].as_i64().unwrap();

        // Wait for AI session processing
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;

        // Get response from AI session outputs using manager API
        let ai_outputs = get_ai_outputs_for_work(&test_app, work_id)
            .await
            .expect("Failed to get AI session outputs");

        let response_content =
            if let Some(output) = ai_outputs.iter().find(|output| !output.content.is_empty()) {
                output.content.clone()
            } else {
                // If no outputs yet, wait a bit more
                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                let ai_outputs = get_ai_outputs_for_work(&test_app, work_id)
                    .await
                    .expect("Failed to get AI session outputs");
                ai_outputs
                    .iter()
                    .find(|output| !output.content.is_empty())
                    .map(|output| output.content.clone())
                    .unwrap_or_else(|| "No response generated".to_string())
            };

        // Validate response
        let validation_result =
            KeywordValidator::validate_response(&response_content, &scenario.expected_keywords);

        println!(
            "   📊 Score: {:.2}, Passed: {}",
            validation_result.score, validation_result.passed
        );

        // For multiple scenarios, we'll be more lenient but still check basic requirements
        assert!(
            validation_result.score >= 0.5,
            "Scenario {} failed with score {:.2}: {}",
            i + 1,
            validation_result.score,
            scenario.name
        );

        assert!(
            validation_result.found_forbidden.is_empty()
                || !validation_result.found_required.is_empty(),
            "Scenario {} had forbidden keywords or no required keywords",
            i + 1
        );
    }

    println!("\n✅ All scenarios completed successfully!");
}

/// Helper function that gets AI session outputs using the manager API
/// This ensures we test the actual API endpoints instead of reading directly from database
async fn get_ai_outputs_for_work(
    test_app: &crate::common::TestApp,
    work_id: i64,
) -> anyhow::Result<Vec<nocodo_manager::models::AiSessionOutput>> {
    use actix_web::test;

    // Make API call to get AI session outputs using the manager API endpoint
    let uri = format!("/api/work/{}/outputs", work_id);
    let req = test::TestRequest::get().uri(&uri).to_request();

    // Add mock user authentication for testing
    let mock_user = nocodo_manager::models::UserInfo {
        id: 1,
        username: "test_user".to_string(),
        email: "test@example.com".to_string(),
    };
    req.extensions_mut().insert(mock_user);

    let service = test::init_service(App::new().app_data(test_app.app_state.clone()).route(
        "/api/work/{id}/outputs",
        web::get().to(nocodo_manager::handlers::list_ai_session_outputs),
    ))
    .await;

    let resp = test::call_service(&service, req).await;

    if !resp.status().is_success() {
        return Err(anyhow::anyhow!(
            "Failed to get AI outputs from API: {}",
            resp.status()
        ));
    }

    let body: nocodo_manager::models::AiSessionOutputListResponse =
        test::read_body_json(resp).await;

    Ok(body.outputs)
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use crate::common::keyword_validation::{KeywordValidator, LlmKeywordExpectations};

    #[tokio::test]
    async fn test_keyword_validation_logic() {
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

    #[tokio::test]
    async fn test_scenario_creation() {
        let scenario = LlmTestScenario::tech_stack_analysis_saleor();

        assert!(!scenario.name.is_empty());
        assert!(!scenario.prompt.is_empty());
        assert!(!scenario.context.git_repo.is_empty());
        assert!(!scenario.expected_keywords.required_keywords.is_empty());

        // Verify specific content
        assert!(scenario.context.git_repo.contains("saleor"));
        assert!(scenario.context.git_repo.starts_with("git@github.com:"));
        assert!(scenario
            .expected_keywords
            .required_keywords
            .contains(&"Django".to_string()));
        assert!(scenario
            .expected_keywords
            .required_keywords
            .contains(&"Python".to_string()));
        assert!(scenario
            .expected_keywords
            .required_keywords
            .contains(&"PostgreSQL".to_string()));
        assert!(scenario
            .expected_keywords
            .required_keywords
            .contains(&"GraphQL".to_string()));
    }

    #[tokio::test]
    async fn test_dynamic_project_naming() {
        use crate::common::{app::TestApp, keyword_validation::LlmTestScenario};

        let llm_config = crate::common::llm_config::LlmTestConfig::from_environment();
        let provider = llm_config.get_default_provider();

        // Skip test if no provider available
        if provider.is_none() {
            return;
        }

        let provider = provider.unwrap();
        let test_app = TestApp::new_with_llm(provider).await;
        let scenario = LlmTestScenario::tech_stack_analysis_saleor();

        // Test that create_project_from_scenario returns a dynamic project name
        let project_id = test_app
            .create_project_from_scenario(&scenario.context)
            .await
            .expect("Failed to create project from scenario");

        // Verify the project ID is valid
        assert!(project_id > 0);

        // Verify the project was created in the database with the dynamic ID
        let projects = test_app
            .db()
            .get_all_projects()
            .expect("Failed to get projects");
        assert!(!projects.is_empty());

        let created_project = projects.iter().find(|p| p.id == project_id);
        assert!(
            created_project.is_some(),
            "Project with dynamic ID should exist"
        );

        let project = created_project.unwrap();
        assert_eq!(project.id, project_id);
        assert!(project.name.contains("Saleor"));
    }

    #[tokio::test]
    async fn test_llm_config_from_environment() {
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
