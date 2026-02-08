use super::types::*;
use crate::tools::{ProviderToolFormat, Tool, ToolChoice};
use serde_json::Value;

pub struct GeminiToolFormat;

impl ProviderToolFormat for GeminiToolFormat {
    type ProviderTool = GeminiTool;

    fn to_provider_tool(tool: &Tool) -> Self::ProviderTool {
        let mut parameters = serde_json::to_value(tool.parameters())
            .unwrap_or(Value::Object(serde_json::Map::new()));
        sanitize_schema(&mut parameters);

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

fn sanitize_schema(value: &mut Value) {
    match value {
        Value::Object(map) => {
            map.remove("$schema");

            if let Some(ty_value) = map.get_mut("type") {
                if let Value::Array(types) = ty_value {
                    let mut nullable = false;
                    let mut first_non_null: Option<Value> = None;
                    for item in types.iter() {
                        if item == "null" {
                            nullable = true;
                        } else if first_non_null.is_none() {
                            first_non_null = Some(item.clone());
                        }
                    }

                    if let Some(non_null) = first_non_null {
                        *ty_value = non_null;
                        if nullable {
                            map.insert("nullable".to_string(), Value::Bool(true));
                        }
                    } else {
                        map.remove("type");
                    }
                }
            }

            for (_, child) in map.iter_mut() {
                sanitize_schema(child);
            }
        }
        Value::Array(items) => {
            for item in items.iter_mut() {
                sanitize_schema(item);
            }
        }
        _ => {}
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
