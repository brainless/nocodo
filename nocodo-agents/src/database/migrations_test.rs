#[cfg(test)]
mod tests {
    use crate::database::migrations::run_agent_migrations;
    use rusqlite::Connection;

    #[test]
    fn test_agent_migrations() {
        let mut conn = Connection::open_in_memory().unwrap();

        // Run migrations
        run_agent_migrations(&mut conn).expect("Migrations should succeed");

        // Verify tables exist
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert!(tables.contains(&"agent_sessions".to_string()));
        assert!(tables.contains(&"agent_messages".to_string()));
        assert!(tables.contains(&"agent_tool_calls".to_string()));
        assert!(tables.contains(&"project_requirements_qna".to_string()));
        assert!(tables.contains(&"project_settings".to_string()));
        assert!(tables.contains(&"refinery_schema_history".to_string()));

        // Verify we can insert data
        conn.execute(
            "INSERT INTO agent_sessions (agent_name, provider, model, user_prompt, started_at)
             VALUES ('test', 'openai', 'gpt-4', 'test prompt', 1234567890)",
            [],
        )
        .unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM agent_sessions", [], |row| row.get(0))
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_migrations_are_idempotent() {
        let mut conn = Connection::open_in_memory().unwrap();

        // Run migrations twice
        run_agent_migrations(&mut conn).expect("First migration should succeed");
        run_agent_migrations(&mut conn).expect("Second migration should succeed");

        // Verify migrations were only applied once
        let migration_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM refinery_schema_history", [], |row| {
                row.get(0)
            })
            .unwrap();

        assert_eq!(migration_count, 5); // We have 5 migration files (3 core + 1 requirements + 1 settings)
    }
}
