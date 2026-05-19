CREATE TABLE project_note (
    id                      INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id              INTEGER NOT NULL REFERENCES project(id),
    topic                   TEXT NOT NULL,
    title                   TEXT NOT NULL,
    note                    TEXT NOT NULL,
    source_session_id       INTEGER NULL REFERENCES user_chat_session(id),
    source_epic_comment_id  INTEGER NULL REFERENCES epic_comment(id),
    source_task_comment_id  INTEGER NULL REFERENCES task_comment(id),
    replaces_id             INTEGER NULL REFERENCES project_note(id),
    created_at              INTEGER NOT NULL,
    CHECK (NOT (source_epic_comment_id IS NOT NULL AND source_task_comment_id IS NOT NULL))
);

CREATE INDEX idx_project_note_project_topic ON project_note(project_id, topic);
