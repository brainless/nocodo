/// Create the project_requirements_qna table for storing user questions and answers
///
/// **Agent-Specific Migration**: This table is specific to the requirements_gathering agent.
/// Applications that don't use the requirements gathering agent can skip this migration.
pub fn migration() -> String {
    r#"
CREATE TABLE project_requirements_qna (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL,
    tool_call_id INTEGER,
    question_id TEXT NOT NULL,
    question TEXT NOT NULL,
    description TEXT,
    response_type TEXT NOT NULL DEFAULT 'text',
    answer TEXT,
    created_at INTEGER NOT NULL,
    answered_at INTEGER,
    FOREIGN KEY (session_id) REFERENCES agent_sessions (id) ON DELETE CASCADE,
    FOREIGN KEY (tool_call_id) REFERENCES agent_tool_calls (id) ON DELETE CASCADE
);

CREATE INDEX idx_project_requirements_qna_session
    ON project_requirements_qna(session_id);
"#
    .to_string()
}
