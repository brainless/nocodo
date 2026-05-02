-- Epic: high-level goal spanning multiple agents
CREATE TABLE epic (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id       INTEGER NOT NULL REFERENCES project(id),
    title            TEXT    NOT NULL,
    description      TEXT    NOT NULL,
    source_prompt    TEXT    NOT NULL,
    status           TEXT    NOT NULL DEFAULT 'open',
    created_by_agent TEXT    NOT NULL,
    created_at       INTEGER NOT NULL,
    updated_at       INTEGER NOT NULL
);

-- Task: unit of work assigned to one agent
CREATE TABLE task (
    id                 INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id         INTEGER NOT NULL REFERENCES project(id),
    epic_id            INTEGER REFERENCES epic(id),
    title              TEXT    NOT NULL,
    description        TEXT    NOT NULL,
    source_prompt      TEXT    NOT NULL,
    assigned_to_agent  TEXT    NOT NULL,
    status             TEXT    NOT NULL DEFAULT 'open',
    depends_on_task_id INTEGER REFERENCES task(id),
    created_by_agent   TEXT    NOT NULL,
    created_at         INTEGER NOT NULL,
    updated_at         INTEGER NOT NULL
);

-- Rebuild agent_chat_session scoped to tasks (task_id NOT NULL, clean break)
DROP TABLE IF EXISTS agent_chat_message;
DROP TABLE IF EXISTS agent_chat_session;

CREATE TABLE agent_chat_session (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL REFERENCES project(id),
    agent_type TEXT    NOT NULL,
    task_id    INTEGER NOT NULL REFERENCES task(id),
    created_at INTEGER NOT NULL
);

CREATE TABLE agent_chat_message (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id   INTEGER NOT NULL REFERENCES agent_chat_session(id),
    role         TEXT    NOT NULL,
    agent_type   TEXT,
    content      TEXT    NOT NULL,
    tool_call_id TEXT,
    tool_name    TEXT,
    turn_id      INTEGER,
    created_at   INTEGER NOT NULL
);
