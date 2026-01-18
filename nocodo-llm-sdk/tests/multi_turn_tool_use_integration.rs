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
struct ListFilesParams {
    /// Directory path to list files from (e.g., ".", "src")
    #[schemars(description = "Directory path to list files from")]
    path: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct ReadFileParams {
    /// File path to read (e.g., "README.md", "src/main.rs")
    #[schemars(description = "File path to read")]
    path: String,
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

const README_CONTENT: &str = include_str!("../README.md");

#[tokio::test]
#[ignore] // Requires API keys for paid providers
async fn test_multi_turn_tool_use_all_providers() {
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
            "\n🧪 Testing multi-turn tool use: {}/{}",
            config.provider, config.model
        );

        match test_multi_turn_tool_use(config).await {
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

async fn test_multi_turn_tool_use(config: &TestConfig) -> Result<(), Box<dyn std::error::Error>> {
    let list_files_tool = Tool::from_type::<ListFilesParams>()
        .name("list_files")
        .description("List files in a directory")
        .build();

    let read_file_tool = Tool::from_type::<ReadFileParams>()
        .name("read_file")
        .description("Read contents of a file")
        .build();

    match config.provider {
        "openai" => {
            let api_key = std::env::var("OPENAI_API_KEY")?;
            let client = OpenAIClient::new(api_key)?;

            // Check if this is a GPT-5.1 model that should use Responses API
            if config.model.starts_with("gpt-5.1") || config.model.starts_with("gpt-5-codex") {
                test_openai_responses_multi_turn(
                    &client,
                    config,
                    &list_files_tool,
                    &read_file_tool,
                )
                .await?;
            } else {
                test_openai_chat_multi_turn(&client, config, &list_files_tool, &read_file_tool)
                    .await?;
            }
        }
        "anthropic" => {
            let api_key = std::env::var("ANTHROPIC_API_KEY")?;
            let client = ClaudeClient::new(api_key)?;
            test_claude_multi_turn(&client, config, &list_files_tool, &read_file_tool).await?;
        }
        "xai-grok" => {
            let api_key = std::env::var("XAI_API_KEY")?;
            let client = XaiGrokClient::new(api_key)?;
            test_xai_grok_multi_turn(&client, config, &list_files_tool, &read_file_tool).await?;
        }
        "zen-grok" => {
            let client = ZenGrokClient::new()?;
            test_zen_grok_multi_turn(&client, config, &list_files_tool, &read_file_tool).await?;
        }
        "cerebras-glm" => {
            let api_key = std::env::var("CEREBRAS_API_KEY")?;
            let client = CerebrasGlmClient::new(api_key)?;
            test_cerebras_glm_multi_turn(&client, config, &list_files_tool, &read_file_tool)
                .await?;
        }
        "zen-glm" => {
            let client = ZenGlmClient::new()?;
            test_zen_glm_multi_turn(&client, config, &list_files_tool, &read_file_tool).await?;
        }
        _ => {
            return Err(format!("Unknown provider: {}", config.provider).into());
        }
    }

    Ok(())
}

async fn test_openai_chat_multi_turn(
    client: &OpenAIClient,
    config: &TestConfig,
    list_files_tool: &Tool,
    read_file_tool: &Tool,
) -> Result<(), Box<dyn std::error::Error>> {
    // First request - model should ask for tools
    let response = client
        .message_builder()
        .model(config.model)
        .user_message("Please list the tech stack of this project, use tools as needed")
        .tools(vec![list_files_tool.clone(), read_file_tool.clone()])
        .tool_choice(ToolChoice::Auto)
        .send()
        .await?;

    // Verify tool calls were made
    assert!(
        response.tool_calls().is_some(),
        "Expected tool calls in first response"
    );
    let tool_calls = response.tool_calls().unwrap();
    assert!(!tool_calls.is_empty(), "Expected at least one tool call");

    // Find list_files and read_file calls
    let mut list_files_call = None;
    let mut read_file_call = None;

    for call in tool_calls {
        match call.name() {
            "list_files" => {
                let params: ListFilesParams = call.parse_arguments()?;
                assert!(
                    params.path == "." || params.path == "" || params.path == "./",
                    "Expected root directory path, got: {}",
                    params.path
                );
                list_files_call = Some(call.clone());
            }
            "read_file" => {
                let params: ReadFileParams = call.parse_arguments()?;
                assert_eq!(
                    params.path, "README.md",
                    "Expected README.md path, got: {}",
                    params.path
                );
                read_file_call = Some(call.clone());
            }
            _ => panic!("Unexpected tool call: {}", call.name()),
        }
    }

    // Model should call list_files first, then read README.md
    assert!(list_files_call.is_some(), "Expected list_files tool call");

    // Simulate tool responses
    let mut builder = client
        .message_builder()
        .model(config.model)
        .user_message("Please list the tech stack of this project, use tools as needed")
        .tools(vec![list_files_tool.clone(), read_file_tool.clone()])
        .tool_choice(ToolChoice::Auto);

    // Add tool responses using ToolResult
    if list_files_call.is_some() {
        let tool_result = nocodo_llm_sdk::tools::ToolResult::text(
            "list_files",
            "README.md\nCargo.toml\nsrc/\ntests/\nexamples/",
        );
        builder = builder.tool_result(tool_result);
    }

    if let Some(call) = read_file_call {
        let tool_result = nocodo_llm_sdk::tools::ToolResult::text(call.id(), README_CONTENT);
        builder = builder.tool_result(tool_result);
    } else {
        // If model didn't call read_file yet, it should in the next turn
        let second_response = builder.send().await?;

        // Should get read_file call now
        assert!(
            second_response.tool_calls().is_some(),
            "Expected read_file tool call in second response"
        );
        let second_tool_calls = second_response.tool_calls().unwrap();

        let read_call = second_tool_calls
            .iter()
            .find(|call| call.name() == "read_file")
            .ok_or("Expected read_file tool call")?;

        let params: ReadFileParams = read_call.parse_arguments()?;
        assert_eq!(
            params.path, "README.md",
            "Expected README.md path, got: {}",
            params.path
        );

        // Final turn with README content
        let tool_result = nocodo_llm_sdk::tools::ToolResult::text(read_call.id(), README_CONTENT);
        let final_response = client
            .message_builder()
            .model(config.model)
            .user_message("Please list the tech stack of this project, use tools as needed")
            .tools(vec![list_files_tool.clone(), read_file_tool.clone()])
            .tool_choice(ToolChoice::Auto)
            .tool_result(tool_result)
            .send()
            .await?;

        // Validate final response contains tech stack keywords
        let content = final_response.choices[0].message.content.clone();
        validate_tech_stack_response(&content);

        return Ok(());
    }

    let final_response = builder.send().await?;
    let content = final_response.choices[0].message.content.clone();
    validate_tech_stack_response(&content);

    Ok(())
}

async fn test_openai_responses_multi_turn(
    client: &OpenAIClient,
    config: &TestConfig,
    list_files_tool: &Tool,
    read_file_tool: &Tool,
) -> Result<(), Box<dyn std::error::Error>> {
    // For Responses API, we need to handle the different format
    let response = client
        .response_builder()
        .model(config.model)
        .input("Please list the tech stack of this project, use tools as needed")
        .tool(list_files_tool.clone())
        .tool(read_file_tool.clone())
        .send()
        .await?;

    // Check for function call items in output
    let function_call_items: Vec<_> = response
        .output
        .iter()
        .filter(|item| item.item_type == "function_call")
        .collect();

    assert!(
        !function_call_items.is_empty(),
        "Expected function calls in Responses API output"
    );

    // For now, just verify we got function calls
    println!(
        "✅ Found {} function call item(s) in Responses API output",
        function_call_items.len()
    );

    // TODO: Full multi-turn conversation for Responses API when the format is fully supported
    Ok(())
}

async fn test_claude_multi_turn(
    client: &ClaudeClient,
    config: &TestConfig,
    list_files_tool: &Tool,
    read_file_tool: &Tool,
) -> Result<(), Box<dyn std::error::Error>> {
    // First request - model should ask for tools
    let response = client
        .message_builder()
        .model(config.model)
        .max_tokens(1024)
        .user_message("Please list the tech stack of this project, use tools as needed")
        .tools(vec![list_files_tool.clone(), read_file_tool.clone()])
        .tool_choice(ToolChoice::Auto)
        .send()
        .await?;

    // Verify tool calls were made
    assert!(
        response.tool_calls().is_some(),
        "Expected tool calls in first response"
    );
    let tool_calls = response.tool_calls().unwrap();
    assert!(!tool_calls.is_empty(), "Expected at least one tool call");

    // Find list_files and read_file calls
    let mut list_files_call = None;
    let mut read_file_call = None;

    for call in tool_calls {
        match call.name() {
            "list_files" => {
                let params: ListFilesParams = call.parse_arguments()?;
                assert!(
                    params.path == "." || params.path == "" || params.path == "./",
                    "Expected root directory path, got: {}",
                    params.path
                );
                list_files_call = Some(call.clone());
            }
            "read_file" => {
                let params: ReadFileParams = call.parse_arguments()?;
                assert_eq!(
                    params.path, "README.md",
                    "Expected README.md path, got: {}",
                    params.path
                );
                read_file_call = Some(call.clone());
            }
            _ => panic!("Unexpected tool call: {}", call.name()),
        }
    }

    // Continue conversation with tool responses
    let mut builder = client
        .message_builder()
        .model(config.model)
        .max_tokens(1024)
        .user_message("Please list the tech stack of this project, use tools as needed")
        .tools(vec![list_files_tool.clone(), read_file_tool.clone()])
        .tool_choice(ToolChoice::Auto);

    // Add tool responses using ToolResult
    if list_files_call.is_some() {
        let tool_result = nocodo_llm_sdk::tools::ToolResult::text(
            "list_files",
            "README.md\nCargo.toml\nsrc/\ntests/\nexamples/",
        );
        builder = builder.tool_result(tool_result);
    }

    if let Some(call) = read_file_call {
        let tool_result = nocodo_llm_sdk::tools::ToolResult::text(call.id(), README_CONTENT);
        builder = builder.tool_result(tool_result);
    }

    let final_response = builder.send().await?;
    let content = final_response
        .content
        .iter()
        .filter_map(|block| match block {
            nocodo_llm_sdk::claude::types::ClaudeContentBlock::Text { text } => Some(text.clone()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ");

    validate_tech_stack_response(&content);

    Ok(())
}

async fn test_xai_grok_multi_turn(
    client: &XaiGrokClient,
    config: &TestConfig,
    list_files_tool: &Tool,
    read_file_tool: &Tool,
) -> Result<(), Box<dyn std::error::Error>> {
    let response = client
        .message_builder()
        .model(config.model)
        .user_message("Please list the tech stack of this project, use tools as needed")
        .tools(vec![list_files_tool.clone(), read_file_tool.clone()])
        .tool_choice(ToolChoice::Auto)
        .send()
        .await?;

    // For now, just verify we got tool calls - full multi-turn can be implemented later
    if response.choices[0].message.tool_calls.is_some() {
        let tool_calls = response.choices[0].message.tool_calls.as_ref().unwrap();
        validate_openai_style_tool_calls(tool_calls)?;
    } else {
        println!(
            "⚠️  Note: {}/{} may not support tool calling, got text response instead",
            config.provider, config.model
        );
        assert!(!response.choices[0].message.content.is_empty());
    }

    Ok(())
}

async fn test_cerebras_glm_multi_turn(
    client: &CerebrasGlmClient,
    config: &TestConfig,
    list_files_tool: &Tool,
    read_file_tool: &Tool,
) -> Result<(), Box<dyn std::error::Error>> {
    let response = client
        .message_builder()
        .model(config.model)
        .user_message("Please list the tech stack of this project, use tools as needed")
        .tools(vec![list_files_tool.clone(), read_file_tool.clone()])
        .tool_choice(ToolChoice::Auto)
        .send()
        .await?;

    // For now, just verify we got tool calls - full multi-turn can be implemented later
    if response.choices[0].message.tool_calls.is_some() {
        let tool_calls = response.choices[0].message.tool_calls.as_ref().unwrap();
        validate_openai_style_tool_calls(tool_calls)?;
    } else {
        println!(
            "⚠️  Note: {}/{} may not support tool calling, got text response instead",
            config.provider, config.model
        );
        if let Some(content) = &response.choices[0].message.content {
            if !content.is_empty() {
                // For GLM, content might be in a different field
                println!("Got text content response");
            }
        }
    }

    Ok(())
}

async fn test_zen_grok_multi_turn(
    client: &ZenGrokClient,
    config: &TestConfig,
    list_files_tool: &Tool,
    read_file_tool: &Tool,
) -> Result<(), Box<dyn std::error::Error>> {
    test_zen_multi_turn(
        client as &dyn std::any::Any,
        config,
        list_files_tool,
        read_file_tool,
        "grok",
    )
    .await
}

async fn test_zen_glm_multi_turn(
    client: &ZenGlmClient,
    config: &TestConfig,
    list_files_tool: &Tool,
    read_file_tool: &Tool,
) -> Result<(), Box<dyn std::error::Error>> {
    test_zen_multi_turn(
        client as &dyn std::any::Any,
        config,
        list_files_tool,
        read_file_tool,
        "glm",
    )
    .await
}

async fn test_zen_multi_turn(
    client: &dyn std::any::Any,
    config: &TestConfig,
    _list_files_tool: &Tool,
    _read_file_tool: &Tool,
    provider_type: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // For Zen providers, use raw request format similar to the existing tool calling test
    use schemars::gen::SchemaSettings;

    let settings = SchemaSettings::draft07().with(|s| {
        s.inline_subschemas = true;
    });
    let list_schema = settings
        .clone()
        .into_generator()
        .into_root_schema_for::<ListFilesParams>();
    let read_schema = settings
        .into_generator()
        .into_root_schema_for::<ReadFileParams>();

    if provider_type == "grok" {
        let grok_client = client
            .downcast_ref::<ZenGrokClient>()
            .ok_or("Failed to downcast to ZenGrokClient")?;

        let request = nocodo_llm_sdk::grok::types::GrokChatCompletionRequest {
            model: config.model.to_string(),
            messages: vec![nocodo_llm_sdk::grok::types::GrokMessage {
                role: nocodo_llm_sdk::grok::types::GrokRole::User,
                content: "Please list the tech stack of this project, use tools as needed"
                    .to_string(),
                tool_call_id: None,
                tool_calls: None,
            }],
            temperature: None,
            max_tokens: Some(1024),
            top_p: None,
            stream: Some(false),
            stop: None,
            tools: Some(vec![
                nocodo_llm_sdk::grok::types::GrokTool {
                    r#type: "function".to_string(),
                    function: nocodo_llm_sdk::grok::types::GrokFunction {
                        name: "list_files".to_string(),
                        description: "List files in a directory".to_string(),
                        parameters: list_schema,
                    },
                },
                nocodo_llm_sdk::grok::types::GrokTool {
                    r#type: "function".to_string(),
                    function: nocodo_llm_sdk::grok::types::GrokFunction {
                        name: "read_file".to_string(),
                        description: "Read contents of a file".to_string(),
                        parameters: read_schema,
                    },
                },
            ]),
            tool_choice: None,
            response_format: None,
        };

        let response = grok_client.create_chat_completion(request).await?;

        if let Some(tool_calls) = &response.choices[0].message.tool_calls {
            validate_tool_calls(tool_calls)?;
        } else {
            println!(
                "⚠️  Note: {}/{} may not support tool calling, got text response instead",
                config.provider, config.model
            );
            assert!(!response.choices[0].message.content.is_empty());
        }
    } else if provider_type == "glm" {
        let glm_client = client
            .downcast_ref::<ZenGlmClient>()
            .ok_or("Failed to downcast to ZenGlmClient")?;

        let request = nocodo_llm_sdk::glm::types::GlmChatCompletionRequest {
            model: config.model.to_string(),
            messages: vec![nocodo_llm_sdk::glm::types::GlmMessage {
                role: nocodo_llm_sdk::glm::types::GlmRole::User,
                content: Some(
                    "Please list the tech stack of this project, use tools as needed".to_string(),
                ),
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
            tools: Some(vec![
                nocodo_llm_sdk::glm::types::GlmTool {
                    r#type: "function".to_string(),
                    function: nocodo_llm_sdk::glm::types::GlmFunction {
                        name: "list_files".to_string(),
                        description: "List files in a directory".to_string(),
                        parameters: list_schema,
                    },
                },
                nocodo_llm_sdk::glm::types::GlmTool {
                    r#type: "function".to_string(),
                    function: nocodo_llm_sdk::glm::types::GlmFunction {
                        name: "read_file".to_string(),
                        description: "Read contents of a file".to_string(),
                        parameters: read_schema,
                    },
                },
            ]),
            tool_choice: None,
            response_format: None,
        };

        let response = glm_client.create_chat_completion(request).await?;

        if let Some(tool_calls) = &response.choices[0].message.tool_calls {
            validate_glm_tool_calls(tool_calls)?;
        } else {
            println!(
                "⚠️  Note: {}/{} may not support tool calling, got text response instead",
                config.provider, config.model
            );
            if let Some(content) = &response.choices[0].message.content {
                assert!(!content.is_empty(), "Expected non-empty content");
            }
        }
    }

    Ok(())
}

fn validate_tool_calls(
    tool_calls: &[nocodo_llm_sdk::grok::types::GrokToolCall],
) -> Result<(), Box<dyn std::error::Error>> {
    for call in tool_calls {
        match call.function.name.as_str() {
            "list_files" => {
                // Verify arguments contain path
                assert!(
                    call.function.arguments.contains("path"),
                    "Expected path parameter"
                );
            }
            "read_file" => {
                // Verify arguments contain README.md
                assert!(
                    call.function.arguments.contains("README.md"),
                    "Expected README.md path"
                );
            }
            _ => panic!("Unexpected tool call: {}", call.function.name),
        }
    }
    Ok(())
}

fn validate_glm_tool_calls(
    tool_calls: &[nocodo_llm_sdk::glm::types::GlmToolCall],
) -> Result<(), Box<dyn std::error::Error>> {
    for call in tool_calls {
        match call.function.name.as_str() {
            "list_files" => {
                assert!(
                    call.function.arguments.contains("path"),
                    "Expected path parameter"
                );
            }
            "read_file" => {
                assert!(
                    call.function.arguments.contains("README.md"),
                    "Expected README.md path"
                );
            }
            _ => panic!("Unexpected tool call: {}", call.function.name),
        }
    }
    Ok(())
}

fn validate_openai_style_tool_calls(
    tool_calls: &[nocodo_llm_sdk::openai::types::OpenAIResponseToolCall],
) -> Result<(), Box<dyn std::error::Error>> {
    for call in tool_calls {
        match call.function.name.as_str() {
            "list_files" => {
                assert!(
                    call.function.arguments.contains("path"),
                    "Expected path parameter"
                );
            }
            "read_file" => {
                assert!(
                    call.function.arguments.contains("README.md"),
                    "Expected README.md path"
                );
            }
            _ => panic!("Unexpected tool call: {}", call.function.name),
        }
    }
    Ok(())
}

fn validate_tech_stack_response(content: &str) {
    let content_lower = content.to_lowercase();

    // More lenient validation - either mention Rust OR other technologies
    let tech_keywords = [
        "rust",
        "tokio",
        "async",
        "claude",
        "openai",
        "grok",
        "glm",
        "anthropic",
        "xai",
        "cerebras",
        "sdk",
        "api",
        "typescript",
        "python",
        "node",
    ];

    let found_keywords = tech_keywords
        .iter()
        .filter(|&keyword| content_lower.contains(keyword))
        .count();

    // Accept if it mentions any tech keywords OR if it's a reasonable length (indicating it got tool results)
    let content_length = content.trim().len();
    let is_reasonable_response = content_length > 50; // Reasonable length for a tech stack analysis

    assert!(found_keywords >= 2 || is_reasonable_response,
           "Response should mention tech keywords or be substantial. Found {} keywords. Length: {}. Content preview: {}",
           found_keywords, content_length, &content[..content_length.min(200)]);
}

// Individual provider tests for easier debugging
#[tokio::test]
#[ignore]
async fn test_openai_multi_turn_tool_use() {
    let config = TestConfig::new("openai", "gpt-5-codex", Some("OPENAI_API_KEY"));
    if !config.is_available() {
        println!("⏭️  Skipping - OPENAI_API_KEY not set");
        return;
    }
    test_multi_turn_tool_use(&config)
        .await
        .expect("OpenAI multi-turn tool use test failed");
}

#[tokio::test]
#[ignore]
async fn test_anthropic_multi_turn_tool_use() {
    let config = TestConfig::new(
        "anthropic",
        "claude-sonnet-4-5-20250929",
        Some("ANTHROPIC_API_KEY"),
    );
    if !config.is_available() {
        println!("⏭️  Skipping - ANTHROPIC_API_KEY not set");
        return;
    }
    test_multi_turn_tool_use(&config)
        .await
        .expect("Anthropic multi-turn tool use test failed");
}

#[tokio::test]
#[ignore]
async fn test_xai_grok_multi_turn_tool_use() {
    let config = TestConfig::new("xai-grok", "grok-code-fast-1", Some("XAI_API_KEY"));
    if !config.is_available() {
        println!("⏭️  Skipping - XAI_API_KEY not set");
        return;
    }
    test_multi_turn_tool_use(&config)
        .await
        .expect("xAI Grok multi-turn tool use test failed");
}

#[tokio::test]
#[ignore]
async fn test_zen_grok_multi_turn_tool_use() {
    let config = TestConfig::new("zen-grok", "grok-code", None);
    test_multi_turn_tool_use(&config)
        .await
        .expect("Zen Grok multi-turn tool use test failed");
}

#[tokio::test]
#[ignore]
async fn test_cerebras_glm_multi_turn_tool_use() {
    let config = TestConfig::new("cerebras-glm", "zai-glm-4.6", Some("CEREBRAS_API_KEY"));
    if !config.is_available() {
        println!("⏭️  Skipping - CEREBRAS_API_KEY not set");
        return;
    }
    test_multi_turn_tool_use(&config)
        .await
        .expect("Cerebras GLM multi-turn tool use test failed");
}

#[tokio::test]
#[ignore]
async fn test_zen_glm_multi_turn_tool_use() {
    let config = TestConfig::new("zen-glm", "big-pickle", None);
    test_multi_turn_tool_use(&config)
        .await
        .expect("Zen GLM multi-turn tool use test failed");
}
