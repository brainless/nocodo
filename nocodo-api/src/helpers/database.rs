use crate::DbConnection;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub fn initialize_database(
) -> anyhow::Result<(DbConnection, Arc<nocodo_agents::database::Database>)> {
    let db_path = super::agents::get_api_db_path()?;

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let conn = Connection::open(&db_path)?;
    conn.execute("PRAGMA foreign_keys = ON", [])?;

    let db = Arc::new(nocodo_agents::database::Database::new(&db_path)?);

    Ok((Arc::new(Mutex::new(conn)), db))
}
