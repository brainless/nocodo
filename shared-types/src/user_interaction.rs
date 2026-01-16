// User interaction types shared between agents, api and gui

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::default::Default;
use ts_rs::TS;

/// Ask the user a list of questions to gather information or confirm actions
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
pub struct AskUserRequest {
    /// List of questions to ask the user
    pub questions: Vec<UserQuestion>,
}

/// Individual question to ask the user
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
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
}

/// Type of question and expected response format
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum QuestionType {
    /// Simple text input
    Text,
    // TODO: Enable these variants when needed
    // /// Numeric input (integer or float)
    // Number,
    // /// Yes/No boolean question
    // Boolean,
    // /// Single choice from multiple options
    // Select,
    // /// Multiple choices from options
    // Multiselect,
    // /// Password input (masked)
    // Password,
    // /// File path input
    // FilePath,
    // /// Email address input
    // Email,
    // /// URL input
    // Url,
}

/// Response from the ask_user tool containing user answers
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AskUserResponse {
    /// Whether the user responded to all required questions
    pub completed: bool,
    /// User's responses to each question
    pub responses: Vec<UserQuestionResponse>,
    /// Any error or status message
    pub message: String,
}

/// Individual user response to a question
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct UserQuestionResponse {
    /// ID of the question being answered
    pub question_id: String,
    /// The user's answer
    pub answer: String,
}

impl AskUserRequest {
    /// Generate example JSON schema for this request type
    pub fn example_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "questions": {
                    "type": "array",
                    "description": "List of questions to ask the user (empty array if no clarification needed)",
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
                                "enum": ["text"]
                            },
                            "default": {
                                "type": "string",
                                "description": "Default value if user doesn't provide one"
                            },
                            "description": {
                                "type": "string",
                                "description": "Additional description or help text for the question"
                            }
                        },
                        "required": ["id", "question", "type"]
                    }
                }
            },
            "required": ["questions"]
        })
    }

    /// Validate the request parameters
    pub fn validate(&self) -> Result<(), String> {
        // Empty questions list is now valid - means no clarifications needed
        if self.questions.is_empty() {
            return Ok(());
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

            // TODO: Re-enable validation for select/multiselect when those types are enabled
            // match question.response_type {
            //     QuestionType::Select | QuestionType::Multiselect => {
            //         if question
            //             .options
            //             .as_ref()
            //             .map_or(true, |opts| opts.is_empty())
            //         {
            //             return Err(format!(
            //                 "Question '{}' of type {:?} requires at least one option",
            //                 question.id, question.response_type
            //             ));
            //         }
            //     }
            //     _ => {}
            // }
        }

        Ok(())
    }
}
