CREATE TABLE IF NOT EXISTS agent_chat_session (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id   INTEGER NOT NULL REFERENCES project(id),
    agent_type   TEXT    NOT NULL,
    created_at   INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS agent_chat_message (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id   INTEGER NOT NULL REFERENCES agent_chat_session(id),
    role         TEXT    NOT NULL,  -- 'user' | 'assistant' | 'tool'
    content      TEXT    NOT NULL,
    tool_call_id TEXT,              -- set for role='tool', holds LLM's call_id
    created_at   INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS agent_tool_call (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    message_id   INTEGER NOT NULL REFERENCES agent_chat_message(id),
    call_id      TEXT    NOT NULL,  -- LLM-assigned call identifier
    tool_name    TEXT    NOT NULL,
    arguments    TEXT    NOT NULL,  -- JSON
    result       TEXT,
    created_at   INTEGER NOT NULL
);
