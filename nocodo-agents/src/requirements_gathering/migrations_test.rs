#[cfg(test)]
mod tests {
    use crate::database::migrations::run_agent_migrations;
    use rusqlite::Connection;

    #[test]
    fn test_requirements_migrations() {
        let mut conn = Connection::open_in_memory().unwrap();

        // Run all agent migrations (includes requirements gathering migrations)
        run_agent_migrations(&mut conn).expect("Migrations should succeed");

        // Verify table exists
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert!(tables.contains(&"project_requirements_qna".to_string()));

        // Verify we can insert data (requires agent_sessions to exist first)
        conn.execute(
            "INSERT INTO agent_sessions (agent_name, provider, model, user_prompt, started_at)
             VALUES ('test', 'openai', 'gpt-4', 'test prompt', 1234567890)",
            [],
        ).unwrap();

        let session_id: i64 = conn.last_insert_rowid();

        conn.execute(
            "INSERT INTO project_requirements_qna (session_id, question_id, question, response_type, created_at)
             VALUES (?, 'q1', 'What is your goal?', 'text', 1234567890)",
            [session_id],
        ).unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM project_requirements_qna", [], |row| row.get(0))
            .unwrap();

        assert_eq!(count, 1);
    }
}
