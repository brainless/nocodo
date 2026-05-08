CREATE TABLE IF NOT EXISTS project_context (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id   INTEGER NOT NULL REFERENCES project(id),
    context_type TEXT    NOT NULL,
    context      TEXT    NOT NULL,
    created_at   INTEGER NOT NULL,
    updated_at   INTEGER NOT NULL,
    UNIQUE(project_id, context_type)
);