use std::sync::Arc;

use llm_sdk::{
    client::LlmClient,
    types::{CompletionRequest, Message, Role},
};

use super::prompts::PO_USER_SESSION_SYSTEM_PROMPT;
use crate::{
    config::AgentConfig,
    error::AgentError,
    storage::{AgentStorage, AgentType, CommentStorage, TaskStorage},
    task_policy,
};

// ---------------------------------------------------------------------------
// Public result type
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum PoSessionResult {
    Text(String),
    Silent,
}

// ---------------------------------------------------------------------------
// Agent
// ---------------------------------------------------------------------------

pub struct ProductOwnerAgent {
    llm_client: Arc<dyn LlmClient>,
    _storage: Arc<dyn AgentStorage>,
    task_storage: Arc<dyn TaskStorage>,
    _comment_storage: Arc<dyn CommentStorage>,
    model: String,
}

impl ProductOwnerAgent {
    pub fn new(
        _storage: Arc<dyn AgentStorage>,
        task_storage: Arc<dyn TaskStorage>,
        _comment_storage: Arc<dyn CommentStorage>,
        config: AgentConfig,
    ) -> Result<Self, AgentError> {
        let llm_client = crate::make_llm_client(&config)?;
        Ok(Self {
            llm_client,
            _storage,
            task_storage,
            _comment_storage,
            model: config.model,
        })
    }

    pub async fn respond_in_session(
        &self,
        messages: Vec<(String, String)>,
    ) -> Result<PoSessionResult, AgentError> {
        let llm_messages: Vec<Message> = messages
            .iter()
            .map(|(role, content)| {
                let r = match role.as_str() {
                    "assistant" => Role::Assistant,
                    "tool" => Role::Tool,
                    _ => Role::User,
                };
                Message {
                    role: r,
                    content: vec![llm_sdk::types::ContentBlock::Text {
                        text: content.clone(),
                    }],
                    tool_call_id: None,
                    tool_name: None,
                }
            })
            .collect();

        let request = CompletionRequest {
            messages: llm_messages,
            max_tokens: 1024,
            model: self.model.clone(),
            system: Some(PO_USER_SESSION_SYSTEM_PROMPT.to_string()),
            temperature: Some(0.3),
            top_p: None,
            stop_sequences: None,
            tools: None,
            tool_choice: None,
            response_format: None,
        };

        log::info!("[PO:user_session] Calling LLM with model={}", self.model);
        let response = self.llm_client.complete(request).await?;

        let text = response
            .content
            .iter()
            .filter_map(|b| match b {
                llm_sdk::types::ContentBlock::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");

        if text.trim().is_empty() {
            Ok(PoSessionResult::Silent)
        } else {
            Ok(PoSessionResult::Text(text))
        }
    }

    pub async fn validate_tasks(&self, task_ids: Vec<i64>) -> Result<(), AgentError> {
        for task_id in task_ids {
            let task = self
                .task_storage
                .get_task(task_id)
                .await?
                .ok_or_else(|| AgentError::Other(format!("Task {} not found", task_id)))?;

            let agent_type = AgentType::from_str(&task.assigned_to_agent);
            let next_state = task_policy::initial_state_for(&agent_type);
            self.task_storage
                .update_task_status(task_id, next_state)
                .await?;
        }
        Ok(())
    }
}
