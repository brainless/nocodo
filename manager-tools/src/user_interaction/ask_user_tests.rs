#[cfg(test)]
mod tests {
    use manager_models::{AskUserRequest, UserQuestion, QuestionType};

    #[tokio::test]
    async fn test_ask_user_validation() {
        // Test valid request
        let valid_request = AskUserRequest {
            prompt: "Please answer the following questions:".to_string(),
            questions: vec![
                UserQuestion {
                    id: "name".to_string(),
                    question: "What is your name?".to_string(),
                    response_type: QuestionType::Text,
                    default: None,
                    options: None,
                    description: None,
                    validation: None,
                },
                UserQuestion {
                    id: "age".to_string(),
                    question: "What is your age?".to_string(),
                    response_type: QuestionType::Number,
                    default: Some("25".to_string()),
                    options: None,
                    description: None,
                    validation: None,
                },
                UserQuestion {
                    id: "experience".to_string(),
                    question: "What is your experience level?".to_string(),
                    response_type: QuestionType::Select,
                    default: None,
                    options: Some(vec![
                        "Beginner".to_string(),
                        "Intermediate".to_string(),
                        "Advanced".to_string(),
                    ]),
                    description: Some("Choose your experience level".to_string()),
                    validation: None,
                }
            ],
            required: Some(true),
            timeout_secs: Some(300),
        };

        assert!(valid_request.validate().is_ok());
    }

    #[tokio::test]
    async fn test_ask_user_invalid_requests() {
        // Empty questions
        let empty_questions = AskUserRequest {
            prompt: "Test".to_string(),
            questions: vec![],
            required: Some(true),
            timeout_secs: None,
        };
        assert!(empty_questions.validate().is_err());

        // Select question without options
        let select_no_options = AskUserRequest {
            prompt: "Test".to_string(),
            questions: vec![
                UserQuestion {
                    id: "choice".to_string(),
                    question: "Choose an option".to_string(),
                    response_type: QuestionType::Select,
                    default: None,
                    options: None,
                    description: None,
                    validation: None,
                }
            ],
            required: Some(true),
            timeout_secs: None,
        };
        assert!(select_no_options.validate().is_err());

        // Duplicate question IDs
        let duplicate_ids = AskUserRequest {
            prompt: "Test".to_string(),
            questions: vec![
                UserQuestion {
                    id: "duplicate".to_string(),
                    question: "First question".to_string(),
                    response_type: QuestionType::Text,
                    default: None,
                    options: None,
                    description: None,
                    validation: None,
                },
                UserQuestion {
                    id: "duplicate".to_string(),
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
        assert!(duplicate_ids.validate().is_err());
    }
}