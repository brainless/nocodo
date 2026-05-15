-- Canonical task.status values: draft, needs_technical_shaping, ready, in_progress, done, blocked
ALTER TABLE task ADD COLUMN source_session_id INTEGER NULL REFERENCES user_chat_session(id);

CREATE INDEX IF NOT EXISTS idx_task_status_agent ON task(status, assigned_to_agent);
