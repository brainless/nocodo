/// Create the agent_messages table for storing conversation messages
pub fn migration() -> String {
    r#"
CREATE TABLE agent_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system', 'tool')),
    content TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (session_id) REFERENCES agent_sessions (id) ON DELETE CASCADE
);

CREATE INDEX idx_agent_messages_session_created
    ON agent_messages(session_id, created_at);
"#.to_string()
}
