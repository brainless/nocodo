use nocodo_llm_sdk::claude::ClaudeClient;
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::glm::cerebras::CerebrasGlmClient;
use nocodo_llm_sdk::glm::zen::ZenGlmClient;
use nocodo_llm_sdk::grok::xai::XaiGrokClient;
use nocodo_llm_sdk::grok::zen::ZenGrokClient;
use nocodo_llm_sdk::openai::OpenAIClient;

#[test]
fn test_all_clients_implement_trait() {
    fn assert_implements_trait<T: LlmClient>() {}

    assert_implements_trait::<OpenAIClient>();
    assert_implements_trait::<ClaudeClient>();
    assert_implements_trait::<XaiGrokClient>();
    assert_implements_trait::<CerebrasGlmClient>();
    assert_implements_trait::<ZenGlmClient>();
    assert_implements_trait::<ZenGrokClient>();
}

#[test]
fn test_trait_object_usage() {
    // Test that we can create trait objects
    let _client: Box<dyn LlmClient> = Box::new(OpenAIClient::new("test-key").unwrap());
}

#[test]
fn test_provider_and_model_names() {
    let openai_client = OpenAIClient::new("test-key").unwrap();
    assert_eq!(openai_client.provider_name(), "openai");
    assert_eq!(openai_client.model_name(), "gpt-4o");

    let claude_client = ClaudeClient::new("test-key").unwrap();
    assert_eq!(claude_client.provider_name(), "anthropic");
    assert_eq!(claude_client.model_name(), "claude-sonnet-4-5-20250929");
}
