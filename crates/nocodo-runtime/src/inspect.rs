//! `inspect`: what is stored, which migrations ran, how many backups exist.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use diesel::prelude::*;
use diesel_migrations::{FileBasedMigrations, MigrationHarness};

use crate::apply::{DB_FILE, RuntimeError, count_rows};

/// What `data.sqlite` and the project folder hold right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inspection {
    /// Row count per user table (empty without a `data.sqlite`).
    pub tables: BTreeMap<String, u64>,
    /// Migrations already recorded in `data.sqlite`, in order.
    pub applied: Vec<String>,
    /// Migrations not yet applied, in order.
    pub pending: Vec<String>,
    /// Number of files in `backups/`.
    pub backups: usize,
}

/// `file:` URI that opens `db` read-only; `%`, `?` and `#` in the path are escaped.
pub(crate) fn read_only_uri(db: &Path) -> String {
    let mut out = String::from("file:");
    for c in db.to_string_lossy().chars() {
        match c {
            '%' => out.push_str("%25"),
            '?' => out.push_str("%3f"),
            '#' => out.push_str("%23"),
            c => out.push(c),
        }
    }
    out.push_str("?mode=ro");
    out
}

/// Read-only look at the project. Works without a `data.sqlite`.
pub fn inspect(project_dir: &Path) -> Result<Inspection, RuntimeError> {
    let db = project_dir.join(DB_FILE);
    let migrations = FileBasedMigrations::from_path(project_dir.join("migrations"))
        .map_err(|e| RuntimeError::Migrations(e.to_string()))?;
    let all = diesel::migration::MigrationSource::<diesel::sqlite::Sqlite>::migrations(&migrations)
        .map_err(|e| RuntimeError::Migrations(e.to_string()))?;
    let mut all: Vec<String> = all.iter().map(|m| m.name().to_string()).collect();
    all.sort();

    let (tables, pending) = if db.exists() {
        let url = read_only_uri(&db);
        let mut conn =
            SqliteConnection::establish(&url).map_err(|e| RuntimeError::Database(e.to_string()))?;
        let pending = conn
            .pending_migrations(migrations)
            .map_err(|e| RuntimeError::Database(e.to_string()))?
            .iter()
            .map(|m| m.name().to_string())
            .collect::<Vec<_>>();
        (count_rows(&mut conn)?, pending)
    } else {
        (BTreeMap::new(), all.clone())
    };
    let applied = all.into_iter().filter(|n| !pending.contains(n)).collect();

    let backups = match fs::read_dir(project_dir.join("backups")) {
        Ok(rd) => rd
            .filter_map(Result::ok)
            .filter(|e| e.path().is_file())
            .count(),
        Err(_) => 0,
    };
    Ok(Inspection {
        tables,
        applied,
        pending,
        backups,
    })
}
