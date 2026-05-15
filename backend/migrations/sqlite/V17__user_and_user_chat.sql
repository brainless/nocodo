CREATE TABLE IF NOT EXISTS user (
    id            INTEGER PRIMARY KEY,
    display_name  TEXT    NOT NULL,
    email         TEXT    UNIQUE NULL,
    password_hash TEXT    NULL,
    is_guest      BOOLEAN NOT NULL DEFAULT 1,
    created_at    INTEGER NOT NULL,
    updated_at    INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS user_chat_session (
    id                INTEGER PRIMARY KEY,
    project_id        INTEGER NOT NULL REFERENCES project(id),
    created_by_user_id INTEGER NOT NULL REFERENCES user(id),
    status            TEXT    NOT NULL DEFAULT 'open',
    created_at        INTEGER NOT NULL,
    updated_at        INTEGER NOT NULL,
    completed_at      INTEGER NULL
);

CREATE TABLE IF NOT EXISTS user_chat_message (
    id             INTEGER PRIMARY KEY,
    session_id     INTEGER NOT NULL REFERENCES user_chat_session(id),
    author_type    TEXT    NOT NULL,
    author_user_id INTEGER NULL REFERENCES user(id),
    agent_type     TEXT    NULL,
    turn_id        INTEGER NULL,
    content        TEXT    NOT NULL,
    created_at     INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_user_chat_session_project_status_created
    ON user_chat_session(project_id, status, created_at);

CREATE INDEX IF NOT EXISTS idx_user_chat_message_session_created
    ON user_chat_message(session_id, created_at);
