use crate::storage::{AgentStorage, StorageError};
use crate::types::{Message, Session, ToolCall};
use shared_types::user_interaction::UserQuestion;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct InMemoryStorage {
    sessions: Arc<Mutex<HashMap<i64, Session>>>,
    messages: Arc<Mutex<HashMap<i64, Vec<Message>>>>,
    tool_calls: Arc<Mutex<HashMap<i64, Vec<ToolCall>>>>,
    questions: Arc<Mutex<HashMap<i64, Vec<UserQuestion>>>>,
    answers: Arc<Mutex<HashMap<i64, HashMap<String, String>>>>,
    counter: Arc<Mutex<i64>>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            messages: Arc::new(Mutex::new(HashMap::new())),
            tool_calls: Arc::new(Mutex::new(HashMap::new())),
            questions: Arc::new(Mutex::new(HashMap::new())),
            answers: Arc::new(Mutex::new(HashMap::new())),
            counter: Arc::new(Mutex::new(0)),
        }
    }

    fn next_id(&self) -> i64 {
        let mut counter = self.counter.lock().unwrap();
        *counter += 1;
        *counter
    }
}

#[async_trait::async_trait]
impl AgentStorage for InMemoryStorage {
    async fn create_session(&self, session: Session) -> Result<i64, StorageError> {
        let session_id = self.next_id();
        let mut session_with_id = session;
        session_with_id.id = Some(session_id);
        self.sessions
            .lock()
            .unwrap()
            .insert(session_id, session_with_id);
        Ok(session_id)
    }

    async fn get_session(&self, session_id: i64) -> Result<Option<Session>, StorageError> {
        Ok(self.sessions.lock().unwrap().get(&session_id).cloned())
    }

    async fn update_session(&self, session: Session) -> Result<(), StorageError> {
        let session_id = session.id.unwrap_or_else(|| self.next_id());
        self.sessions.lock().unwrap().insert(session_id, session);
        Ok(())
    }

    async fn create_message(&self, message: Message) -> Result<i64, StorageError> {
        let message_id = self.next_id();
        let session_id = message.session_id;
        let mut message_with_id = message;
        message_with_id.id = Some(message_id);
        let mut messages = self.messages.lock().unwrap();
        messages
            .entry(session_id)
            .or_insert_with(Vec::new)
            .push(message_with_id);
        Ok(message_id)
    }

    async fn get_messages(&self, session_id: i64) -> Result<Vec<Message>, StorageError> {
        Ok(self
            .messages
            .lock()
            .unwrap()
            .get(&session_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn create_tool_call(&self, tool_call: ToolCall) -> Result<i64, StorageError> {
        let tool_call_id = self.next_id();
        let session_id = tool_call.session_id;
        let mut tool_call_with_id = tool_call;
        tool_call_with_id.id = Some(tool_call_id);
        let mut tool_calls = self.tool_calls.lock().unwrap();
        tool_calls
            .entry(session_id)
            .or_insert_with(Vec::new)
            .push(tool_call_with_id);
        Ok(tool_call_id)
    }

    async fn update_tool_call(&self, tool_call: ToolCall) -> Result<(), StorageError> {
        let tool_call_id = tool_call.id.unwrap_or_else(|| self.next_id());
        let mut tool_calls = self.tool_calls.lock().unwrap();
        if let Some(calls) = tool_calls.get_mut(&tool_call.session_id) {
            if let Some(pos) = calls.iter().position(|c| c.id == Some(tool_call_id)) {
                calls[pos] = tool_call;
            }
        }
        Ok(())
    }

    async fn get_tool_calls(&self, session_id: i64) -> Result<Vec<ToolCall>, StorageError> {
        Ok(self
            .tool_calls
            .lock()
            .unwrap()
            .get(&session_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn get_pending_tool_calls(
        &self,
        session_id: i64,
    ) -> Result<Vec<ToolCall>, StorageError> {
        Ok(self
            .tool_calls
            .lock()
            .unwrap()
            .get(&session_id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|c| matches!(c.status, crate::types::ToolCallStatus::Pending))
            .collect())
    }
}

#[async_trait::async_trait]
impl crate::requirements_gathering::storage::RequirementsStorage for InMemoryStorage {
    async fn store_questions(
        &self,
        session_id: i64,
        _tool_call_id: Option<i64>,
        questions: &[UserQuestion],
    ) -> Result<(), StorageError> {
        let mut question_store = self.questions.lock().unwrap();
        question_store
            .entry(session_id)
            .or_insert_with(Vec::new)
            .extend(questions.iter().cloned());
        Ok(())
    }

    async fn get_pending_questions(
        &self,
        session_id: i64,
    ) -> Result<Vec<UserQuestion>, StorageError> {
        let questions = self.questions.lock().unwrap();
        let answers = self.answers.lock().unwrap();

        let session_answers = answers.get(&session_id);
        let session_questions = questions.get(&session_id).cloned().unwrap_or_default();

        // Filter out questions that have been answered
        Ok(session_questions
            .into_iter()
            .filter(|q| {
                session_answers
                    .map(|ans| !ans.contains_key(&q.id))
                    .unwrap_or(true)
            })
            .collect())
    }

    async fn store_answers(
        &self,
        session_id: i64,
        answers: &std::collections::HashMap<String, String>,
    ) -> Result<(), StorageError> {
        let mut answer_store = self.answers.lock().unwrap();
        answer_store
            .entry(session_id)
            .or_insert_with(HashMap::new)
            .extend(answers.clone());
        Ok(())
    }
}
