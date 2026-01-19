/// Create the agent_sessions table for tracking agent execution sessions
pub fn migration() -> String {
    r#"
CREATE TABLE agent_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_name TEXT NOT NULL,
    provider TEXT NOT NULL,
    model TEXT NOT NULL,
    system_prompt TEXT,
    user_prompt TEXT NOT NULL,
    config TEXT,
    status TEXT NOT NULL DEFAULT 'running' CHECK (status IN ('running', 'completed', 'failed', 'waiting_for_user_input')),
    started_at INTEGER NOT NULL,
    ended_at INTEGER,
    result TEXT,
    error TEXT
);
"#.to_string()
}
