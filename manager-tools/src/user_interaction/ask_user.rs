use crate::types::{
    AskUserRequest, AskUserResponse, QuestionType, ToolErrorResponse, ToolResponse, UserQuestion,
    UserQuestionResponse,
};
use anyhow::Result;
use regex;
use std::io::{self, Write};

pub async fn ask_user(request: AskUserRequest) -> Result<ToolResponse> {
    // Validate the request
    if let Err(error) = request.validate() {
        return Ok(ToolResponse::Error(ToolErrorResponse {
            tool: "ask_user".to_string(),
            error: "InvalidRequest".to_string(),
            message: error,
        }));
    }

    // Handle empty questions - no clarifications needed
    if request.questions.is_empty() {
        return Ok(ToolResponse::AskUser(AskUserResponse {
            completed: true,
            responses: vec![],
            message: "No clarifications needed".to_string(),
            response_time_secs: Some(0.0),
        }));
    }

    let start_time = std::time::Instant::now();
    let mut responses = Vec::new();
    let mut all_valid = true;

    // Ask each question
    for question in &request.questions {
        let answer = prompt_question(question)?;
        let validation_result = validate_response(question, &answer);
        let is_valid = validation_result.is_ok();

        responses.push(UserQuestionResponse {
            question_id: question.id.clone(),
            answer: answer.clone(),
            valid: is_valid,
            validation_error: validation_result.err(),
        });

        if !is_valid {
            all_valid = false;
        }
    }

    let response_time = start_time.elapsed().as_secs_f64();

    Ok(ToolResponse::AskUser(AskUserResponse {
        completed: all_valid,
        responses,
        message: if all_valid {
            "All questions answered successfully".to_string()
        } else {
            "Some questions have invalid responses".to_string()
        },
        response_time_secs: Some(response_time),
    }))
}

/// Prompt a single question to the user and get their response
fn prompt_question(question: &UserQuestion) -> Result<String> {
    let mut input = String::new();

    // Build the question prompt
    let mut prompt_parts = Vec::new();

    // Add the question text
    prompt_parts.push(format!("Q: {}", question.question));

    // Add description if provided
    if let Some(description) = &question.description {
        prompt_parts.push(format!("  {}", description));
    }

    // Add response type indicator - currently only Text is supported
    let response_indicator = match question.response_type {
        QuestionType::Text => " (text)",
        // TODO: Enable other types when needed
        // QuestionType::Number => " (number)",
        // QuestionType::Boolean => " (yes/no)",
        // QuestionType::Select => " (enter number)",
        // QuestionType::Multiselect => " (enter numbers, comma-separated)",
        // QuestionType::Password => " (password - will be hidden)",
        // QuestionType::FilePath => " (file path)",
        // QuestionType::Email => " (email)",
        // QuestionType::Url => " (url)",
    };

    // Combine all parts
    let full_prompt = prompt_parts.join("\n");
    println!("\n{}", full_prompt);

    // Add default value indicator
    let default_indicator = if let Some(default) = &question.default {
        format!(" [{}]", default)
    } else {
        String::new()
    };

    print!("A{}{}: ", response_indicator, default_indicator);
    io::stdout().flush()?;

    io::stdin().read_line(&mut input)?;

    // Handle empty input with default
    let trimmed = input.trim();
    let response = if trimmed.is_empty() {
        question.default.clone().unwrap_or_default()
    } else {
        trimmed.to_string()
    };

    Ok(response)
}

/// Validate a response against the question's validation rules
fn validate_response(question: &UserQuestion, response: &str) -> Result<(), String> {
    if let Some(validation) = &question.validation {
        // Length validation for text responses
        match question.response_type {
            QuestionType::Text => {
                if let Some(min_length) = validation.min_length {
                    if response.len() < min_length {
                        return Err(format!(
                            "Response too short (minimum {} characters)",
                            min_length
                        ));
                    }
                }
                if let Some(max_length) = validation.max_length {
                    if response.len() > max_length {
                        return Err(format!(
                            "Response too long (maximum {} characters)",
                            max_length
                        ));
                    }
                }
            } // TODO: Enable other types when needed
        }

        // Pattern validation
        if let Some(pattern) = &validation.pattern {
            if let Ok(regex) = regex::Regex::new(pattern) {
                if !regex.is_match(response) {
                    return Err(validation.error_message.clone().unwrap_or_else(|| {
                        format!("Response does not match required pattern: {}", pattern)
                    }));
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ask_user_request_validation() {
        // Valid request with questions
        let valid_request = AskUserRequest {
            questions: vec![UserQuestion {
                id: "q1".to_string(),
                question: "What is your name?".to_string(),
                response_type: QuestionType::Text,
                default: None,
                options: None,
                description: None,
                validation: None,
            }],
        };
        assert!(valid_request.validate().is_ok());

        // Empty questions is valid - means no clarifications needed
        let empty_request = AskUserRequest { questions: vec![] };
        assert!(empty_request.validate().is_ok());

        // Duplicate question IDs
        let duplicate_request = AskUserRequest {
            questions: vec![
                UserQuestion {
                    id: "q1".to_string(),
                    question: "First question".to_string(),
                    response_type: QuestionType::Text,
                    default: None,
                    options: None,
                    description: None,
                    validation: None,
                },
                UserQuestion {
                    id: "q1".to_string(),
                    question: "Second question".to_string(),
                    response_type: QuestionType::Text,
                    default: None,
                    options: None,
                    description: None,
                    validation: None,
                },
            ],
        };
        assert!(duplicate_request.validate().is_err());
    }
}
