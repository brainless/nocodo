use config::{Config, ConfigError, File};
use serde::Deserialize;
use std::path::Path;

/// Configuration structure for prompts loaded from TOML
#[derive(Debug, Deserialize, Clone)]
pub struct PromptsConfig {
    pub tech_stack_analysis: TechStackAnalysisPrompt,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TechStackAnalysisPrompt {
    pub prompt: String,
}

impl PromptsConfig {
    /// Load prompts from TOML file
    pub fn load_from_file(config_path: &Path) -> Result<Self, ConfigError> {
        if !config_path.exists() {
            return Err(ConfigError::Message(format!(
                "Prompts configuration file not found: {}",
                config_path.display()
            )));
        }

        let builder = Config::builder()
            .add_source(File::from(config_path.to_path_buf()))
            .build()?;

        builder.try_deserialize()
    }
}

/// Test scenario with expected keywords for validation
#[derive(Debug, Clone)]
pub struct LlmTestScenario {
    pub name: String,
    pub context: LlmTestContext,
    pub prompt: String,
    pub expected_keywords: LlmKeywordExpectations,
}

/// Context for LLM test (git repository)
#[derive(Debug, Clone)]
pub struct LlmTestContext {
    pub git_repo: String,
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
    /// Create a tech stack analysis test for Saleor
    pub fn tech_stack_analysis_saleor() -> Self {
        // Load prompt from TOML file - fixed path relative to manager directory
        let prompts_path = std::path::Path::new("prompts/default.toml");
        let prompt = PromptsConfig::load_from_file(prompts_path)
            .map(|config| config.tech_stack_analysis.prompt)
            .unwrap_or_else(|_| {
                // Fallback to hardcoded prompt if TOML loading fails
                "What is the tech stack of this project? You must examine at least 3 different configuration files (such as package.json, pyproject.toml, manage.py, requirements.txt, or setup.py) before providing your final answer. Please read each file individually and then provide a comprehensive analysis of all technologies found. Return a simple array of technologies only at the end.".to_string()
            });

        Self {
            name: "Tech Stack Analysis - Saleor".to_string(),
            context: LlmTestContext {
                git_repo: "git@github.com:saleor/saleor.git".to_string(),
            },
            prompt,
            expected_keywords: LlmKeywordExpectations {
                required_keywords: vec![
                    "Django".to_string(),
                    "Python".to_string(),
                    "PostgreSQL".to_string(),
                    "GraphQL".to_string(),
                ],
                optional_keywords: vec!["JavaScript".to_string(), "Node".to_string()],
                forbidden_keywords: vec![],
                minimum_score: 0.7,
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
    fn test_saleor_scenario_creation() {
        let scenario = LlmTestScenario::tech_stack_analysis_saleor();

        assert_eq!(scenario.name, "Tech Stack Analysis - Saleor");
        assert_eq!(
            scenario.context.git_repo,
            "git@github.com:saleor/saleor.git"
        );

        // The prompt should now be loaded from TOML or use fallback
        assert!(!scenario.prompt.is_empty());
        assert!(scenario.prompt.contains("tech stack"));

        assert_eq!(scenario.expected_keywords.required_keywords.len(), 4);
        assert!(scenario
            .expected_keywords
            .required_keywords
            .contains(&"Django".to_string()));
        assert!(scenario
            .expected_keywords
            .required_keywords
            .contains(&"Python".to_string()));
        assert!(scenario
            .expected_keywords
            .required_keywords
            .contains(&"PostgreSQL".to_string()));
        assert!(scenario
            .expected_keywords
            .required_keywords
            .contains(&"GraphQL".to_string()));

        assert_eq!(scenario.expected_keywords.optional_keywords.len(), 2);
        assert_eq!(scenario.expected_keywords.forbidden_keywords.len(), 0);
    }

    #[test]
    fn test_prompts_config_loading() {
        // Fixed path relative to manager directory
        let prompts_path = std::path::Path::new("prompts/default.toml");
        let result = PromptsConfig::load_from_file(prompts_path);

        // Should be able to load the TOML file
        assert!(
            result.is_ok(),
            "Failed to load prompts config: {:?}",
            result.err()
        );

        let config = result.unwrap();
        assert!(!config.tech_stack_analysis.prompt.is_empty());
        assert!(config.tech_stack_analysis.prompt.contains("tech stack"));
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
