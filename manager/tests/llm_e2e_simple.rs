mod common;

use actix::Actor;
use actix_web::{test, web, App};
use std::sync::Arc;
use std::time::SystemTime;
use tempfile::tempdir;

use nocodo_manager::{
    config::AppConfig,
    database::Database,
    handlers::{create_ai_session, health_check, AppState},
    llm_agent::LlmAgent,
    models::CreateLlmAgentSessionRequest,
    websocket::{WebSocketBroadcaster, WebSocketServer},
};

use crate::common::{
    keyword_validation::{KeywordValidator, LlmTestScenario},
    llm_config::LlmTestConfig,
};

/// Simple LLM E2E test that makes real API calls
#[actix_rt::test]
async fn test_simple_llm_e2e() {
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
    println!(
        "🚀 Running simple LLM E2E test with provider: {}",
        provider.name
    );

    // Create isolated test environment
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let database = Arc::new(Database::new(&db_path).unwrap());

    // Create WebSocket infrastructure
    let ws_server = WebSocketServer::default().start();
    let ws_broadcaster = Arc::new(WebSocketBroadcaster::new(ws_server));

    // Create projects directory
    let projects_dir = temp_dir.path().join("projects");
    std::fs::create_dir_all(&projects_dir).unwrap();

    // Create real LLM agent
    let llm_agent = Some(Arc::new(LlmAgent::new(
        database.clone(),
        ws_broadcaster.clone(),
        projects_dir.clone(),
        Arc::new(provider.to_app_config()),
    )));

    let app_state = web::Data::new(AppState {
        database: database.clone(),
        start_time: SystemTime::now(),
        ws_broadcaster,
        llm_agent,
        config: Arc::new(AppConfig::default()),
    });

    // Create test app with minimal routes
    let app = test::init_service(
        App::new()
            .app_data(app_state)
            .route("/api/health", web::get().to(health_check))
            .route(
                "/work/{work_id}/llm-agent/sessions",
                web::post().to(create_ai_session),
            ),
    )
    .await;

    // Test health check first
    println!("✅ Testing health check");
    let req = test::TestRequest::get().uri("/api/health").to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    // Create a test work session in the database
    println!("✅ Creating test work session");
    let work = nocodo_manager::models::Work {
        id: 300, // Test ID
        title: "Test Work".to_string(),
        tool_name: Some("llm_e2e_test".to_string()),
        model: Some("gpt-5".to_string()),
        status: "active".to_string(),
        project_id: None,
        created_at: chrono::Utc::now().timestamp(),
        updated_at: chrono::Utc::now().timestamp(),
    };
    database.create_work(&work).unwrap();

    // Create project files for context
    println!("✅ Creating project context");
    let project_dir = projects_dir.join("test-project");
    std::fs::create_dir_all(&project_dir).unwrap();

    // Create a simple Python FastAPI project
    std::fs::write(
        project_dir.join("main.py"),
        "from fastapi import FastAPI\napp = FastAPI()\n@app.get(\"/\")\ndef read_root():\n    return {\"Hello\": \"World\"}"
    ).unwrap();

    std::fs::write(
        project_dir.join("requirements.txt"),
        "fastapi==0.104.1\nuvicorn==0.24.0",
    )
    .unwrap();

    // Test LLM session creation
    println!("🤖 Creating LLM session");
    let session_request = CreateLlmAgentSessionRequest {
        provider: provider.name.clone(),
        model: provider.default_model().to_string(),
        system_prompt: Some(
            "You are a helpful coding assistant. Analyze the tech stack and be concise."
                .to_string(),
        ),
    };

    let uri = format!("/work/{}/llm-agent/sessions", work.id);
    let req = test::TestRequest::post()
        .uri(&uri)
        .set_json(&session_request)
        .to_request();

    let resp = test::call_service(&app, req).await;
    println!("📊 LLM session creation status: {}", resp.status());

    let is_success = resp.status().is_success();
    if !is_success {
        let body = test::read_body(resp).await;
        let body_str = String::from_utf8_lossy(&body);
        println!("❌ Response body: {}", body_str);
        panic!("Failed to create LLM session");
    }

    let body: serde_json::Value = test::read_body_json(resp).await;
    let session_id = body["session"]["id"]
        .as_i64()
        .expect("No session ID returned");
    println!("✅ LLM session created: {}", session_id);

    // For this simple test, we'll validate that the session was created properly
    // In a full implementation, you would send messages and validate responses
    let session = database.get_llm_agent_session(session_id).unwrap();
    assert_eq!(session.provider, provider.name);
    assert_eq!(session.model, provider.default_model());
    assert_eq!(session.work_id, work.id);

    println!("🎉 Simple LLM E2E test completed successfully!");
    println!("   ✅ Test isolation working");
    println!("   ✅ Real LLM agent integrated");
    println!("   ✅ Session creation successful");
}

/// Test the keyword validation system independently
#[test]
async fn test_keyword_validation_system() {
    println!("🧪 Testing keyword validation system");

    let scenario = LlmTestScenario::tech_stack_analysis_saleor();

    // Simulate a good LLM response
    let good_response = "This project uses Django, Python, PostgreSQL, and GraphQL";

    let result = KeywordValidator::validate_response(good_response, &scenario.expected_keywords);

    println!("📊 Validation results:");
    println!("   Score: {:.2}", result.score);
    println!("   Required found: {:?}", result.found_required);
    println!("   Optional found: {:?}", result.found_optional);
    println!("   Forbidden found: {:?}", result.found_forbidden);

    assert!(
        result.passed,
        "Keyword validation should pass for good response"
    );
    assert!(result.score >= 0.7, "Score should be at least 0.7");
    assert_eq!(result.found_required.len(), 4); // Django, Python, PostgreSQL, GraphQL
    assert_eq!(result.found_optional.len(), 0); // No optional keywords
    assert_eq!(result.found_forbidden.len(), 0); // No forbidden keywords

    println!("✅ Keyword validation system working correctly");
}

/// Test LLM provider configuration
#[test]
async fn test_llm_provider_config() {
    println!("🔧 Testing LLM provider configuration");

    let config = LlmTestConfig::from_environment();

    println!("Available providers: {}", config.enabled_providers.len());
    for provider in &config.enabled_providers {
        println!(
            "   - {}: {} ({})",
            provider.name,
            provider.default_model(),
            if provider.enabled {
                "enabled"
            } else {
                "disabled"
            }
        );
    }

    // Test that configuration is sane
    assert!(config.test_timeouts.request_timeout_secs > 0);
    assert!(config.test_timeouts.total_test_timeout_secs > 0);

    if config.has_available_providers() {
        let default_provider = config.get_default_provider().unwrap();
        assert!(!default_provider.name.is_empty());
        assert!(!default_provider.models.is_empty());
        assert!(!default_provider.api_key_env.is_empty());
        println!("✅ Default provider configured: {}", default_provider.name);
    } else {
        println!("⚠️  No providers available - set API keys to test LLM integration");
    }

    println!("✅ LLM provider configuration working correctly");
}
