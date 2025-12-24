use super::types::ClaudeTool;
use crate::tools::{ProviderToolFormat, Tool, ToolChoice};
use serde_json::{json, Value};

/// Claude tool format implementation
pub struct ClaudeToolFormat;

impl ProviderToolFormat for ClaudeToolFormat {
    type ProviderTool = ClaudeTool;

    fn to_provider_tool(tool: &Tool) -> Self::ProviderTool {
        ClaudeTool {
            name: tool.name().to_string(),
            description: tool.description().to_string(),
            input_schema: tool.parameters().clone(),
            cache_control: None,
        }
    }

    fn to_provider_tool_choice(choice: &ToolChoice) -> Value {
        match choice {
            ToolChoice::Auto => json!({"type": "auto"}),
            ToolChoice::Required => json!({"type": "any"}),
            ToolChoice::None => json!({"type": "none"}),
            ToolChoice::Specific { name } => json!({
                "type": "tool",
                "name": name
            }),
        }
    }
}
