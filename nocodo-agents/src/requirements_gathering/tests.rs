use super::*;
use crate::database::Database;
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::error::LlmError;
use nocodo_llm_sdk::tools::ToolCall;
use nocodo_llm_sdk::types::{CompletionRequest, CompletionResponse, ContentBlock, Role, Usage};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

struct MockLlmClient {
    response_content: String,
    include_tool_call: bool,
    call_count: Arc<AtomicUsize>,
}

#[async_trait::async_trait]
impl LlmClient for MockLlmClient {
    async fn complete(&self, _request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let count = self.call_count.fetch_add(1, Ordering::SeqCst);

        let tool_calls = if self.include_tool_call && count == 0 {
            Some(vec![ToolCall::new(
                "call_123".to_string(),
                "ask_user".to_string(),
                serde_json::json!({
                    "questions": [
                        {
                            "id": "q1",
                            "question": "What is the primary purpose of the website?",
                            "type": "text",
                            "description": "e.g., portfolio, e-commerce, blog"
                        }
                    ]
                }),
            )])
        } else {
            None
        };

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
            tool_calls,
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
    include_tool_call: bool,
) -> anyhow::Result<(UserClarificationAgent, Arc<Database>)> {
    let call_count = Arc::new(AtomicUsize::new(0));
    let client: Arc<dyn LlmClient> = Arc::new(MockLlmClient {
        response_content: response_content.to_string(),
        include_tool_call,
        call_count,
    });

    let database = Arc::new(Database::new(&PathBuf::from(":memory:"))?);
    let tool_executor = Arc::new(nocodo_tools::ToolExecutor::new(PathBuf::from(".")));
    let agent = UserClarificationAgent::new(client, database.clone(), tool_executor);

    Ok((agent, database))
}

#[tokio::test]
async fn test_user_clarification_agent_uses_ask_user_tool() {
    let mock_response = "I need to gather some information about your requirements.";

    let (agent, database) = setup_test_agent(mock_response, true).unwrap();

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

    assert!(result.contains("requirements") || result.contains("information"));

    let messages = database.get_messages(session_id).unwrap();
    let tool_messages: Vec<_> = messages.iter().filter(|m| m.role == "tool").collect();

    assert!(
        !tool_messages.is_empty(),
        "Expected at least one tool call message"
    );
}
