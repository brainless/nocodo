use nocodo_llm_sdk::{
    openai::OpenAIClient,
    tools::{Tool, ToolChoice, ToolResult},
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

#[tokio::test]
#[ignore] // Requires OPENAI_API_KEY environment variable
async fn test_openai_tool_calling() {
    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");
    let client = OpenAIClient::new(api_key).unwrap();

    let weather_tool = Tool::from_type::<WeatherParams>()
        .name("get_weather")
        .description("Get current weather for a location")
        .build();

    let response = client
        .message_builder()
        .model("gpt-4o")
        .user_message("What's the weather in Tokyo?")
        .tool(weather_tool)
        .tool_choice(ToolChoice::Auto)
        .send()
        .await
        .unwrap();

    // Should trigger tool call
    assert!(response.tool_calls().is_some());

    let tool_calls = response.tool_calls().unwrap();
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0].name(), "get_weather");

    // Parse arguments
    let params: WeatherParams = tool_calls[0].parse_arguments().unwrap();
    assert!(params.location.to_lowercase().contains("tokyo"));

    // Return result
    let result = ToolResult::text(
        tool_calls[0].id(),
        r#"{"temperature": 22, "condition": "Sunny", "location": "Tokyo"}"#,
    );

    let final_response = client
        .message_builder()
        .continue_from(&response)
        .tool_result(result)
        .send()
        .await
        .unwrap();

    assert!(!final_response.content().is_empty());
}