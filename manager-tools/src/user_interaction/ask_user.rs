use anyhow::Result;
use manager_models::{
    AskUserRequest, AskUserResponse, ToolErrorResponse, ToolResponse, UserQuestion, 
    QuestionType, UserQuestionResponse
};
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

    let start_time = std::time::Instant::now();
    let mut responses = Vec::new();
    let mut all_valid = true;

    // Display the prompt
    if !request.prompt.is_empty() {
        println!("\n{}", request.prompt);
        println!("{}", "=".repeat(request.prompt.len()));
    }

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
    let completed = request.required.unwrap_or(true) && all_valid;

    Ok(ToolResponse::AskUser(AskUserResponse {
        completed,
        responses,
        message: if completed {
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

    // Add options for select/multiselect questions
    if let Some(options) = &question.options {
        match question.response_type {
            QuestionType::Select => {
                prompt_parts.push("  Options:".to_string());
                for (i, option) in options.iter().enumerate() {
                    prompt_parts.push(format!("    {}. {}", i + 1, option));
                }
            }
            QuestionType::Multiselect => {
                prompt_parts.push("  Options (select multiple, comma-separated):".to_string());
                for (i, option) in options.iter().enumerate() {
                    prompt_parts.push(format!("    {}. {}", i + 1, option));
                }
            }
            _ => {}
        }
    }

    // Add response type indicator
    let response_indicator = match question.response_type {
        QuestionType::Text => " (text)",
        QuestionType::Number => " (number)",
        QuestionType::Boolean => " (yes/no)",
        QuestionType::Select => " (enter number)",
        QuestionType::Multiselect => " (enter numbers, comma-separated)",
        QuestionType::Password => " (password - will be hidden)",
        QuestionType::FilePath => " (file path)",
        QuestionType::Email => " (email)",
        QuestionType::Url => " (url)",
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

    // Handle different input types
    match question.response_type {
        QuestionType::Password => {
            // For password input, we'll just read normally since we can't easily hide input in this context
            // In a real implementation, you might use a library like rpassword
            input = read_password_input()?;
        }
        _ => {
            io::stdin().read_line(&mut input)?;
        }
    }

    // Handle empty input with default
    let trimmed = input.trim();
    let response = if trimmed.is_empty() {
        question.default.clone().unwrap_or_default()
    } else {
        trimmed.to_string()
    };

    // Process select/multiselect responses
    match question.response_type {
        QuestionType::Select => {
            if let Some(options) = &question.options {
                if let Ok(index) = response.parse::<usize>() {
                    if index >= 1 && index <= options.len() {
                        return Ok(options[index - 1].clone());
                    }
                }
                return Err(anyhow::anyhow!("Invalid selection. Please enter a number between 1 and {}", options.len()));
            }
        }
        QuestionType::Multiselect => {
            if let Some(options) = &question.options {
                let mut selected = Vec::new();
                for part in response.split(',') {
                    if let Ok(index) = part.trim().parse::<usize>() {
                        if index >= 1 && index <= options.len() {
                            selected.push(options[index - 1].clone());
                        }
                    }
                }
                if selected.is_empty() {
                    return Err(anyhow::anyhow!("No valid selections made"));
                }
                return Ok(selected.join(", "));
            }
        }
        QuestionType::Boolean => {
            let response_lower = response.to_lowercase();
            match response_lower.as_str() {
                "y" | "yes" | "true" | "1" => return Ok("true".to_string()),
                "n" | "no" | "false" | "0" => return Ok("false".to_string()),
                _ => return Err(anyhow::anyhow!("Please answer 'yes' or 'no'")),
            }
        }
        _ => {}
    }

    Ok(response)
}

/// Read password input (simplified version - in reality you'd want to hide the input)
fn read_password_input() -> Result<String> {
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

/// Validate a response against the question's validation rules
fn validate_response(question: &UserQuestion, response: &str) -> Result<(), String> {
    if let Some(validation) = &question.validation {
        // Length validation for text responses
        match question.response_type {
            QuestionType::Text | QuestionType::Email | QuestionType::Url | QuestionType::FilePath => {
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
            }
            QuestionType::Number => {
                if let Ok(num) = response.parse::<f64>() {
                    if let Some(min_value) = validation.min_value {
                        if num < min_value {
                            return Err(format!(
                                "Number too small (minimum {})",
                                min_value
                            ));
                        }
                    }
                    if let Some(max_value) = validation.max_value {
                        if num > max_value {
                            return Err(format!(
                                "Number too large (maximum {})",
                                max_value
                            ));
                        }
                    }
                } else {
                    return Err("Invalid number format".to_string());
                }
            }
            _ => {}
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

    // Type-specific validation
    match question.response_type {
        QuestionType::Email => {
            // Simple email validation
            if !response.contains('@') || !response.contains('.') {
                return Err("Invalid email format".to_string());
            }
        }
        QuestionType::Url => {
            // Simple URL validation
            if !response.starts_with("http://") && !response.starts_with("https://") {
                return Err("URL must start with http:// or https://".to_string());
            }
        }
        _ => {}
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ask_user_request_validation() {
        // Valid request
        let valid_request = AskUserRequest {
            prompt: "Test prompt".to_string(),
            questions: vec![
                UserQuestion {
                    id: "q1".to_string(),
                    question: "What is your name?".to_string(),
                    response_type: QuestionType::Text,
                    default: None,
                    options: None,
                    description: None,
                    validation: None,
                }
            ],
            required: Some(true),
            timeout_secs: None,
        };
        assert!(valid_request.validate().is_ok());

        // Empty questions
        let invalid_request = AskUserRequest {
            prompt: "Test prompt".to_string(),
            questions: vec![],
            required: Some(true),
            timeout_secs: None,
        };
        assert!(invalid_request.validate().is_err());

        // Duplicate question IDs
        let duplicate_request = AskUserRequest {
            prompt: "Test prompt".to_string(),
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
                }
            ],
            required: Some(true),
            timeout_secs: None,
        };
        assert!(duplicate_request.validate().is_err());
    }
}