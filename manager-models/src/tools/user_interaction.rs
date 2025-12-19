// Tool request and response types for user interaction

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::default::Default;

/// Ask the user a list of questions to gather information or confirm actions
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AskUserRequest {
    /// The main prompt or context for the questions
    pub prompt: String,
    /// List of questions to ask the user
    pub questions: Vec<UserQuestion>,
    /// Whether the user responses are required (true) or optional (false)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    /// Optional timeout in seconds for user response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,
}

/// Individual question to ask the user
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UserQuestion {
    /// Unique identifier for this question
    pub id: String,
    /// The question text to display to the user
    pub question: String,
    /// Type of response expected
    #[serde(rename = "type")]
    pub response_type: QuestionType,
    /// Default value if user doesn't provide one
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    /// List of possible options for multiple choice or select questions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
    /// Additional description or help text for the question
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Validation rules for the response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation: Option<QuestionValidation>,
}

/// Type of question and expected response format
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QuestionType {
    /// Simple text input
    Text,
    /// Numeric input (integer or float)
    Number,
    /// Yes/No boolean question
    Boolean,
    /// Single choice from multiple options
    Select,
    /// Multiple choices from options
    Multiselect,
    /// Password input (masked)
    Password,
    /// File path input
    FilePath,
    /// Email address input
    Email,
    /// URL input
    Url,
}

/// Validation rules for question responses
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default)]
pub struct QuestionValidation {
    /// Minimum length for text responses
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<usize>,
    /// Maximum length for text responses
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<usize>,
    /// Minimum value for numeric responses
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value: Option<f64>,
    /// Maximum value for numeric responses
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_value: Option<f64>,
    /// Regular expression pattern for text validation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    /// Custom validation error message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}



/// Response from the ask_user tool containing user answers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AskUserResponse {
    /// Whether the user responded to all required questions
    pub completed: bool,
    /// User's responses to each question
    pub responses: Vec<UserQuestionResponse>,
    /// Any error or status message
    pub message: String,
    /// How long the user took to respond (in seconds)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_time_secs: Option<f64>,
}

/// Individual user response to a question
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserQuestionResponse {
    /// ID of the question being answered
    pub question_id: String,
    /// The user's answer
    pub answer: String,
    /// Whether the response is valid according to validation rules
    pub valid: bool,
    /// Validation error message if response is invalid
    #[serde(skip_serializing_if = "Option::is_none")]
    pub validation_error: Option<String>,
}

impl AskUserRequest {
    /// Generate example JSON schema for this request type
    pub fn example_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "prompt": {
                    "type": "string",
                    "description": "The main prompt or context for the questions"
                },
                "questions": {
                    "type": "array",
                    "description": "List of questions to ask the user",
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": {
                                "type": "string",
                                "description": "Unique identifier for this question"
                            },
                            "question": {
                                "type": "string",
                                "description": "The question text to display to the user"
                            },
                            "type": {
                                "type": "string",
                                "description": "Type of response expected",
                                "enum": ["text", "number", "boolean", "select", "multiselect", "password", "file_path", "email", "url"]
                            },
                            "default": {
                                "type": "string",
                                "description": "Default value if user doesn't provide one"
                            },
                            "options": {
                                "type": "array",
                                "description": "List of possible options for multiple choice questions",
                                "items": {
                                    "type": "string"
                                }
                            },
                            "description": {
                                "type": "string",
                                "description": "Additional description or help text for the question"
                            },
                            "validation": {
                                "type": "object",
                                "description": "Validation rules for the response",
                                "properties": {
                                    "min_length": {
                                        "type": "number",
                                        "description": "Minimum length for text responses"
                                    },
                                    "max_length": {
                                        "type": "number",
                                        "description": "Maximum length for text responses"
                                    },
                                    "min_value": {
                                        "type": "number",
                                        "description": "Minimum value for numeric responses"
                                    },
                                    "max_value": {
                                        "type": "number",
                                        "description": "Maximum value for numeric responses"
                                    },
                                    "pattern": {
                                        "type": "string",
                                        "description": "Regular expression pattern for text validation"
                                    },
                                    "error_message": {
                                        "type": "string",
                                        "description": "Custom validation error message"
                                    }
                                }
                            }
                        },
                        "required": ["id", "question", "type"]
                    }
                },
                "required": {
                    "type": "boolean",
                    "description": "Whether the user responses are required",
                    "default": true
                },
                "timeout_secs": {
                    "type": "number",
                    "description": "Optional timeout in seconds for user response"
                }
            },
            "required": ["prompt", "questions"]
        })
    }

    /// Validate the request parameters
    pub fn validate(&self) -> Result<(), String> {
        if self.questions.is_empty() {
            return Err("At least one question must be provided".to_string());
        }

        let mut question_ids = std::collections::HashSet::new();
        for (index, question) in self.questions.iter().enumerate() {
            if question.id.is_empty() {
                return Err(format!("Question at index {} has empty ID", index));
            }

            if question.question.is_empty() {
                return Err(format!(
                    "Question '{}' at index {} has empty text",
                    question.id, index
                ));
            }

            if question_ids.contains(&question.id) {
                return Err(format!("Duplicate question ID: {}", question.id));
            }
            question_ids.insert(question.id.clone());

            // Validate select/multiselect questions have options
            match question.response_type {
                QuestionType::Select | QuestionType::Multiselect => {
                    if question
                        .options
                        .as_ref()
                        .map_or(true, |opts| opts.is_empty())
                    {
                        return Err(format!(
                            "Question '{}' of type {:?} requires at least one option",
                            question.id, question.response_type
                        ));
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }
}
