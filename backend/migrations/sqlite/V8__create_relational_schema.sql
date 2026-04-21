-- ============================================================================
-- V8__create_relational_schema.sql
--
-- Canonical relational schema tables replacing the old sheet/sheet_tab/
-- sheet_tab_column approach. The old tables are kept intact for now.
-- ============================================================================

-- A Schema is a named collection of tables within a project.
CREATE TABLE IF NOT EXISTS app_schema (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id  INTEGER NOT NULL REFERENCES project(id),
    name        TEXT    NOT NULL,
    created_at  INTEGER NOT NULL
);

-- A Table is a relational table within a schema.
CREATE TABLE IF NOT EXISTS schema_table (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    schema_id   INTEGER NOT NULL REFERENCES app_schema(id) ON DELETE CASCADE,
    name        TEXT    NOT NULL,
    created_at  INTEGER NOT NULL
);

-- A Column in a relational table.
-- data_type is stored as a snake_case string: text|integer|real|boolean|date|date_time
CREATE TABLE IF NOT EXISTS schema_column (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    table_id        INTEGER NOT NULL REFERENCES schema_table(id) ON DELETE CASCADE,
    name            TEXT    NOT NULL,
    data_type       TEXT    NOT NULL DEFAULT 'text',
    nullable        INTEGER NOT NULL DEFAULT 1,
    primary_key     INTEGER NOT NULL DEFAULT 0,
    display_order   INTEGER NOT NULL DEFAULT 0,
    created_at      INTEGER NOT NULL
);

-- A foreign key constraint on a column (name-based, resolved at query time).
CREATE TABLE IF NOT EXISTS schema_fk (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    column_id   INTEGER NOT NULL REFERENCES schema_column(id) ON DELETE CASCADE,
    ref_table   TEXT    NOT NULL,
    ref_column  TEXT    NOT NULL DEFAULT 'id'
);

-- UI display metadata for a column (decoupled from relational schema).
CREATE TABLE IF NOT EXISTS column_display (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    column_id       INTEGER NOT NULL REFERENCES schema_column(id) ON DELETE CASCADE,
    width           INTEGER NOT NULL DEFAULT 120,
    display_column  TEXT
);

CREATE INDEX IF NOT EXISTS idx_app_schema_project   ON app_schema(project_id);
CREATE INDEX IF NOT EXISTS idx_schema_table_schema  ON schema_table(schema_id);
CREATE INDEX IF NOT EXISTS idx_schema_column_table  ON schema_column(table_id);
CREATE INDEX IF NOT EXISTS idx_schema_fk_column     ON schema_fk(column_id);
CREATE INDEX IF NOT EXISTS idx_column_display_col   ON column_display(column_id);
