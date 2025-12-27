use rusqlite::Connection;
use anyhow::Result;

pub fn initialize_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    conn.execute_batch(r#"
        CREATE TABLE IF NOT EXISTS items (
            id INTEGER PRIMARY KEY,
            type TEXT NOT NULL,
            by TEXT,
            time INTEGER NOT NULL,
            deleted INTEGER DEFAULT 0,
            dead INTEGER DEFAULT 0,
            parent INTEGER,
            poll INTEGER,
            kids TEXT,
            url TEXT,
            score INTEGER,
            title TEXT,
            text TEXT,
            parts TEXT,
            descendants INTEGER,
            fetched_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            created INTEGER NOT NULL,
            karma INTEGER NOT NULL,
            about TEXT,
            submitted TEXT,
            fetched_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS download_state (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            mode TEXT NOT NULL,
            story_types TEXT,
            current_max_id INTEGER,
            batch_size INTEGER NOT NULL DEFAULT 20,
            updated_at INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS fetch_queue (
            item_id INTEGER PRIMARY KEY,
            queued_at INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_items_by ON items(by);
        CREATE INDEX IF NOT EXISTS idx_items_type ON items(type);
        CREATE INDEX IF NOT EXISTS idx_items_parent ON items(parent);
        CREATE INDEX IF NOT EXISTS idx_items_time ON items(time DESC);
        CREATE INDEX IF NOT EXISTS idx_fetch_queue_queued_at ON fetch_queue(queued_at);
    "#)?;

    Ok(())
}
