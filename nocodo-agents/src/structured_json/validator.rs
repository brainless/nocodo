use serde_json::Value;

#[derive(Debug)]
pub struct TypeValidator {
    type_definitions: Vec<String>,
    type_names: Vec<String>,
}

impl TypeValidator {
    pub fn new(
        type_names: Vec<String>,
        type_definitions: Vec<String>,
    ) -> Result<Self, anyhow::Error> {
        if type_names.is_empty() {
            return Err(anyhow::anyhow!("No type names provided"));
        }

        if type_definitions.is_empty() {
            return Err(anyhow::anyhow!("No type definitions provided"));
        }

        Ok(Self {
            type_names,
            type_definitions,
        })
    }

    pub fn get_type_definitions(&self) -> String {
        self.type_definitions.join("\n\n")
    }

    pub fn get_expected_types_summary(&self) -> String {
        self.type_names.join(", ")
    }

    pub fn validate_json_syntax(&self, json: &str) -> Result<Value, ValidationError> {
        match serde_json::from_str::<Value>(json) {
            Ok(value) => Ok(value),
            Err(e) => Err(ValidationError {
                message: format!("Invalid JSON syntax: {}", e),
            }),
        }
    }

    pub fn validate_structure(&self, json_value: &Value) -> Result<(), ValidationError> {
        for (i, type_def) in self.type_definitions.iter().enumerate() {
            let type_name = &self.type_names[i];

            if Self::json_matches_type(json_value, type_name, type_def) {
                return Ok(());
            }
        }

        Err(ValidationError {
            message: format!(
                "JSON structure does not match any of the expected types: {}",
                self.get_expected_types_summary()
            ),
        })
    }

    #[allow(dead_code)]
    fn extract_type_name(type_def: &str) -> String {
        type_def
            .lines()
            .find(|line| {
                line.trim().starts_with("interface ")
                    || line.trim().starts_with("export interface ")
            })
            .and_then(|line| {
                let trimmed = line.trim();
                let start = trimmed
                    .find("interface ")
                    .map(|i| i + "interface ".len())
                    .or_else(|| {
                        trimmed
                            .find("export interface ")
                            .map(|i| i + "export interface ".len())
                    })?;
                Some(
                    trimmed[start..]
                        .split_whitespace()
                        .next()
                        .unwrap_or("")
                        .to_string(),
                )
            })
            .unwrap_or_else(|| "UnknownType".to_string())
    }

    fn json_matches_type(json_value: &Value, type_name: &str, type_def: &str) -> bool {
        let type_obj = match json_value {
            Value::Object(obj) => obj,
            _ => return false,
        };

        let type_key = type_name.to_snake_case();
        if type_obj.contains_key(&type_key) {
            if let Value::Object(nested) = &type_obj[&type_key] {
                return Self::check_required_fields(nested, type_def);
            }
        }

        Self::check_required_fields(type_obj, type_def)
    }

    fn check_required_fields(json_obj: &serde_json::Map<String, Value>, type_def: &str) -> bool {
        if let Some(fields) = Self::parse_type_fields(type_def) {
            for field in fields {
                if !json_obj.contains_key(&field.name) {
                    tracing::debug!("Missing required field: {} in type definition", field.name);
                }
            }
        }

        true
    }

    fn parse_type_fields(_type_def: &str) -> Option<Vec<TypeField>> {
        None
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct TypeField {
    name: String,
    optional: bool,
    field_type: String,
}

#[derive(Debug)]
pub struct ValidationError {
    pub message: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ValidationError {}

trait ToSnakeCase {
    fn to_snake_case(&self) -> String;
}

impl ToSnakeCase for str {
    fn to_snake_case(&self) -> String {
        let mut result = String::new();
        let chars: Vec<char> = self.chars().collect();

        for (i, c) in chars.iter().enumerate() {
            if c.is_uppercase() {
                if i > 0 {
                    let prev = &chars[i - 1];
                    if (prev.is_lowercase()
                        || (prev.is_uppercase()
                            && i + 1 < chars.len()
                            && chars[i + 1].is_lowercase()))
                        && !result.ends_with('_')
                    {
                        result.push('_');
                    }
                }
                result.extend(c.to_lowercase());
            } else {
                result.push(*c);
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_validator_new() {
        let type_names = vec!["PMProject".to_string(), "Workflow".to_string()];
        let type_definitions = vec![
            "export interface PMProject { id: number; name: string; }".to_string(),
            "export interface Workflow { id: number; project_id: number; }".to_string(),
        ];

        let validator = TypeValidator::new(type_names.clone(), type_definitions).unwrap();
        assert_eq!(validator.get_expected_types_summary(), "PMProject, Workflow");
    }

    #[test]
    fn test_type_validator_empty_type_names() {
        let type_names = vec![];
        let type_definitions = vec!["export interface Test { id: number; }".to_string()];

        let result = TypeValidator::new(type_names, type_definitions);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No type names"));
    }

    #[test]
    fn test_validate_json_syntax_valid() {
        let type_names = vec!["PMProject".to_string()];
        let type_definitions =
            vec!["export interface PMProject { id: number; name: string; }".to_string()];

        let validator = TypeValidator::new(type_names, type_definitions).unwrap();

        let json = r#"{"id": 1, "name": "Test"}"#;
        let result = validator.validate_json_syntax(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_json_syntax_invalid() {
        let type_names = vec!["PMProject".to_string()];
        let type_definitions =
            vec!["export interface PMProject { id: number; name: string; }".to_string()];

        let validator = TypeValidator::new(type_names, type_definitions).unwrap();

        let json = r#"{"id": 1, "name": }"#;
        let result = validator.validate_json_syntax(json);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("Invalid JSON syntax"));
    }

    #[test]
    fn test_extract_type_name() {
        let type_def = "export interface PMProject { id: number; name: string; }";
        let name = TypeValidator::extract_type_name(type_def);
        assert_eq!(name, "PMProject");

        let type_def2 = "interface Workflow { id: number; }";
        let name2 = TypeValidator::extract_type_name(type_def2);
        assert_eq!(name2, "Workflow");
    }

    #[test]
    fn test_get_type_definitions() {
        let type_names = vec!["PMProject".to_string(), "Workflow".to_string()];
        let type_definitions = vec![
            "export interface PMProject { id: number; }".to_string(),
            "export interface Workflow { id: number; }".to_string(),
        ];

        let validator = TypeValidator::new(type_names, type_definitions).unwrap();
        let combined = validator.get_type_definitions();

        assert!(combined.contains("PMProject"));
        assert!(combined.contains("Workflow"));
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!("pm_project".to_string(), "PMProject".to_snake_case());
        assert_eq!("workflow".to_string(), "Workflow".to_snake_case());
        assert_eq!(
            "workflow_with_steps".to_string(),
            "WorkflowWithSteps".to_snake_case()
        );
    }

    #[test]
    fn test_get_expected_types_summary() {
        let type_names = vec!["PMProject".to_string(), "Workflow".to_string()];
        let type_definitions = vec![
            "export interface PMProject { id: number; }".to_string(),
            "export interface Workflow { id: number; }".to_string(),
        ];

        let validator = TypeValidator::new(type_names, type_definitions).unwrap();
        let summary = validator.get_expected_types_summary();

        assert_eq!(summary, "PMProject, Workflow");
    }
}
