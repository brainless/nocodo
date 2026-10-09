//! Apply pending migrations to `data.sqlite`, with a backup before each one.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use diesel::migration::MigrationConnection;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Text};
use diesel_migrations::{FileBasedMigrations, MigrationHarness};
use nocodo_types::apply::{AppliedMigration, ApplyReport, FailedMigration};

type RowCounts = (BTreeMap<String, u64>, BTreeMap<String, u64>);

pub(crate) const DB_FILE: &str = "data.sqlite";
const BACKUP_DIR: &str = "backups";

/// A problem that stops the run before any migration starts.
#[derive(Debug)]
pub enum RuntimeError {
    /// `migrations/` could not be read.
    Migrations(String),
    /// A file or folder could not be created or read.
    Io(io::Error),
    /// The existing `data.sqlite` could not be opened or inspected.
    Database(String),
    /// The request cannot be carried out (e.g. sample rows while a migration is pending).
    Refused(String),
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Migrations(e) => write!(f, "could not read the migrations folder: {e}"),
            Self::Io(e) => write!(f, "file problem: {e}"),
            Self::Database(e) => write!(f, "could not open data.sqlite: {e}"),
            Self::Refused(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for RuntimeError {}

impl From<io::Error> for RuntimeError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<diesel::result::Error> for RuntimeError {
    fn from(e: diesel::result::Error) -> Self {
        Self::Database(e.to_string())
    }
}

#[derive(QueryableByName)]
struct TableName {
    #[diesel(sql_type = Text)]
    name: String,
}

#[derive(QueryableByName)]
pub(crate) struct Count {
    #[diesel(sql_type = BigInt)]
    pub(crate) n: i64,
}

pub(crate) fn quote_ident(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

pub(crate) fn quote_str(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// Row counts of every user table (ignores `sqlite_*` and the Diesel bookkeeping table).
pub(crate) fn count_rows(conn: &mut SqliteConnection) -> QueryResult<BTreeMap<String, u64>> {
    let tables = diesel::sql_query(
        "SELECT name FROM sqlite_master WHERE type = 'table' \
         AND name NOT LIKE 'sqlite\\_%' ESCAPE '\\' \
         AND name != '__diesel_schema_migrations' ORDER BY name",
    )
    .load::<TableName>(conn)?;
    let mut counts = BTreeMap::new();
    for t in tables {
        let c = diesel::sql_query(format!(
            "SELECT COUNT(*) AS n FROM {}",
            quote_ident(&t.name)
        ))
        .get_result::<Count>(conn)?;
        counts.insert(t.name, c.n as u64);
    }
    Ok(counts)
}

/// Pick a backup file name that is not taken: `before_<m>.sqlite`, then `before_<m>-2.sqlite`, ...
fn free_backup_name(project_dir: &Path, migration: &str) -> String {
    let mut n = 1;
    loop {
        let name = if n == 1 {
            format!("{BACKUP_DIR}/before_{migration}.sqlite")
        } else {
            format!("{BACKUP_DIR}/before_{migration}-{n}.sqlite")
        };
        if !project_dir.join(&name).exists() {
            return name;
        }
        n += 1;
    }
}

fn sidecar(db: &Path, suffix: &str) -> PathBuf {
    let mut s = db.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

fn remove_sidecars(db: &Path) {
    for suffix in ["-journal", "-wal", "-shm"] {
        let _ = fs::remove_file(sidecar(db, suffix));
    }
}

/// Put `data.sqlite` back as it was. Returns whether that worked.
fn restore(db: &Path, backup: Option<&Path>) -> bool {
    let ok = match backup {
        Some(b) => fs::copy(b, db).is_ok(),
        None => match fs::remove_file(db) {
            Ok(()) => true,
            Err(e) => e.kind() == io::ErrorKind::NotFound,
        },
    };
    remove_sidecars(db);
    ok
}

/// Steps of one migration. Errors are verbatim text.
fn run_one(
    db: &Path,
    migration: &dyn diesel::migration::Migration<diesel::sqlite::Sqlite>,
) -> Result<RowCounts, String> {
    let mut conn = SqliteConnection::establish(&db.to_string_lossy()).map_err(|e| e.to_string())?;
    conn.setup().map_err(|e| e.to_string())?;
    let before = count_rows(&mut conn).map_err(|e| e.to_string())?;
    conn.run_migration(migration).map_err(|e| e.to_string())?;
    let after = count_rows(&mut conn).map_err(|e| e.to_string())?;
    Ok((before, after))
}

/// Apply every pending migration in `<project>/migrations/`, one at a time.
///
/// Before each migration on an existing `data.sqlite` the file is backed up into
/// `backups/`. On any failure the database is put back exactly as it was and
/// later migrations are not attempted.
pub fn apply_pending(project_dir: &Path) -> Result<ApplyReport, RuntimeError> {
    let db = project_dir.join(DB_FILE);
    let migrations = FileBasedMigrations::from_path(project_dir.join("migrations"))
        .map_err(|e| RuntimeError::Migrations(e.to_string()))?;

    let pending = if db.exists() {
        let mut conn = SqliteConnection::establish(&db.to_string_lossy())
            .map_err(|e| RuntimeError::Database(e.to_string()))?;
        conn.pending_migrations(migrations)
            .map_err(|e| RuntimeError::Database(e.to_string()))?
    } else {
        let all =
            diesel::migration::MigrationSource::<diesel::sqlite::Sqlite>::migrations(&migrations)
                .map_err(|e| RuntimeError::Migrations(e.to_string()))?;
        // Same order Diesel uses for pending migrations.
        let mut all = all;
        all.sort_by_key(|m| m.name().to_string());
        all
    };

    let mut report = ApplyReport::default();
    if pending.is_empty() {
        return Ok(report);
    }
    fs::create_dir_all(project_dir.join(BACKUP_DIR))?;

    for m in pending {
        let name = m.name().to_string();
        let existed = db.exists();

        let backup_rel = if existed {
            let rel = free_backup_name(project_dir, &name);
            let backup_path = project_dir.join(&rel);
            let result = SqliteConnection::establish(&db.to_string_lossy())
                .map_err(|e| e.to_string())
                .and_then(|mut conn| {
                    diesel::sql_query(format!(
                        "VACUUM INTO {}",
                        quote_str(&backup_path.to_string_lossy())
                    ))
                    .execute(&mut conn)
                    .map(|_| ())
                    .map_err(|e| e.to_string())
                });
            if let Err(error) = result {
                // Nothing was changed; drop a partial backup file.
                let _ = fs::remove_file(&backup_path);
                report.failed = Some(FailedMigration {
                    migration: name,
                    backup: None,
                    error: format!("backup failed: {error}"),
                    restored: true,
                });
                return Ok(report);
            }
            Some(rel)
        } else {
            None
        };

        match run_one(&db, &*m) {
            Ok((rows_before, rows_after)) => report.applied.push(AppliedMigration {
                migration: name,
                backup: backup_rel,
                rows_before,
                rows_after,
            }),
            Err(error) => {
                let backup_path = backup_rel.as_ref().map(|r| project_dir.join(r));
                let restored = restore(&db, backup_path.as_deref());
                report.failed = Some(FailedMigration {
                    migration: name,
                    backup: backup_rel,
                    error,
                    restored,
                });
                return Ok(report);
            }
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn add_migration(project: &Path, name: &str, up: &str) {
        let dir = project.join("migrations").join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("up.sql"), up).unwrap();
        fs::write(dir.join("down.sql"), "SELECT 1;").unwrap();
        fs::write(dir.join("metadata.toml"), "run_in_transaction = false\n").unwrap();
    }

    fn project() -> TempDir {
        let p = tempfile::tempdir().unwrap();
        fs::create_dir_all(p.path().join("migrations")).unwrap();
        p
    }

    fn exec(project: &Path, sql: &str) {
        let mut c = SqliteConnection::establish(&project.join(DB_FILE).to_string_lossy()).unwrap();
        diesel::sql_query(sql).execute(&mut c).unwrap();
    }

    fn versions(path: &Path) -> Vec<String> {
        #[derive(QueryableByName)]
        struct V {
            #[diesel(sql_type = Text)]
            version: String,
        }
        let mut c = SqliteConnection::establish(&path.to_string_lossy()).unwrap();
        diesel::sql_query("SELECT version FROM __diesel_schema_migrations ORDER BY version")
            .load::<V>(&mut c)
            .unwrap()
            .into_iter()
            .map(|v| v.version)
            .collect()
    }

    const CREATE_A: &str = "CREATE TABLE a (id INTEGER PRIMARY KEY, v TEXT);";
    const BAD: &str =
        "CREATE TABLE b (id INTEGER PRIMARY KEY);\nINSERT INTO a (v) VALUES ('x');\nNOT VALID SQL;";

    #[test]
    fn first_migration_creates_db_without_backup() {
        let p = project();
        add_migration(p.path(), "0001_a", CREATE_A);
        let r = apply_pending(p.path()).unwrap();
        assert_eq!(r.failed, None);
        assert_eq!(r.applied.len(), 1);
        assert_eq!(r.applied[0].backup, None);
        assert!(r.failed.is_none());
        assert!(p.path().join(DB_FILE).exists());
        assert_eq!(fs::read_dir(p.path().join(BACKUP_DIR)).unwrap().count(), 0);
        // Nothing pending now: empty report.
        let again = apply_pending(p.path()).unwrap();
        assert_eq!(again, ApplyReport::default());
    }

    #[test]
    fn nothing_pending_touches_nothing() {
        let p = project();
        let r = apply_pending(p.path()).unwrap();
        assert_eq!(r, ApplyReport::default());
        assert!(!p.path().join(DB_FILE).exists());
        assert!(!p.path().join(BACKUP_DIR).exists());
    }

    #[test]
    fn backup_holds_rows_from_before() {
        let p = project();
        add_migration(p.path(), "0001_a", CREATE_A);
        apply_pending(p.path()).unwrap();
        exec(p.path(), "INSERT INTO a (v) VALUES ('one'), ('two');");
        add_migration(
            p.path(),
            "0002_c",
            "CREATE TABLE c (id INTEGER PRIMARY KEY);",
        );
        let r = apply_pending(p.path()).unwrap();
        let ap = &r.applied[0];
        assert_eq!(ap.backup.as_deref(), Some("backups/before_0002_c.sqlite"));
        assert_eq!(ap.rows_before["a"], 2);
        assert_eq!(ap.rows_after["a"], 2);
        assert_eq!(ap.rows_after["c"], 0);
        let mut b = SqliteConnection::establish(
            &p.path()
                .join("backups/before_0002_c.sqlite")
                .to_string_lossy(),
        )
        .unwrap();
        let counts = count_rows(&mut b).unwrap();
        assert_eq!(counts["a"], 2);
        assert!(!counts.contains_key("c"));
    }

    #[test]
    fn failure_restores_and_stops() {
        let p = project();
        add_migration(p.path(), "0001_a", CREATE_A);
        apply_pending(p.path()).unwrap();
        exec(p.path(), "INSERT INTO a (v) VALUES ('one');");
        let db = p.path().join(DB_FILE);
        let before_bytes_rows = {
            let mut c = SqliteConnection::establish(&db.to_string_lossy()).unwrap();
            count_rows(&mut c).unwrap()
        };
        add_migration(p.path(), "0002_bad", BAD);
        add_migration(
            p.path(),
            "0003_later",
            "CREATE TABLE later (id INTEGER PRIMARY KEY);",
        );
        let r = apply_pending(p.path()).unwrap();
        assert!(r.applied.is_empty());
        let f = r.failed.unwrap();
        assert_eq!(f.migration, "0002_bad");
        assert!(f.restored);
        assert!(f.error.contains("syntax error"), "{}", f.error);
        assert_eq!(f.backup.as_deref(), Some("backups/before_0002_bad.sqlite"));
        assert!(p.path().join("backups/before_0002_bad.sqlite").exists());
        let mut c = SqliteConnection::establish(&db.to_string_lossy()).unwrap();
        assert_eq!(count_rows(&mut c).unwrap(), before_bytes_rows);
        assert_eq!(versions(&db).len(), 1);

        // Retry never overwrites the backup.
        let r = apply_pending(p.path()).unwrap();
        assert!(r.failed.is_some());
        assert!(p.path().join("backups/before_0002_bad-2.sqlite").exists());
        assert!(p.path().join("backups/before_0002_bad.sqlite").exists());
    }

    #[test]
    fn failure_on_first_migration_leaves_no_database() {
        let p = project();
        add_migration(
            p.path(),
            "0001_bad",
            "CREATE TABLE b (id INTEGER);\nNOT VALID SQL;",
        );
        let r = apply_pending(p.path()).unwrap();
        let f = r.failed.unwrap();
        assert!(f.restored);
        assert_eq!(f.backup, None);
        assert!(!p.path().join(DB_FILE).exists());
    }
}
