#[cfg(test)]
mod tests {
    use crate::types::{AskUserRequest, QuestionType, UserQuestion};

    #[tokio::test]
    async fn test_ask_user_validation() {
        // Test valid request with Text questions only
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
                    id: "details".to_string(),
                    question: "What are you looking for?".to_string(),
                    response_type: QuestionType::Text,
                    default: Some("general help".to_string()),
                    options: None,
                    description: Some("Describe what you need help with".to_string()),
                    validation: None,
                },
            ],
            required: Some(true),
            timeout_secs: Some(300),
        };

        assert!(valid_request.validate().is_ok());
    }

    #[tokio::test]
    async fn test_ask_user_empty_questions_is_valid() {
        // Empty questions is valid - means no clarifications needed
        let empty_questions = AskUserRequest {
            prompt: "Test".to_string(),
            questions: vec![],
            required: Some(true),
            timeout_secs: None,
        };
        assert!(empty_questions.validate().is_ok());
    }

    #[tokio::test]
    async fn test_ask_user_invalid_requests() {
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
                },
            ],
            required: Some(true),
            timeout_secs: None,
        };
        assert!(duplicate_ids.validate().is_err());
    }
}
