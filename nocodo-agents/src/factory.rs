use crate::codebase_analysis::CodebaseAnalysisAgent;
use crate::Agent;
use nocodo_llm_sdk::client::LlmClient;
use std::sync::Arc;

/// Enum representing the available agent types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentType {
    /// Agent for analyzing codebase structure and architecture
    CodebaseAnalysis,
}

/// Factory function to create an agent of the specified type
///
/// # Arguments
///
/// * `agent_type` - The type of agent to create
/// * `client` - The LLM client to use for the agent
///
/// # Returns
///
/// A boxed trait object implementing the Agent trait
///
/// # Example
///
/// ```no_run
/// use nocodo_agents::factory::{AgentType, create_agent};
/// use nocodo_llm_sdk::claude::ClaudeClient;
/// use std::sync::Arc;
///
/// # async fn example() -> anyhow::Result<()> {
/// let client = Arc::new(ClaudeClient::new("api-key")?);
/// let agent = create_agent(AgentType::CodebaseAnalysis, client);
///
/// println!("Agent objective: {}", agent.objective());
/// let result = agent.execute("Analyze this codebase").await?;
/// # Ok(())
/// # }
/// ```
pub fn create_agent(agent_type: AgentType, client: Arc<dyn LlmClient>) -> Box<dyn Agent> {
    match agent_type {
        AgentType::CodebaseAnalysis => Box::new(CodebaseAnalysisAgent::new(client)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nocodo_llm_sdk::client::LlmClient;
    use nocodo_llm_sdk::error::LlmError;
    use nocodo_llm_sdk::types::{CompletionRequest, CompletionResponse, ContentBlock, Role, Usage};

    struct MockLlmClient;

    #[async_trait::async_trait]
    impl LlmClient for MockLlmClient {
        async fn complete(
            &self,
            _request: CompletionRequest,
        ) -> Result<CompletionResponse, LlmError> {
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
            })
        }

        fn provider_name(&self) -> &str {
            "mock"
        }

        fn model_name(&self) -> &str {
            "mock-model"
        }
    }

    #[test]
    fn test_create_codebase_analysis_agent() {
        let client: Arc<dyn LlmClient> = Arc::new(MockLlmClient);
        let agent = create_agent(AgentType::CodebaseAnalysis, client);

        assert_eq!(
            agent.objective(),
            "Analyze codebase structure and identify architectural patterns"
        );
    }

    #[test]
    fn test_agent_type_values() {
        let agent_types = vec![AgentType::CodebaseAnalysis];

        // Ensure we can iterate over agent types
        for agent_type in agent_types {
            let client: Arc<dyn LlmClient> = Arc::new(MockLlmClient);
            let _agent = create_agent(agent_type, client);
        }
    }
}
