use crate::database::Database;
use rusqlite::params;

/// Requirements gathering database operations
impl Database {
    /// Store questions in the project_requirements_qna table
    pub fn store_questions(
        &self,
        session_id: i64,
        tool_call_id: Option<i64>,
        questions: &[shared_types::user_interaction::UserQuestion],
    ) -> anyhow::Result<()> {
        let conn = self.connection.lock().unwrap();
        let now = chrono::Utc::now().timestamp();

        for question in questions {
            conn.execute(
                "INSERT INTO project_requirements_qna (session_id, tool_call_id, question_id, question, description, response_type, created_at)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    session_id,
                    tool_call_id,
                    &question.id,
                    &question.question,
                    &question.description,
                    format!("{:?}", question.response_type).to_lowercase(),
                    now
                ],
            )?;
        }

        Ok(())
    }

    /// Get pending (unanswered) questions from the database
    pub fn get_pending_questions(
        &self,
        session_id: i64,
    ) -> anyhow::Result<Vec<shared_types::user_interaction::UserQuestion>> {
        let conn = self.connection.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT question_id, question, description, response_type
                FROM project_requirements_qna
                WHERE session_id = ?1 AND answer IS NULL
                ORDER BY created_at ASC",
        )?;

        let questions = stmt.query_map([session_id], |row| {
            let response_type_str: String = row.get(3)?;
            let response_type = match response_type_str.as_str() {
                "text" => shared_types::user_interaction::QuestionType::Text,
                "password" => shared_types::user_interaction::QuestionType::Password,
                "file_path" => shared_types::user_interaction::QuestionType::FilePath,
                "email" => shared_types::user_interaction::QuestionType::Email,
                "url" => shared_types::user_interaction::QuestionType::Url,
                _ => shared_types::user_interaction::QuestionType::Text,
            };

            Ok(shared_types::user_interaction::UserQuestion {
                id: row.get(0)?,
                question: row.get(1)?,
                description: row.get(2)?,
                response_type,
                default: None,
                options: None,
            })
        })?;

        let mut result = Vec::new();
        for question in questions {
            result.push(question?);
        }

        Ok(result)
    }

    /// Store answers for questions in the database
    pub fn store_answers(
        &self,
        session_id: i64,
        answers: &std::collections::HashMap<String, String>,
    ) -> anyhow::Result<()> {
        let conn = self.connection.lock().unwrap();
        let now = chrono::Utc::now().timestamp();

        for (question_id, answer) in answers {
            conn.execute(
                "UPDATE project_requirements_qna
                    SET answer = ?1, answered_at = ?2
                    WHERE session_id = ?3 AND question_id = ?4",
                params![answer, now, session_id, question_id],
            )?;
        }

        Ok(())
    }
}
