CREATE TABLE IF NOT EXISTS project_schema (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id  INTEGER NOT NULL REFERENCES project(id),
    session_id  INTEGER NOT NULL REFERENCES agent_chat_session(id),
    schema_json TEXT    NOT NULL,  -- serialized GenerateSchemaParams JSON
    version     INTEGER NOT NULL DEFAULT 1,
    created_at  INTEGER NOT NULL
);
