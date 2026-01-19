use crate::database::Database;
use rusqlite::params;

/// Settings management database operations
impl Database {
    /// Store settings in the project_settings table
    pub fn store_settings(
        &self,
        session_id: i64,
        tool_call_id: Option<i64>,
        settings: &[shared_types::user_interaction::UserQuestion],
    ) -> anyhow::Result<()> {
        let conn = self.connection.lock().unwrap();
        let now = chrono::Utc::now().timestamp();

        for setting in settings {
            conn.execute(
                "INSERT INTO project_settings (session_id, tool_call_id, setting_key, setting_name, description, setting_type, created_at)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    session_id,
                    tool_call_id,
                    &setting.id,
                    &setting.question,
                    &setting.description,
                    format!("{:?}", setting.response_type).to_lowercase(),
                    now
                ],
            )?;
        }

        Ok(())
    }

    /// Get pending (unanswered) settings from the database
    pub fn get_pending_settings(
        &self,
        session_id: i64,
    ) -> anyhow::Result<Vec<shared_types::user_interaction::UserQuestion>> {
        let conn = self.connection.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT setting_key, setting_name, description, setting_type
                FROM project_settings
                WHERE session_id = ?1 AND setting_value IS NULL
                ORDER BY created_at ASC",
        )?;

        let settings = stmt.query_map([session_id], |row| {
            let setting_type_str: String = row.get(3)?;
            let response_type = match setting_type_str.as_str() {
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
        for setting in settings {
            result.push(setting?);
        }

        Ok(result)
    }

    /// Store setting values for settings in the database
    pub fn store_setting_values(
        &self,
        session_id: i64,
        setting_values: &std::collections::HashMap<String, String>,
    ) -> anyhow::Result<()> {
        let conn = self.connection.lock().unwrap();
        let now = chrono::Utc::now().timestamp();

        for (setting_key, value) in setting_values {
            conn.execute(
                "UPDATE project_settings
                    SET setting_value = ?1, updated_at = ?2
                    WHERE session_id = ?3 AND setting_key = ?4",
                params![value, now, session_id, setting_key],
            )?;
        }

        Ok(())
    }

    /// Get all settings for a session (both pending and completed)
    pub fn get_session_settings(
        &self,
        session_id: i64,
    ) -> anyhow::Result<Vec<crate::settings_management::models::ProjectSetting>> {
        let conn = self.connection.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, session_id, tool_call_id, setting_key, setting_name, description, setting_type, setting_value, created_at, updated_at
                FROM project_settings
                WHERE session_id = ?1
                ORDER BY created_at ASC",
        )?;

        let settings = stmt.query_map([session_id], |row| {
            Ok(crate::settings_management::models::ProjectSetting {
                id: row.get(0)?,
                session_id: row.get(1)?,
                tool_call_id: row.get(2)?,
                setting_key: row.get(3)?,
                setting_name: row.get(4)?,
                description: row.get(5)?,
                setting_type: row.get(6)?,
                setting_value: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;

        let mut result = Vec::new();
        for setting in settings {
            result.push(setting?);
        }

        Ok(result)
    }
}
