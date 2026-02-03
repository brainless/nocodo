use crate::storage::StorageError;
use async_trait::async_trait;
use shared_types::user_interaction::UserQuestion;

#[async_trait]
pub trait RequirementsStorage: Send + Sync {
    async fn store_questions(
        &self,
        session_id: &str,
        tool_call_id: Option<&str>,
        questions: &[UserQuestion],
    ) -> Result<(), StorageError>;

    async fn get_pending_questions(
        &self,
        session_id: &str,
    ) -> Result<Vec<UserQuestion>, StorageError>;

    async fn store_answers(
        &self,
        session_id: &str,
        answers: &std::collections::HashMap<String, String>,
    ) -> Result<(), StorageError>;
}
