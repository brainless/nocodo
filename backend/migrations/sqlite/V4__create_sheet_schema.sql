-- ============================================================================
-- V4__create_sheet_schema.sql
-- ============================================================================

-- Sheets group related tabs together (like a database/schema)
CREATE TABLE IF NOT EXISTS sheet (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id  INTEGER NOT NULL REFERENCES project(id),
    name        TEXT    NOT NULL,
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);

-- Sheet tabs (like tables/spreadsheet pages within a sheet)
CREATE TABLE IF NOT EXISTS sheet_tab (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    sheet_id        INTEGER NOT NULL REFERENCES sheet(id) ON DELETE CASCADE,
    name            TEXT    NOT NULL,
    display_order   INTEGER NOT NULL DEFAULT 0,
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);

-- Column definitions for each sheet tab (the schema)
CREATE TABLE IF NOT EXISTS sheet_tab_column (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    sheet_tab_id    INTEGER NOT NULL REFERENCES sheet_tab(id) ON DELETE CASCADE,
    name            TEXT    NOT NULL,
    -- JSON: {"type": "text"} or {"type": "relation", "target_sheet_tab_id": 5, ...}
    column_type     TEXT    NOT NULL,
    is_required     INTEGER NOT NULL DEFAULT 0,
    is_unique       INTEGER NOT NULL DEFAULT 0,
    default_value   TEXT,
    display_order   INTEGER NOT NULL DEFAULT 0,
    created_at      INTEGER NOT NULL
);

-- Rows store JSON data keyed by column_id
-- Example: {"1": "Alice", "2": "Acme Inc", "3": "qualified"}
CREATE TABLE IF NOT EXISTS sheet_tab_row (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    sheet_tab_id    INTEGER NOT NULL REFERENCES sheet_tab(id) ON DELETE CASCADE,
    -- JSON map: {column_id as string: cell value}
    data            TEXT    NOT NULL DEFAULT '{}',
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_sheet_project ON sheet(project_id);
CREATE INDEX IF NOT EXISTS idx_sheet_tab_sheet ON sheet_tab(sheet_id);
CREATE INDEX IF NOT EXISTS idx_sheet_tab_column_tab ON sheet_tab_column(sheet_tab_id);
CREATE INDEX IF NOT EXISTS idx_sheet_tab_row_tab ON sheet_tab_row(sheet_tab_id);
