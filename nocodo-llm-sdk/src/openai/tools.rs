use crate::tools::{ProviderToolFormat, Tool, ToolChoice};
use super::types::{OpenAITool, OpenAIFunction};
use serde_json::{json, Value};

/// OpenAI tool format implementation
pub struct OpenAIToolFormat;

impl ProviderToolFormat for OpenAIToolFormat {
    type ProviderTool = OpenAITool;

    fn to_provider_tool(tool: &Tool) -> Self::ProviderTool {
        OpenAITool {
            r#type: "function".to_string(),
            function: OpenAIFunction {
                name: tool.name().to_string(),
                description: tool.description().to_string(),
                parameters: tool.parameters().clone(),
            },
        }
    }

    fn to_provider_tool_choice(choice: &ToolChoice) -> Value {
        match choice {
            ToolChoice::Auto => json!("auto"),
            ToolChoice::Required => json!("required"),
            ToolChoice::None => json!("none"),
            ToolChoice::Specific { name } => json!({
                "type": "function",
                "function": { "name": name }
            }),
        }
    }
}