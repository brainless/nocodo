use crate::storage::migrations;
use crate::storage::SqliteAgentStorage;
use crate::DbConnection;
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub fn initialize_database(
    db_path: &PathBuf,
) -> anyhow::Result<(DbConnection, Arc<SqliteAgentStorage>)> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut conn = Connection::open(&db_path)?;
    conn.execute("PRAGMA foreign_keys = ON", [])?;

    migrations::run_migrations(&mut conn)?;

    let db_connection = Arc::new(Mutex::new(conn));
    let storage = Arc::new(SqliteAgentStorage::new(db_connection.clone()));

    Ok((db_connection, storage))
}
