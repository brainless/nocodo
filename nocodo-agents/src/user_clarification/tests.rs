use super::*;
use crate::database::Database;
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::error::LlmError;
use nocodo_llm_sdk::types::{CompletionRequest, CompletionResponse, ContentBlock, Role, Usage};
use std::path::PathBuf;
use std::sync::Arc;

struct MockLlmClient {
    response_content: String,
}

#[async_trait::async_trait]
impl LlmClient for MockLlmClient {
    async fn complete(&self, _request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        Ok(CompletionResponse {
            content: vec![ContentBlock::Text {
                text: self.response_content.clone(),
            }],
            role: Role::Assistant,
            usage: Usage {
                input_tokens: 10,
                output_tokens: 20,
            },
            stop_reason: Some("end_turn".to_string()),
            tool_calls: None,
        })
    }

    fn provider_name(&self) -> &str {
        "mock"
    }

    fn model_name(&self) -> &str {
        "mock-model"
    }
}

fn setup_test_agent(
    response_content: &str,
) -> anyhow::Result<(UserClarificationAgent, Arc<Database>)> {
    let client: Arc<dyn LlmClient> = Arc::new(MockLlmClient {
        response_content: response_content.to_string(),
    });

    let database = Arc::new(Database::new(&PathBuf::from(":memory:"))?);
    let agent = UserClarificationAgent::new(client, database.clone());

    Ok((agent, database))
}

#[tokio::test]
async fn test_user_clarification_agent_returns_questions_when_needed() {
    // Mock LLM response with clarifying questions
    let mock_response = r#"{
        "questions": [
            {
                "id": "q1",
                "question": "What is the primary purpose of the website?",
                "type": "text",
                "description": "e.g., portfolio, e-commerce, blog"
            },
            {
                "id": "q2",
                "question": "Do you have any technology preferences?",
                "type": "text"
            }
        ]
    }"#;

    let (agent, database) = setup_test_agent(mock_response).unwrap();

    let session_id = database
        .create_session(
            "user-clarification",
            "test",
            "test",
            None,
            "Build me a website",
            None,
        )
        .unwrap();

    let result = agent
        .execute("Build me a website", session_id)
        .await
        .unwrap();

    // Parse the result to verify it's valid AskUserRequest
    let parsed: AskUserRequest = serde_json::from_str(&result).unwrap();

    assert_eq!(parsed.questions.len(), 2);
    assert_eq!(parsed.questions[0].id, "q1");
    assert_eq!(
        parsed.questions[0].question,
        "What is the primary purpose of the website?"
    );
    assert!(matches!(
        parsed.questions[0].response_type,
        QuestionType::Text
    ));
}
