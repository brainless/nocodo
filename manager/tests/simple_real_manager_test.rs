mod common;

use crate::common::{llm_config::LlmTestConfig, RealManagerInstance};

/// Simple test to verify real manager instance startup
#[actix_rt::test]
async fn test_real_manager_startup() {
    let llm_config = LlmTestConfig::from_environment();
    if !llm_config.has_available_providers() {
        println!("⚠️  Skipping test - no API keys available");
        return;
    }

    let provider = llm_config
        .get_default_provider()
        .expect("No default provider available");

    println!("🚀 Testing real manager startup");
    println!("   Provider: {}", provider.name);

    // Start real manager instance
    let manager = RealManagerInstance::start(provider)
        .await
        .expect("Failed to start real manager instance");

    println!("   ✅ Manager started at {}", manager.base_url);

    // Test health endpoint
    let client = manager.http_client();
    let health_response = client
        .get(manager.api_url("/health"))
        .send()
        .await
        .expect("Failed to call health endpoint");

    assert!(health_response.status().is_success());
    println!("   ✅ Health endpoint working");

    // Manager will be cleaned up automatically when dropped
    println!("   🎉 Real manager test successful");
}
