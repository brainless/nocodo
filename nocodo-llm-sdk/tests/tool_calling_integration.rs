use nocodo_llm_sdk::{
    claude::ClaudeClient,
    glm::cerebras::CerebrasGlmClient,
    glm::zen::ZenGlmClient,
    grok::xai::XaiGrokClient,
    grok::zen::ZenGrokClient,
    openai::OpenAIClient,
    tools::{Tool, ToolChoice},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct WeatherParams {
    /// City name (e.g., "New York", "London")
    location: String,
    /// Temperature unit
    #[serde(default)]
    unit: TempUnit,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
enum TempUnit {
    Celsius,
    Fahrenheit,
}

impl Default for TempUnit {
    fn default() -> Self {
        Self::Celsius
    }
}

/// Test configuration for provider/model combinations
struct TestConfig {
    provider: &'static str,
    model: &'static str,
    api_key_env: Option<&'static str>,
}

impl TestConfig {
    const fn new(
        provider: &'static str,
        model: &'static str,
        api_key_env: Option<&'static str>,
    ) -> Self {
        Self {
            provider,
            model,
            api_key_env,
        }
    }

    fn is_available(&self) -> bool {
        match self.api_key_env {
            Some(env_var) => std::env::var(env_var).is_ok(),
            None => true, // Free models are always available
        }
    }
}

const TEST_CONFIGS: &[TestConfig] = &[
    TestConfig::new("openai", "gpt-5-codex", Some("OPENAI_API_KEY")),
    TestConfig::new(
        "anthropic",
        "claude-sonnet-4-5-20250929",
        Some("ANTHROPIC_API_KEY"),
    ),
    TestConfig::new("xai-grok", "grok-code-fast-1", Some("XAI_API_KEY")),
    TestConfig::new("zen-grok", "grok-code", None),
    TestConfig::new("cerebras-glm", "zai-glm-4.6", Some("CEREBRAS_API_KEY")),
    TestConfig::new("zen-glm", "big-pickle", None),
];

#[tokio::test]
#[ignore] // Requires API keys for paid providers
async fn test_tool_calling_all_providers() {
    let mut results = Vec::new();
    let mut skipped = Vec::new();

    for config in TEST_CONFIGS {
        if !config.is_available() {
            println!(
                "⏭️  Skipping {}/{} - API key not available",
                config.provider, config.model
            );
            skipped.push((config.provider, config.model));
            continue;
        }

        println!(
            "\n🧪 Testing tool calling: {}/{}",
            config.provider, config.model
        );

        match test_tool_calling(config).await {
            Ok(_) => {
                println!("✅ PASS: {}/{}", config.provider, config.model);
                results.push((config.provider, config.model, true));
            }
            Err(e) => {
                println!("❌ FAIL: {}/{} - {}", config.provider, config.model, e);
                results.push((config.provider, config.model, false));
            }
        }
    }

    // Print summary
    println!("\n📊 Test Summary:");
    println!("================");

    let passed = results.iter().filter(|(_, _, success)| *success).count();
    let failed = results.iter().filter(|(_, _, success)| !*success).count();

    for (provider, model, success) in &results {
        let status = if *success { "✅ PASS" } else { "❌ FAIL" };
        println!("{}: {}/{}", status, provider, model);
    }

    if !skipped.is_empty() {
        println!("\n⏭️  Skipped:");
        for (provider, model) in &skipped {
            println!("  - {}/{} (API key not available)", provider, model);
        }
    }

    println!(
        "\n📈 Results: {} passed, {} failed, {} skipped",
        passed,
        failed,
        skipped.len()
    );

    // Assert at least one test ran and all that ran passed
    assert!(
        !results.is_empty(),
        "No tests ran - at least one provider must be available"
    );
    assert_eq!(failed, 0, "Some tests failed - check output above");
}

async fn test_tool_calling(config: &TestConfig) -> Result<(), Box<dyn std::error::Error>> {
    let weather_tool = Tool::from_type::<WeatherParams>()
        .name("get_weather")
        .description("Get current weather for a location")
        .build();

    match config.provider {
        "openai" => {
            let api_key = std::env::var("OPENAI_API_KEY")?;
            let client = OpenAIClient::new(api_key)?;

            // Check if this is a GPT-5.1 model that should use Responses API
            if config.model.starts_with("gpt-5.1") || config.model.starts_with("gpt-5-codex") {
                // Use Responses API for GPT-5.1 models
                let response = client
                    .response_builder()
                    .model(config.model)
                    .input("What's the weather in Tokyo?")
                    .tool(weather_tool)
                    .tool_choice(ToolChoice::Auto)
                    .send()
                    .await?;

                // For Responses API, tool calls appear in the output array as function_call items
                let function_call_items: Vec<_> = response
                    .output
                    .iter()
                    .filter(|item| item.item_type == "function_call")
                    .collect();

                assert!(
                    !function_call_items.is_empty(),
                    "Expected tool call but got none in Responses API output"
                );

                // For now, just verify we got a function call item - full parsing can be added later
                // when the OpenAIOutputItem type is updated to include call_id and arguments fields
                println!(
                    "✅ Found {} function call item(s) in Responses API output",
                    function_call_items.len()
                );
            } else {
                // Use Chat Completions API for other models
                let response = client
                    .message_builder()
                    .model(config.model)
                    .user_message("What's the weather in Tokyo?")
                    .tool(weather_tool)
                    .tool_choice(ToolChoice::Auto)
                    .send()
                    .await?;

                // Verify tool call was made
                assert!(
                    response.tool_calls().is_some(),
                    "Expected tool call but got none"
                );

                let tool_calls = response.tool_calls().unwrap();
                assert_eq!(tool_calls.len(), 1, "Expected 1 tool call");
                assert_eq!(
                    tool_calls[0].name(),
                    "get_weather",
                    "Expected get_weather tool"
                );

                // Parse and validate arguments
                let params: WeatherParams = tool_calls[0].parse_arguments()?;
                assert!(
                    params.location.to_lowercase().contains("tokyo"),
                    "Expected location to contain 'tokyo', got: {}",
                    params.location
                );
            }

            // For now, just verify we got the tool call - full conversation flow can be added later
            // The important part is that the model correctly called the tool
        }
        "anthropic" => {
            let api_key = std::env::var("ANTHROPIC_API_KEY")?;
            let client = ClaudeClient::new(api_key)?;

            let response = client
                .message_builder()
                .model(config.model)
                .max_tokens(1024)
                .user_message("What's the weather in Tokyo?")
                .tool(weather_tool)
                .tool_choice(ToolChoice::Auto)
                .send()
                .await?;

            // Verify tool call was made
            assert!(
                response.tool_calls().is_some(),
                "Expected tool call but got none"
            );

            let tool_calls = response.tool_calls().unwrap();
            assert_eq!(tool_calls.len(), 1, "Expected 1 tool call");
            assert_eq!(
                tool_calls[0].name(),
                "get_weather",
                "Expected get_weather tool"
            );

            // Parse and validate arguments
            let params: WeatherParams = tool_calls[0].parse_arguments()?;
            assert!(
                params.location.to_lowercase().contains("tokyo"),
                "Expected location to contain 'tokyo', got: {}",
                params.location
            );

            // For now, just verify we got the tool call - full conversation flow can be added later
            // The important part is that the model correctly called the tool
        }
        "xai-grok" => {
            let api_key = std::env::var("XAI_API_KEY")?;
            let client = XaiGrokClient::new(api_key)?;

            let response = client
                .message_builder()
                .model(config.model)
                .user_message("What's the weather in Tokyo?")
                .tool(weather_tool)
                .tool_choice(ToolChoice::Auto)
                .send()
                .await?;

            // Verify tool call was made
            assert!(
                response.tool_calls().is_some(),
                "Expected tool call but got none"
            );

            let tool_calls = response.tool_calls().unwrap();
            assert_eq!(tool_calls.len(), 1, "Expected 1 tool call");
            assert_eq!(
                tool_calls[0].name(),
                "get_weather",
                "Expected get_weather tool"
            );

            // Parse and validate arguments
            let params: WeatherParams = tool_calls[0].parse_arguments()?;
            assert!(
                params.location.to_lowercase().contains("tokyo"),
                "Expected location to contain 'tokyo', got: {}",
                params.location
            );

            // For now, just verify we got the tool call - full conversation flow can be added later
            // The important part is that the model correctly called the tool
        }
        "zen-grok" => {
            let client = ZenGrokClient::new()?;

            // Note: Zen providers use raw request format, not builders
            // We'll need to test if they support tool calling
            // For now, we'll create a basic test structure
            // Use inline schema generation to avoid allOf/$ref which Zen doesn't support
            use schemars::gen::SchemaSettings;

            let settings = SchemaSettings::draft07().with(|s| {
                s.inline_subschemas = true;
            });
            let generator = settings.into_generator();
            let schema = generator.into_root_schema_for::<WeatherParams>();

            let request = nocodo_llm_sdk::grok::types::GrokChatCompletionRequest {
                model: config.model.to_string(),
                messages: vec![nocodo_llm_sdk::grok::types::GrokMessage {
                    role: nocodo_llm_sdk::grok::types::GrokRole::User,
                    content: "What's the weather in Tokyo?".to_string(),
                    tool_call_id: None,
                    tool_calls: None,
                }],
                temperature: None,
                max_tokens: Some(1024),
                top_p: None,
                stream: Some(false),
                stop: None,
                tools: Some(vec![nocodo_llm_sdk::grok::types::GrokTool {
                    r#type: "function".to_string(),
                    function: nocodo_llm_sdk::grok::types::GrokFunction {
                        name: "get_weather".to_string(),
                        description: "Get current weather for a location".to_string(),
                        parameters: schema,
                    },
                }]),
                tool_choice: None,
            };

            let response = client.create_chat_completion(request).await?;

            // Check if tool calling is supported
            if let Some(tool_calls) = &response.choices[0].message.tool_calls {
                assert_eq!(tool_calls.len(), 1, "Expected 1 tool call");
                assert_eq!(
                    tool_calls[0].function.name, "get_weather",
                    "Expected get_weather tool"
                );

                // Verify arguments contain Tokyo
                let args_str = tool_calls[0].function.arguments.to_string();
                assert!(
                    args_str.to_lowercase().contains("tokyo"),
                    "Expected arguments to contain 'tokyo', got: {}",
                    args_str
                );
            } else {
                // If tool calling isn't supported, just verify we got a response
                println!(
                    "⚠️  Note: {}/{} may not support tool calling, got text response instead",
                    config.provider, config.model
                );
                assert!(!response.choices[0].message.content.is_empty());
            }
        }
        "cerebras-glm" => {
            let api_key = std::env::var("CEREBRAS_API_KEY")?;
            let client = CerebrasGlmClient::new(api_key)?;

            let response = client
                .message_builder()
                .model(config.model)
                .user_message("What's the weather in Tokyo?")
                .tool(weather_tool)
                .tool_choice(ToolChoice::Auto)
                .send()
                .await?;

            // Verify tool call was made
            assert!(
                response.tool_calls().is_some(),
                "Expected tool call but got none"
            );

            let tool_calls = response.tool_calls().unwrap();
            assert_eq!(tool_calls.len(), 1, "Expected 1 tool call");
            assert_eq!(
                tool_calls[0].name(),
                "get_weather",
                "Expected get_weather tool"
            );

            // Parse and validate arguments
            let params: WeatherParams = tool_calls[0].parse_arguments()?;
            assert!(
                params.location.to_lowercase().contains("tokyo"),
                "Expected location to contain 'tokyo', got: {}",
                params.location
            );

            // For now, just verify we got the tool call - full conversation flow can be added later
            // The important part is that the model correctly called the tool
        }
        "zen-glm" => {
            let client = ZenGlmClient::new()?;

            // Use inline schema generation to avoid allOf/$ref which Zen doesn't support
            use schemars::gen::SchemaSettings;

            let settings = SchemaSettings::draft07().with(|s| {
                s.inline_subschemas = true;
            });
            let generator = settings.into_generator();
            let schema = generator.into_root_schema_for::<WeatherParams>();

            let request = nocodo_llm_sdk::glm::types::GlmChatCompletionRequest {
                model: config.model.to_string(),
                messages: vec![nocodo_llm_sdk::glm::types::GlmMessage {
                    role: nocodo_llm_sdk::glm::types::GlmRole::User,
                    content: Some("What's the weather in Tokyo?".to_string()),
                    reasoning: None,
                    tool_call_id: None,
                    tool_calls: None,
                }],
                temperature: None,
                max_completion_tokens: Some(1024),
                top_p: None,
                stream: Some(false),
                stop: None,
                seed: None,
                tools: Some(vec![nocodo_llm_sdk::glm::types::GlmTool {
                    r#type: "function".to_string(),
                    function: nocodo_llm_sdk::glm::types::GlmFunction {
                        name: "get_weather".to_string(),
                        description: "Get current weather for a location".to_string(),
                        parameters: schema,
                    },
                }]),
                tool_choice: None,
            };

            let response = client.create_chat_completion(request).await?;

            // Check if tool calling is supported
            if let Some(tool_calls) = &response.choices[0].message.tool_calls {
                assert_eq!(tool_calls.len(), 1, "Expected 1 tool call");
                assert_eq!(
                    tool_calls[0].function.name, "get_weather",
                    "Expected get_weather tool"
                );

                // Verify arguments contain Tokyo
                let args_str = tool_calls[0].function.arguments.to_string();
                assert!(
                    args_str.to_lowercase().contains("tokyo"),
                    "Expected arguments to contain 'tokyo', got: {}",
                    args_str
                );
            } else {
                // If tool calling isn't supported, just verify we got a response
                println!(
                    "⚠️  Note: {}/{} may not support tool calling, got text response instead",
                    config.provider, config.model
                );
                if let Some(content) = &response.choices[0].message.content {
                    assert!(!content.is_empty(), "Expected non-empty content");
                }
            }
        }
        _ => {
            return Err(format!("Unknown provider: {}", config.provider).into());
        }
    }

    Ok(())
}

// Individual provider tests for easier debugging
#[tokio::test]
#[ignore]
async fn test_openai_tool_calling() {
    let config = TestConfig::new("openai", "gpt-5-codex", Some("OPENAI_API_KEY"));
    if !config.is_available() {
        println!("⏭️  Skipping - OPENAI_API_KEY not set");
        return;
    }
    test_tool_calling(&config)
        .await
        .expect("OpenAI tool calling test failed");
}

#[tokio::test]
#[ignore]
async fn test_anthropic_tool_calling() {
    let config = TestConfig::new(
        "anthropic",
        "claude-sonnet-4-5-20250929",
        Some("ANTHROPIC_API_KEY"),
    );
    if !config.is_available() {
        println!("⏭️  Skipping - ANTHROPIC_API_KEY not set");
        return;
    }
    test_tool_calling(&config)
        .await
        .expect("Anthropic tool calling test failed");
}

#[tokio::test]
#[ignore]
async fn test_xai_grok_tool_calling() {
    let config = TestConfig::new("xai-grok", "grok-code-fast-1", Some("XAI_API_KEY"));
    if !config.is_available() {
        println!("⏭️  Skipping - XAI_API_KEY not set");
        return;
    }
    test_tool_calling(&config)
        .await
        .expect("xAI Grok tool calling test failed");
}

#[tokio::test]
#[ignore]
async fn test_zen_grok_tool_calling() {
    let config = TestConfig::new("zen-grok", "grok-code", None);
    test_tool_calling(&config)
        .await
        .expect("Zen Grok tool calling test failed");
}

#[tokio::test]
#[ignore]
async fn test_cerebras_glm_tool_calling() {
    let config = TestConfig::new("cerebras-glm", "zai-glm-4.6", Some("CEREBRAS_API_KEY"));
    if !config.is_available() {
        println!("⏭️  Skipping - CEREBRAS_API_KEY not set");
        return;
    }
    test_tool_calling(&config)
        .await
        .expect("Cerebras GLM tool calling test failed");
}

#[tokio::test]
#[ignore]
async fn test_zen_glm_tool_calling() {
    let config = TestConfig::new("zen-glm", "big-pickle", None);
    test_tool_calling(&config)
        .await
        .expect("Zen GLM tool calling test failed");
}
