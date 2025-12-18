use crate::{Agent, AgentTool};
use async_trait::async_trait;
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::types::{CompletionRequest, ContentBlock, Message, Role};
use std::sync::Arc;

#[cfg(test)]
mod tests;

/// Agent specialized in analyzing codebase structure and identifying architectural patterns
pub struct CodebaseAnalysisAgent {
    client: Arc<dyn LlmClient>,
}

impl CodebaseAnalysisAgent {
    /// Create a new CodebaseAnalysisAgent with the given LLM client
    pub fn new(client: Arc<dyn LlmClient>) -> Self {
        Self { client }
    }
}

#[async_trait]
impl Agent for CodebaseAnalysisAgent {
    fn objective(&self) -> &str {
        "Analyze codebase structure and identify architectural patterns"
    }

    fn system_prompt(&self) -> &str {
        "You are a codebase analysis expert. Your role is to examine code repositories, \
         understand their structure, identify architectural patterns, and provide clear insights \
         about the codebase organization. You should analyze file structures, dependencies, \
         design patterns, and architectural decisions."
    }

    fn tools(&self) -> Vec<AgentTool> {
        vec![AgentTool::ListFiles, AgentTool::ReadFile, AgentTool::Grep]
    }

    async fn execute(&self, user_prompt: &str) -> anyhow::Result<String> {
        // Build the completion request
        let request = CompletionRequest {
            messages: vec![
                Message {
                    role: Role::System,
                    content: vec![ContentBlock::Text {
                        text: self.system_prompt().to_string(),
                    }],
                },
                Message {
                    role: Role::User,
                    content: vec![ContentBlock::Text {
                        text: user_prompt.to_string(),
                    }],
                },
            ],
            max_tokens: 4000,
            model: self.client.model_name().to_string(),
            system: Some(self.system_prompt().to_string()),
            temperature: Some(0.7),
            top_p: None,
            stop_sequences: None,
        };

        // Call the LLM
        let response = self.client.complete(request).await?;

        // Extract text from response content
        let text = extract_text_from_content(&response.content);

        // TODO: Implement tool execution flow
        // For now, just return the LLM response
        Ok(text)
    }
}

/// Helper function to extract text from content blocks
fn extract_text_from_content(content: &[ContentBlock]) -> String {
    content
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}
