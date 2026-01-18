use super::types::*;
use crate::tools::{ProviderToolFormat, Tool, ToolChoice};
use serde_json::Value;

pub struct GeminiToolFormat;

impl ProviderToolFormat for GeminiToolFormat {
    type ProviderTool = GeminiTool;

    fn to_provider_tool(tool: &Tool) -> Self::ProviderTool {
        let parameters = serde_json::to_value(tool.parameters())
            .unwrap_or(Value::Object(serde_json::Map::new()));

        GeminiTool {
            function_declarations: Some(vec![GeminiFunctionDeclaration {
                name: tool.name().to_string(),
                description: tool.description().to_string(),
                parameters,
            }]),
        }
    }

    fn to_provider_tool_choice(choice: &ToolChoice) -> Value {
        match choice {
            ToolChoice::Auto => serde_json::json!({
                "functionCallingConfig": {
                    "mode": "AUTO"
                }
            }),
            ToolChoice::Required => serde_json::json!({
                "functionCallingConfig": {
                    "mode": "ANY"
                }
            }),
            ToolChoice::None => serde_json::json!({
                "functionCallingConfig": {
                    "mode": "NONE"
                }
            }),
            ToolChoice::Specific { name } => serde_json::json!({
                "functionCallingConfig": {
                    "mode": "ANY",
                    "allowedFunctionNames": [name]
                }
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_choice_auto() {
        let choice = ToolChoice::Auto;
        let result = GeminiToolFormat::to_provider_tool_choice(&choice);
        assert_eq!(result["functionCallingConfig"]["mode"], "AUTO");
    }

    #[test]
    fn test_tool_choice_required() {
        let choice = ToolChoice::Required;
        let result = GeminiToolFormat::to_provider_tool_choice(&choice);
        assert_eq!(result["functionCallingConfig"]["mode"], "ANY");
    }

    #[test]
    fn test_tool_choice_none() {
        let choice = ToolChoice::None;
        let result = GeminiToolFormat::to_provider_tool_choice(&choice);
        assert_eq!(result["functionCallingConfig"]["mode"], "NONE");
    }

    #[test]
    fn test_tool_choice_specific() {
        let choice = ToolChoice::Specific {
            name: "search".to_string(),
        };
        let result = GeminiToolFormat::to_provider_tool_choice(&choice);
        assert_eq!(result["functionCallingConfig"]["mode"], "ANY");
        let names = result["functionCallingConfig"]["allowedFunctionNames"].as_array();
        assert!(names.is_some());
        assert_eq!(names.unwrap().len(), 1);
    }
}
