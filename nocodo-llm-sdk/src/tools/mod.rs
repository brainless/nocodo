use schemars::schema::RootSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::marker::PhantomData;

/// A tool that can be called by an LLM
#[derive(Debug, Clone)]
pub struct Tool {
    name: String,
    description: String,
    parameters: RootSchema,
}

impl Tool {
    /// Create a tool from a type that implements JsonSchema
    pub fn from_type<T: schemars::JsonSchema>() -> ToolBuilder<T> {
        ToolBuilder {
            name: None,
            description: None,
            _phantom: PhantomData,
        }
    }

    /// Create a tool from raw JSON Schema (for FFI/future language bindings)
    #[cfg(feature = "ffi")]
    pub fn from_json_schema(
        name: String,
        description: String,
        schema: Value,
    ) -> Result<Self, crate::error::LlmError> {
        let parameters: RootSchema = serde_json::from_value(schema)
            .map_err(|e| crate::error::LlmError::InvalidToolSchema {
                message: e.to_string(),
            })?;

        Ok(Tool {
            name,
            description,
            parameters,
        })
    }

    // Getters
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn parameters(&self) -> &RootSchema {
        &self.parameters
    }
}

/// Builder for type-safe tools
pub struct ToolBuilder<T> {
    name: Option<String>,
    description: Option<String>,
    _phantom: PhantomData<T>,
}

impl<T: schemars::JsonSchema> ToolBuilder<T> {
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn build(self) -> Tool {
        // Use inline_subschemas to avoid allOf/$ref which are not supported by xAI
        // and have limited support in OpenAI and Anthropic APIs
        use schemars::gen::SchemaSettings;

        let settings = SchemaSettings::draft07().with(|s| {
            s.inline_subschemas = true;
        });
        let generator = settings.into_generator();
        let schema = generator.into_root_schema_for::<T>();

        Tool {
            name: self.name.expect("Tool name is required"),
            description: self.description.unwrap_or_default(),
            parameters: schema,
        }
    }
}

/// A tool call from the LLM
#[derive(Debug, Clone)]
pub struct ToolCall {
    id: String,
    name: String,
    arguments: Value,
}

impl ToolCall {
    pub fn new(id: String, name: String, arguments: Value) -> Self {
        Self { id, name, arguments }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// Parse arguments into a strongly-typed struct
    pub fn parse_arguments<T>(&self) -> Result<T, crate::error::LlmError>
    where
        T: for<'de> Deserialize<'de>,
    {
        serde_json::from_value(self.arguments.clone())
            .map_err(|e| crate::error::LlmError::ToolArgumentParse {
                tool_name: self.name.clone(),
                source: e,
            })
    }

    /// Get raw JSON arguments
    pub fn raw_arguments(&self) -> &Value {
        &self.arguments
    }
}

/// Tool execution result to send back to the LLM
#[derive(Debug, Clone)]
pub struct ToolResult {
    tool_call_id: String,
    content: String,
}

impl ToolResult {
    /// Create a tool result from any serializable value
    pub fn new<T: Serialize>(
        tool_call_id: impl Into<String>,
        content: T,
    ) -> Result<Self, crate::error::LlmError> {
        let content = serde_json::to_string(&content)
            .map_err(|e| crate::error::LlmError::Parse { source: e })?;
        Ok(Self {
            tool_call_id: tool_call_id.into(),
            content,
        })
    }

    /// Create a tool result from a plain text string
    pub fn text(tool_call_id: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            tool_call_id: tool_call_id.into(),
            content: text.into(),
        }
    }

    pub fn tool_call_id(&self) -> &str {
        &self.tool_call_id
    }

    pub fn content(&self) -> &str {
        &self.content
    }
}

/// Tool choice strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ToolChoice {
    /// Let the model decide whether to use tools
    #[default]
    Auto,
    /// Force the model to use at least one tool
    Required,
    /// Disable tool use
    None,
    /// Force a specific tool by name
    Specific { name: String },
}

/// Convert unified Tool to provider-specific format
pub trait ProviderToolFormat {
    type ProviderTool: Serialize;

    fn to_provider_tool(tool: &Tool) -> Self::ProviderTool;
    fn to_provider_tool_choice(choice: &ToolChoice) -> Value;
}

#[cfg(test)]
mod tests {
    use super::*;
    use schemars::JsonSchema;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, JsonSchema)]
    struct TestParams {
        query: String,
        limit: u32,
    }

    #[test]
    fn test_tool_creation() {
        let tool = Tool::from_type::<TestParams>()
            .name("search")
            .description("Search database")
            .build();

        assert_eq!(tool.name(), "search");
        assert_eq!(tool.description(), "Search database");
    }

    #[test]
    fn test_tool_call_parsing() {
        let args = serde_json::json!({
            "query": "rust",
            "limit": 10
        });

        let call = ToolCall::new(
            "call_123".to_string(),
            "search".to_string(),
            args,
        );

        let params: TestParams = call.parse_arguments().unwrap();
        assert_eq!(params.query, "rust");
        assert_eq!(params.limit, 10);
    }

    #[test]
    fn test_tool_result_creation() {
        let result = ToolResult::text("call_123", "Found 10 results");
        assert_eq!(result.tool_call_id(), "call_123");
        assert_eq!(result.content(), "Found 10 results");
    }
}