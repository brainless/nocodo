use crate::storage::StorageError;
use async_trait::async_trait;
use shared_types::user_interaction::UserQuestion;

#[async_trait]
pub trait RequirementsStorage: Send + Sync {
    async fn store_questions(
        &self,
        session_id: i64,
        tool_call_id: Option<i64>,
        questions: &[UserQuestion],
    ) -> Result<(), StorageError>;

    async fn get_pending_questions(
        &self,
        session_id: i64,
    ) -> Result<Vec<UserQuestion>, StorageError>;

    async fn store_answers(
        &self,
        session_id: i64,
        answers: &std::collections::HashMap<String, String>,
    ) -> Result<(), StorageError>;
}
