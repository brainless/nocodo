use schemars::schema::Schema;
use std::collections::BTreeSet;

/// Trait for model-specific JSON schema customization
///
/// Different LLM providers have different requirements for JSON schemas:
/// - OpenAI strict mode requires ALL fields in the `required` array
/// - Anthropic Claude, GLM, and others respect accurate required/optional distinction
pub trait SchemaProvider {
    /// Provider name for logging
    #[allow(dead_code)]
    fn name(&self) -> &str;

    /// Whether this provider requires all fields to be marked as required
    /// (e.g., OpenAI with strict=true)
    fn requires_all_fields(&self) -> bool;

    /// Customize a schema for this provider
    fn customize_schema(&self, schema: Schema) -> Schema {
        if self.requires_all_fields() {
            self.mark_all_required(schema)
        } else {
            schema
        }
    }

    /// Mark all properties in a schema as required
    fn mark_all_required(&self, schema: Schema) -> Schema {
        match schema {
            Schema::Object(mut obj) => {
                if let Some(object_validation) = &mut obj.object {
                    // Collect all property names
                    let all_props: BTreeSet<String> = object_validation
                        .properties
                        .keys()
                        .cloned()
                        .collect();

                    // Mark all as required
                    object_validation.required = all_props;
                }
                Schema::Object(obj)
            }
            _ => schema,
        }
    }
}

/// OpenAI provider - requires all fields as required in strict mode
pub struct OpenAiSchemaProvider;

impl SchemaProvider for OpenAiSchemaProvider {
    fn name(&self) -> &str {
        "openai"
    }

    fn requires_all_fields(&self) -> bool {
        true
    }
}

/// Anthropic Claude provider - respects actual required/optional fields
pub struct AnthropicSchemaProvider;

impl SchemaProvider for AnthropicSchemaProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    fn requires_all_fields(&self) -> bool {
        false
    }
}

/// GLM (Zhipu AI) provider - respects actual required/optional fields
pub struct GlmSchemaProvider;

impl SchemaProvider for GlmSchemaProvider {
    fn name(&self) -> &str {
        "glm"
    }

    fn requires_all_fields(&self) -> bool {
        false
    }
}

/// xAI Grok provider - respects actual required/optional fields
pub struct XaiSchemaProvider;

impl SchemaProvider for XaiSchemaProvider {
    fn name(&self) -> &str {
        "xai"
    }

    fn requires_all_fields(&self) -> bool {
        false
    }
}

/// Get the appropriate schema provider for a given LLM provider name
pub fn get_schema_provider(provider: &str) -> Box<dyn SchemaProvider> {
    match provider.to_lowercase().as_str() {
        "openai" => Box::new(OpenAiSchemaProvider),
        "anthropic" | "claude" => Box::new(AnthropicSchemaProvider),
        "zai" | "glm" | "zhipu" => Box::new(GlmSchemaProvider),
        "xai" | "grok" => Box::new(XaiSchemaProvider),
        _ => {
            tracing::warn!(
                provider = %provider,
                "Unknown provider, defaulting to flexible schema (not marking all fields as required)"
            );
            Box::new(GlmSchemaProvider) // Default to flexible
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use schemars::{schema_for, JsonSchema};
    use serde::{Deserialize, Serialize};

    #[derive(JsonSchema, Serialize, Deserialize)]
    struct TestStruct {
        required_field: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        optional_field: Option<String>,
    }

    #[test]
    fn test_openai_marks_all_required() {
        let provider = OpenAiSchemaProvider;
        let schema = schema_for!(TestStruct);
        let customized = provider.customize_schema(schema.schema.into());

        if let Schema::Object(obj) = customized {
            if let Some(object_validation) = obj.object {
                assert_eq!(object_validation.required.len(), 2);
                assert!(object_validation.required.contains("required_field"));
                assert!(object_validation.required.contains("optional_field"));
            }
        }
    }

    #[test]
    fn test_anthropic_respects_optional() {
        let provider = AnthropicSchemaProvider;
        let schema = schema_for!(TestStruct);
        let original_required_count = if let Some(ref obj) = schema.schema.object {
            obj.required.len()
        } else {
            0
        };

        let customized = provider.customize_schema(schema.schema.into());

        if let Schema::Object(obj) = customized {
            if let Some(object_validation) = obj.object {
                // Should only have 1 required field (required_field)
                assert_eq!(object_validation.required.len(), original_required_count);
                assert!(object_validation.required.contains("required_field"));
            }
        }
    }

    #[test]
    fn test_get_schema_provider() {
        assert_eq!(get_schema_provider("openai").name(), "openai");
        assert_eq!(get_schema_provider("anthropic").name(), "anthropic");
        assert_eq!(get_schema_provider("glm").name(), "glm");
        assert_eq!(get_schema_provider("xai").name(), "xai");
    }
}
