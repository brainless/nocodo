/// Test scenario with expected keywords for validation
#[derive(Debug, Clone)]
pub struct LlmTestScenario {
    pub name: String,
    pub context: LlmTestContext,
    pub prompt: String,
    pub expected_keywords: LlmKeywordExpectations,
}

/// Context for LLM test (project files and structure)
#[derive(Debug, Clone)]
pub struct LlmTestContext {
    pub files: Vec<TestFile>,
}

/// Test file definition
#[derive(Debug, Clone)]
pub struct TestFile {
    pub path: String,
    pub content: String,
    pub language: String,
}

/// Keyword expectations for validation
#[derive(Debug, Clone)]
pub struct LlmKeywordExpectations {
    pub required_keywords: Vec<String>,  // Must contain ALL of these
    pub optional_keywords: Vec<String>,  // Should contain SOME of these
    pub forbidden_keywords: Vec<String>, // Must NOT contain these
    pub minimum_score: f32,              // Keyword coverage threshold (0.7)
}

/// Result of keyword validation
#[derive(Debug)]
pub struct LlmValidationResult {
    pub passed: bool,
    pub score: f32,
    pub found_required: Vec<String>,
    pub found_optional: Vec<String>,
    pub found_forbidden: Vec<String>,
    pub missing_required: Vec<String>,
}

/// Keyword validator for LLM responses
pub struct KeywordValidator;

impl KeywordValidator {
    /// Validate an LLM response against keyword expectations
    pub fn validate_response(
        response: &str,
        expectations: &LlmKeywordExpectations,
    ) -> LlmValidationResult {
        let response_lower = response.to_lowercase();

        // Check required keywords (ALL must be present)
        let found_required: Vec<_> = expectations
            .required_keywords
            .iter()
            .filter(|k| Self::contains_keyword(&response_lower, k))
            .cloned()
            .collect();

        // Check optional keywords (SOME should be present)
        let found_optional: Vec<_> = expectations
            .optional_keywords
            .iter()
            .filter(|k| Self::contains_keyword(&response_lower, k))
            .cloned()
            .collect();

        // Check forbidden keywords (NONE should be present)
        let found_forbidden: Vec<_> = expectations
            .forbidden_keywords
            .iter()
            .filter(|k| Self::contains_keyword(&response_lower, k))
            .cloned()
            .collect();

        let missing_required: Vec<_> = expectations
            .required_keywords
            .iter()
            .filter(|k| !Self::contains_keyword(&response_lower, k))
            .cloned()
            .collect();

        let score = Self::calculate_score(
            &found_required,
            &found_optional,
            &found_forbidden,
            expectations,
        );

        let passed = found_required.len() == expectations.required_keywords.len()
            && found_forbidden.is_empty()
            && score >= expectations.minimum_score;

        LlmValidationResult {
            passed,
            score,
            found_required,
            found_optional,
            found_forbidden,
            missing_required,
        }
    }

    /// Check if text contains a keyword (with fuzzy matching)
    fn contains_keyword(text: &str, keyword: &str) -> bool {
        let keyword_lower = keyword.to_lowercase();

        // Exact match
        if text.contains(&keyword_lower) {
            return true;
        }

        // Fuzzy matching for common variations
        match keyword_lower.as_str() {
            "fastapi" => text.contains("fast api"),
            "typescript" => text.contains("ts"),
            "javascript" => text.contains("js"),
            "react" => text.contains("reactjs"),
            "python" => text.contains("py"),
            "rust" => text.contains("cargo") || text.contains("rustc"),
            "node" => text.contains("nodejs"),
            "docker" => text.contains("container"),
            _ => false,
        }
    }

    /// Calculate a score based on keyword matches
    fn calculate_score(
        found_required: &[String],
        found_optional: &[String],
        found_forbidden: &[String],
        expectations: &LlmKeywordExpectations,
    ) -> f32 {
        let required_score = if expectations.required_keywords.is_empty() {
            1.0
        } else {
            found_required.len() as f32 / expectations.required_keywords.len() as f32
        };

        let optional_score = if expectations.optional_keywords.is_empty() {
            0.0
        } else {
            found_optional.len() as f32 / expectations.optional_keywords.len() as f32
        };

        let forbidden_penalty = found_forbidden.len() as f32 * 0.1;

        ((required_score * 0.7) + (optional_score * 0.2) - forbidden_penalty).clamp(0.0, 1.0)
    }
}

