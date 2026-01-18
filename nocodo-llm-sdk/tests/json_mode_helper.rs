/// Helper module for JSON mode integration tests
///
/// Provides a consistent prompt and TypeScript type definitions
/// for testing JSON mode across all supported LLM providers.

/// Get the standard JSON mode prompt for testing
///
/// This prompt is consistent across all providers and uses
/// TypeScript type definitions to guide the model's output.
pub fn json_mode_prompt() -> String {
    r#"Extract the person information from this text: "John is 30 years old and lives in New York. He works as a software engineer."

You MUST respond with valid JSON that conforms to the following TypeScript interface:

interface PersonInfo {
    name: string;
    age: number;
    city: string;
    occupation: string;
}

Do not include any markdown code blocks. Do not include any explanations. Only return the JSON object."#.to_string()
}

/// Validate that a JSON string matches the expected structure
///
/// Returns Ok(()) if valid, Err(message) if invalid
pub fn validate_person_info_json(json_str: &str) -> Result<(), String> {
    // Parse the JSON
    let value: serde_json::Value =
        serde_json::from_str(json_str).map_err(|e| format!("Failed to parse JSON: {}", e))?;

    // Check it's an object
    let obj = value
        .as_object()
        .ok_or_else(|| "JSON is not an object".to_string())?;

    // Check required fields
    let name = obj
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing or invalid 'name' field (should be string)")?;

    let age = obj
        .get("age")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| "Missing or invalid 'age' field (should be number)")?;

    let city = obj
        .get("city")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing or invalid 'city' field (should be string)")?;

    let occupation = obj
        .get("occupation")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing or invalid 'occupation' field (should be string)")?;

    // Basic validation
    if name.is_empty() {
        return Err("Name cannot be empty".to_string());
    }

    if age < 0 || age > 150 {
        return Err(format!("Age {} is out of valid range", age));
    }

    if city.is_empty() {
        return Err("City cannot be empty".to_string());
    }

    if occupation.is_empty() {
        return Err("Occupation cannot be empty".to_string());
    }

    Ok(())
}

/// Get expected values for the PersonInfo extraction
///
/// Based on the test text: "John is 30 years old and lives in New York. He works as a software engineer."
pub fn expected_values() -> (String, i64, String, String) {
    (
        "John".to_string(),
        30,
        "New York".to_string(),
        "Software engineer".to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_mode_prompt_is_not_empty() {
        let prompt = json_mode_prompt();
        assert!(!prompt.is_empty());
        assert!(prompt.contains("TypeScript"));
        assert!(prompt.contains("interface PersonInfo"));
    }

    #[test]
    fn test_validate_valid_json() {
        let valid_json =
            r#"{"name":"John","age":30,"city":"New York","occupation":"Software engineer"}"#;
        assert!(validate_person_info_json(valid_json).is_ok());
    }

    #[test]
    fn test_validate_invalid_json() {
        let invalid_json =
            r#"{"name":"John","age":"30","city":"New York","occupation":"Software engineer"}"#;
        assert!(validate_person_info_json(invalid_json).is_err());
    }

    #[test]
    fn test_validate_missing_field() {
        let missing_field = r#"{"name":"John","city":"New York","occupation":"Software engineer"}"#;
        assert!(validate_person_info_json(missing_field).is_err());
    }

    #[test]
    fn test_expected_values() {
        let (name, age, city, occupation) = expected_values();
        assert_eq!(name, "John");
        assert_eq!(age, 30);
        assert_eq!(city, "New York");
        assert_eq!(occupation, "Software engineer");
    }
}
