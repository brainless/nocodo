use crate::storage::{AgentStorage, StorageError};
use crate::types::{Message, Session, ToolCall};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct InMemoryStorage {
    sessions: Arc<Mutex<HashMap<String, Session>>>,
    messages: Arc<Mutex<HashMap<String, Vec<Message>>>>,
    tool_calls: Arc<Mutex<HashMap<String, Vec<ToolCall>>>>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            messages: Arc::new(Mutex::new(HashMap::new())),
            tool_calls: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait::async_trait]
impl AgentStorage for InMemoryStorage {
    async fn create_session(&self, session: Session) -> Result<String, StorageError> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let mut session_with_id = session;
        session_with_id.id = Some(session_id.clone());
        self.sessions
            .lock()
            .unwrap()
            .insert(session_id.clone(), session_with_id);
        Ok(session_id)
    }

    async fn get_session(&self, session_id: &str) -> Result<Option<Session>, StorageError> {
        Ok(self.sessions.lock().unwrap().get(session_id).cloned())
    }

    async fn update_session(&self, session: Session) -> Result<(), StorageError> {
        let session_id = session
            .id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        self.sessions.lock().unwrap().insert(session_id, session);
        Ok(())
    }

    async fn create_message(&self, message: Message) -> Result<String, StorageError> {
        let message_id = uuid::Uuid::new_v4().to_string();
        let mut message_with_id = message;
        message_with_id.id = Some(message_id.clone());
        let mut messages = self.messages.lock().unwrap();
        messages
            .entry(message.session_id.clone())
            .or_insert_with(Vec::new)
            .push(message_with_id);
        Ok(message_id)
    }

    async fn get_messages(&self, session_id: &str) -> Result<Vec<Message>, StorageError> {
        Ok(self
            .messages
            .lock()
            .unwrap()
            .get(session_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn create_tool_call(&self, tool_call: ToolCall) -> Result<String, StorageError> {
        let tool_call_id = uuid::Uuid::new_v4().to_string();
        let mut tool_call_with_id = tool_call;
        tool_call_with_id.id = Some(tool_call_id.clone());
        let mut tool_calls = self.tool_calls.lock().unwrap();
        tool_calls
            .entry(tool_call.session_id.clone())
            .or_insert_with(Vec::new)
            .push(tool_call_with_id);
        Ok(tool_call_id)
    }

    async fn update_tool_call(&self, tool_call: ToolCall) -> Result<(), StorageError> {
        let tool_call_id = tool_call
            .id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let mut tool_calls = self.tool_calls.lock().unwrap();
        if let Some(calls) = tool_calls.get_mut(&tool_call.session_id) {
            if let Some(pos) = calls
                .iter()
                .position(|c| c.id.as_ref() == Some(&tool_call_id))
            {
                calls[pos] = tool_call;
            }
        }
        Ok(())
    }

    async fn get_tool_calls(&self, session_id: &str) -> Result<Vec<ToolCall>, StorageError> {
        Ok(self
            .tool_calls
            .lock()
            .unwrap()
            .get(session_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn get_pending_tool_calls(
        &self,
        session_id: &str,
    ) -> Result<Vec<ToolCall>, StorageError> {
        Ok(self
            .tool_calls
            .lock()
            .unwrap()
            .get(session_id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|c| matches!(c.status, crate::types::ToolCallStatus::Pending))
            .collect())
    }
}
