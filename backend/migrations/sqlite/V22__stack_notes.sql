CREATE TABLE stack_note (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id  INTEGER NOT NULL REFERENCES project(id),
    tag         TEXT NOT NULL,
    note        TEXT NOT NULL,
    file_path   TEXT,
    line_number INTEGER,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);

CREATE INDEX idx_stack_note_project_tag ON stack_note(project_id, tag);
