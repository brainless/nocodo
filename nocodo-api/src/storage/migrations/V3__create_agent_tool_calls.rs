/// Create the agent_tool_calls table for tracking tool executions
pub fn migration() -> String {
    r#"
CREATE TABLE agent_tool_calls (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL,
    message_id INTEGER,
    tool_call_id TEXT NOT NULL,
    tool_name TEXT NOT NULL,
    request TEXT NOT NULL,
    response TEXT,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'executing', 'completed', 'failed')),
    created_at INTEGER NOT NULL,
    completed_at INTEGER,
    execution_time_ms INTEGER,
    error_details TEXT,
    FOREIGN KEY (session_id) REFERENCES agent_sessions (id) ON DELETE CASCADE,
    FOREIGN KEY (message_id) REFERENCES agent_messages (id) ON DELETE SET NULL
);

CREATE INDEX idx_agent_tool_calls_session
    ON agent_tool_calls(session_id);

CREATE INDEX idx_agent_tool_calls_status
    ON agent_tool_calls(session_id, status);
"#.to_string()
}