impl LlmTestScenario {
    /// Create a tech stack analysis test for Python FastAPI + React
    pub fn tech_stack_analysis_python_fastapi() -> Self {
        Self {
            name: "Tech Stack Analysis - Python FastAPI + React".to_string(),
            context: LlmTestContext {
                files: vec![
                    TestFile {
                        path: "requirements.txt".to_string(),
                        content: "fastapi==0.104.1\nuvicorn==0.24.0\npydantic==2.4.0".to_string(),
                        language: "text".to_string(),
                    },
                    TestFile {
                        path: "main.py".to_string(),
                        content: "from fastapi import FastAPI\nfrom pydantic import BaseModel\n\napp = FastAPI()\n\nclass Item(BaseModel):\n    name: str\n    price: float\n\n@app.get(\"/\")\ndef read_root():\n    return {\"Hello\": \"World\"}".to_string(),
                        language: "python".to_string(),
                    },
                    TestFile {
                        path: "package.json".to_string(),
                        content: r#"{"dependencies": {"react": "^18.2.0", "typescript": "^5.0.0", "@types/react": "^18.2.0"}}"#.to_string(),
                        language: "json".to_string(),
                    },
                    TestFile {
                        path: "src/App.tsx".to_string(),
                        content: "import React from 'react';\n\nfunction App() {\n  return (\n    <div className=\"App\">\n      <h1>Hello FastAPI + React!</h1>\n    </div>\n  );\n}\n\nexport default App;".to_string(),
                        language: "typescript".to_string(),
                    },
                ],
            },
            prompt: "Analyze the tech stack of this project. What technologies and frameworks are being used?".to_string(),
            expected_keywords: LlmKeywordExpectations {
                required_keywords: vec!["Python".to_string(), "FastAPI".to_string(), "React".to_string()],
                optional_keywords: vec!["TypeScript".to_string(), "full-stack".to_string(), "API".to_string(), "Pydantic".to_string(), "Uvicorn".to_string()],
                forbidden_keywords: vec!["Django".to_string(), "Vue".to_string(), "Java".to_string(), "Spring".to_string()],
                minimum_score: 0.7,
            },
        }
    }

    /// Create a Rust project analysis test
    pub fn tech_stack_analysis_rust() -> Self {
        Self {
            name: "Tech Stack Analysis - Rust Project".to_string(),
            context: LlmTestContext {
                files: vec![
                    TestFile {
                        path: "Cargo.toml".to_string(),
                        content: "[package]\nname = \"test-project\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\ntokio = { version = \"1.0\", features = [\"full\"] }\nserde = { version = \"1.0\", features = [\"derive\"] }\nactix-web = \"4.4\"".to_string(),
                        language: "toml".to_string(),
                    },
                    TestFile {
                        path: "src/main.rs".to_string(),
                        content: "use actix_web::{web, App, HttpServer, Result};\nuse serde::{Deserialize, Serialize};\n\n#[derive(Serialize, Deserialize)]\nstruct ApiResponse {\n    message: String,\n}\n\nasync fn hello() -> Result<web::Json<ApiResponse>> {\n    Ok(web::Json(ApiResponse {\n        message: \"Hello, Rust!\".to_string(),\n    }))\n}\n\n#[actix_web::main]\nasync fn main() -> std::io::Result<()> {\n    HttpServer::new(|| {\n        App::new()\n            .route(\"/\", web::get().to(hello))\n    })\n    .bind(\"127.0.0.1:8080\")?\n    .run()\n    .await\n}".to_string(),
                        language: "rust".to_string(),
                    },
                ],
            },
            prompt: "Analyze the tech stack of this project. What technologies and frameworks are being used?".to_string(),
            expected_keywords: LlmKeywordExpectations {
                required_keywords: vec!["Rust".to_string(), "Actix".to_string(), "Tokio".to_string()],
                optional_keywords: vec!["web server".to_string(), "async".to_string(), "Serde".to_string(), "HTTP".to_string()],
                forbidden_keywords: vec!["Python".to_string(), "JavaScript".to_string(), "Django".to_string(), "Express".to_string()],
                minimum_score: 0.7,
            },
        }
    }

