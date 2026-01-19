/// Create the project_settings table for storing user settings collected by the settings management agent
///
/// **Agent-Specific Migration**: This table is specific to the settings_management agent.
/// Applications that don't use the settings management agent can skip this migration.
pub fn migration() -> String {
    r#"
CREATE TABLE project_settings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL,
    tool_call_id INTEGER,
    setting_key TEXT NOT NULL,
    setting_name TEXT NOT NULL,
    description TEXT,
    setting_type TEXT NOT NULL DEFAULT 'text',
    setting_value TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER,
    FOREIGN KEY (session_id) REFERENCES agent_sessions (id) ON DELETE CASCADE,
    FOREIGN KEY (tool_call_id) REFERENCES agent_tool_calls (id) ON DELETE CASCADE
);

CREATE INDEX idx_project_settings_session
    ON project_settings(session_id);

CREATE INDEX idx_project_settings_key
    ON project_settings(setting_key);
"#
    .to_string()
}
