use super::*;
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::error::LlmError;
use nocodo_llm_sdk::types::{CompletionRequest, CompletionResponse, ContentBlock, Role, Usage};
use std::sync::Arc;

/// Mock LLM client for testing
struct MockLlmClient;

#[async_trait::async_trait]
impl LlmClient for MockLlmClient {
    async fn complete(&self, _request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        Ok(CompletionResponse {
            content: vec![ContentBlock::Text {
                text: "Mock response".to_string(),
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

fn create_test_agent() -> CodebaseAnalysisAgent {
    use crate::database::Database;
    use manager_tools::ToolExecutor;
    use std::path::PathBuf;

    let client: Arc<dyn LlmClient> = Arc::new(MockLlmClient);
    let database = Arc::new(Database::new(&PathBuf::from(":memory:")).unwrap());
    let tool_executor = Arc::new(
        ToolExecutor::new(PathBuf::from(".")).with_max_file_size(10 * 1024 * 1024), // 10MB
    );

    CodebaseAnalysisAgent::new(client, database, tool_executor)
}

#[test]
fn test_codebase_analysis_agent_objective() {
    let agent = create_test_agent();
    assert_eq!(
        agent.objective(),
        "Analyze codebase structure and identify architectural patterns"
    );
}

#[test]
fn test_codebase_analysis_agent_has_required_tools() {
    let agent = create_test_agent();
    let tools = agent.tools();

    assert!(tools.contains(&AgentTool::ListFiles));
    assert!(tools.contains(&AgentTool::ReadFile));
    assert!(tools.contains(&AgentTool::Grep));
}

#[test]
fn test_codebase_analysis_agent_system_prompt_not_empty() {
    let agent = create_test_agent();
    assert!(!agent.system_prompt().is_empty());
}

#[test]
fn test_codebase_analysis_agent_no_preconditions() {
    let agent = create_test_agent();
    assert!(agent.pre_conditions().is_none());
}
