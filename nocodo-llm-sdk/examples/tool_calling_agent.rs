//! Multi-tool agent example: Search and calculate
//!
//! Run with: OPENAI_API_KEY="..." cargo run --example tool_calling_agent

use nocodo_llm_sdk::{
    openai::OpenAIClient,
    tools::{Tool, ToolChoice, ToolResult},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct SearchParams {
    /// Search query
    query: String,
    /// Maximum results (default: 10)
    #[serde(default = "default_limit")]
    limit: u32,
}

fn default_limit() -> u32 {
    10
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct CalculateParams {
    /// Mathematical expression to evaluate
    expression: String,
}

async fn execute_tool(
    call: &nocodo_llm_sdk::tools::ToolCall,
) -> Result<ToolResult, Box<dyn std::error::Error>> {
    match call.name() {
        "search" => {
            let params: SearchParams = call.parse_arguments()?;
            println!(
                "🔍 Searching for: {} (limit: {})",
                params.query, params.limit
            );

            // Simulate search results
            let results = format!(
                "Found {} results for '{}':\n1. Result A\n2. Result B\n3. Result C",
                params.limit.min(3),
                params.query
            );
            Ok(ToolResult::text(call.id(), results))
        }
        "calculate" => {
            let params: CalculateParams = call.parse_arguments()?;
            println!("🧮 Calculating: {}", params.expression);

            // Simulate calculation (in reality, use a safe eval library)
            let result = "42"; // Placeholder
            Ok(ToolResult::text(call.id(), format!("Result: {}", result)))
        }
        _ => Err(format!("Unknown tool: {}", call.name()).into()),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("OPENAI_API_KEY")?;
    let client = OpenAIClient::new(api_key)?;

    // Define tools
    let search_tool = Tool::from_type::<SearchParams>()
        .name("search")
        .description("Search the knowledge base")
        .build();

    let calc_tool = Tool::from_type::<CalculateParams>()
        .name("calculate")
        .description("Evaluate mathematical expressions")
        .build();

    println!("🤖 Multi-tool agent starting...\n");

    let response = client
        .message_builder()
        .model("gpt-4o")
        .user_message("Search for 'Rust programming' and calculate 123 * 456")
        .tools(vec![search_tool, calc_tool])
        .tool_choice(ToolChoice::Auto)
        .parallel_tool_calls(true)
        .send()
        .await?;

    // Handle tool calls
    if let Some(tool_calls) = response.tool_calls() {
        println!("📞 Executing {} tool(s)...\n", tool_calls.len());

        let mut results = Vec::new();
        for call in tool_calls {
            let result = execute_tool(&call).await?;
            results.push(result);
        }

        // Continue conversation
        let mut builder = client.message_builder().continue_from(&response);
        for result in results {
            builder = builder.tool_result(result);
        }

        let final_response = builder.send().await?;
        println!("\n✅ Agent completed:\n{}", final_response.content());
    }

    Ok(())
}
