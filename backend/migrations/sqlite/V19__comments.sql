CREATE TABLE IF NOT EXISTS epic_comment (
    id             INTEGER PRIMARY KEY,
    epic_id        INTEGER NOT NULL REFERENCES epic(id),
    author_type    TEXT    NOT NULL,
    author_user_id INTEGER NULL REFERENCES user(id),
    agent_type     TEXT    NULL,
    content        TEXT    NOT NULL,
    created_at     INTEGER NOT NULL,
    updated_at     INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS task_comment (
    id             INTEGER PRIMARY KEY,
    task_id        INTEGER NOT NULL REFERENCES task(id),
    author_type    TEXT    NOT NULL,
    author_user_id INTEGER NULL REFERENCES user(id),
    agent_type     TEXT    NULL,
    content        TEXT    NOT NULL,
    created_at     INTEGER NOT NULL,
    updated_at     INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_epic_comment_epic_created
    ON epic_comment(epic_id, created_at);

CREATE INDEX IF NOT EXISTS idx_task_comment_task_created
    ON task_comment(task_id, created_at);
