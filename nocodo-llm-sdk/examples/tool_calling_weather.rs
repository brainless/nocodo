//! Simple tool calling example: Weather lookup
//!
//! Run with: OPENAI_API_KEY="..." cargo run --example tool_calling_weather

use nocodo_llm_sdk::{
    openai::OpenAIClient,
    tools::{Tool, ToolChoice, ToolResult},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct WeatherParams {
    /// City name (e.g., "New York", "London", "Tokyo")
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("OPENAI_API_KEY")?;
    let client = OpenAIClient::new(api_key)?;

    // Define tool
    let weather_tool = Tool::from_type::<WeatherParams>()
        .name("get_weather")
        .description("Get current weather for a city")
        .build();

    println!("🤖 Asking: What's the weather in Paris and Tokyo?");

    // First request with tool
    let response = client
        .message_builder()
        .model("gpt-4o")
        .user_message("What's the weather in Paris and Tokyo?")
        .tool(weather_tool)
        .tool_choice(ToolChoice::Auto)
        .parallel_tool_calls(true)
        .send()
        .await?;

    // Handle tool calls
    if let Some(tool_calls) = response.tool_calls() {
        println!("\n📞 LLM requested {} tool call(s):", tool_calls.len());

        let mut results = Vec::new();

        for call in tool_calls {
            println!("\n  Tool: {}", call.name());

            // Type-safe parameter extraction
            let params: WeatherParams = call.parse_arguments()?;
            println!("  Location: {}", params.location);
            println!("  Unit: {:?}", params.unit);

            // Simulate weather API call
            let weather_data = format!(
                "{{\"temperature\": 22, \"condition\": \"Sunny\", \"location\": \"{}\"}}",
                params.location
            );

            results.push(ToolResult::text(call.id(), weather_data));
        }

        // Continue conversation with results
        let mut builder = client.message_builder().continue_from(&response);
        for result in results {
            builder = builder.tool_result(result);
        }

        let final_response = builder.send().await?;
        println!("\n✅ Final response:\n{}", final_response.content());
    } else {
        println!("\n✅ Direct response:\n{}", response.content());
    }

    Ok(())
}