    /// Create a code generation test
    pub fn code_generation_rust_function() -> Self {
        Self {
            name: "Code Generation - Rust Factorial Function".to_string(),
            context: LlmTestContext {
                files: vec![
                    TestFile {
                        path: "Cargo.toml".to_string(),
                        content: "[package]\nname = \"factorial-project\"\nversion = \"0.1.0\"\nedition = \"2021\"".to_string(),
                        language: "toml".to_string(),
                    },
                    TestFile {
                        path: "src/lib.rs".to_string(),
                        content: "// Empty library file for factorial function".to_string(),
                        language: "rust".to_string(),
                    },
                ],
            },
            prompt: "Write a factorial function in Rust that calculates n! for a given number n.".to_string(),
            expected_keywords: LlmKeywordExpectations {
                required_keywords: vec!["fn".to_string(), "factorial".to_string()],
                optional_keywords: vec!["recursion".to_string(), "u64".to_string(), "match".to_string(), "loop".to_string(), "pub".to_string()],
                forbidden_keywords: vec!["function".to_string(), "def".to_string(), "public".to_string(), "int".to_string()],
                minimum_score: 0.6,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyword_validation_success() {
        let expectations = LlmKeywordExpectations {
            required_keywords: vec!["Python".to_string(), "FastAPI".to_string()],
            optional_keywords: vec!["React".to_string(), "TypeScript".to_string()],
            forbidden_keywords: vec!["Django".to_string()],
            minimum_score: 0.7,
        };

        let response =
            "This is a Python web application using FastAPI framework with React frontend";
        let result = KeywordValidator::validate_response(response, &expectations);

        assert!(result.passed);
        assert_eq!(result.found_required.len(), 2);
        assert!(result.found_required.contains(&"Python".to_string()));
        assert!(result.found_required.contains(&"FastAPI".to_string()));
        assert_eq!(result.found_optional.len(), 1);
        assert!(result.found_optional.contains(&"React".to_string()));
        assert_eq!(result.found_forbidden.len(), 0);
        assert!(result.score >= 0.7);
    }

    #[test]
    fn test_keyword_validation_missing_required() {
        let expectations = LlmKeywordExpectations {
            required_keywords: vec!["Python".to_string(), "FastAPI".to_string()],
            optional_keywords: vec!["React".to_string()],
            forbidden_keywords: vec!["Django".to_string()],
            minimum_score: 0.7,
        };

        let response = "This is a Python web application using Django framework";
        let result = KeywordValidator::validate_response(response, &expectations);

        assert!(!result.passed);
        assert_eq!(result.found_required.len(), 1); // Only Python found
        assert_eq!(result.missing_required.len(), 1); // FastAPI missing
        assert_eq!(result.found_forbidden.len(), 1); // Django found
    }

    #[test]
    fn test_fuzzy_keyword_matching() {
        let expectations = LlmKeywordExpectations {
            required_keywords: vec!["FastAPI".to_string(), "TypeScript".to_string()],
            optional_keywords: vec![],
            forbidden_keywords: vec![],
            minimum_score: 0.5,
        };

        let response = "This uses Fast API and TS for development";
        let result = KeywordValidator::validate_response(response, &expectations);

        assert!(result.passed);
        assert_eq!(result.found_required.len(), 2);
    }

    #[test]
    fn test_tech_stack_scenario_creation() {
        let scenario = LlmTestScenario::tech_stack_analysis_python_fastapi();

        assert_eq!(
            scenario.name,
            "Tech Stack Analysis - Python FastAPI + React"
        );
        assert!(scenario.prompt.contains("tech stack"));
        assert_eq!(scenario.context.files.len(), 4);
        assert!(scenario.context.files.iter().any(|f| f.path == "main.py"));
        assert!(scenario
            .context
            .files
            .iter()
            .any(|f| f.path == "package.json"));

        assert_eq!(scenario.expected_keywords.required_keywords.len(), 3);
        assert!(scenario
            .expected_keywords
            .required_keywords
            .contains(&"Python".to_string()));
        assert!(scenario
            .expected_keywords
            .required_keywords
            .contains(&"FastAPI".to_string()));
        assert!(scenario
            .expected_keywords
            .required_keywords
            .contains(&"React".to_string()));
    }

    #[test]
    fn test_rust_scenario_creation() {
        let scenario = LlmTestScenario::tech_stack_analysis_rust();

        assert_eq!(scenario.name, "Tech Stack Analysis - Rust Project");
        assert_eq!(scenario.context.files.len(), 2);
        assert!(scenario
            .context
            .files
            .iter()
            .any(|f| f.path == "Cargo.toml"));
        assert!(scenario
            .context
            .files
            .iter()
            .any(|f| f.path == "src/main.rs"));

        assert!(scenario
            .expected_keywords
            .required_keywords
            .contains(&"Rust".to_string()));
        assert!(scenario
            .expected_keywords
            .required_keywords
            .contains(&"Actix".to_string()));
        assert!(scenario
            .expected_keywords
            .required_keywords
            .contains(&"Tokio".to_string()));
    }

    #[test]
    fn test_score_calculation() {
        let expectations = LlmKeywordExpectations {
            required_keywords: vec!["A".to_string(), "B".to_string()],
            optional_keywords: vec!["C".to_string(), "D".to_string()],
            forbidden_keywords: vec![],
            minimum_score: 0.5,
        };

        // All required, some optional
        let found_required = vec!["A".to_string(), "B".to_string()];
        let found_optional = vec!["C".to_string()];
        let found_forbidden = vec![];

        let score = KeywordValidator::calculate_score(
            &found_required,
            &found_optional,
            &found_forbidden,
            &expectations,
        );

        // Required: 2/2 = 1.0 (weight 0.7)
        // Optional: 1/2 = 0.5 (weight 0.2)
        // Expected: 1.0 * 0.7 + 0.5 * 0.2 = 0.7 + 0.1 = 0.8
        assert!((score - 0.8).abs() < 0.01);
    }
}
