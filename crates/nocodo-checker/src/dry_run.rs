//! Dry run: replay migrations on a throwaway copy of the DB.

use std::fs;
use std::io;
use std::path::Path;

use diesel::prelude::*;
use diesel::sql_types::Text;
use diesel_migrations::{FileBasedMigrations, MigrationHarness};
use nocodo_sqlgen::METADATA_TOML;
use nocodo_types::event::Finding;
use nocodo_types::schema::SchemaSpec;

use crate::introspect::{Shape, diff, diff_shapes, expected_shape, introspect};

/// A migration that has not been written to the project yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedMigration {
    /// Folder name, e.g. `0002_add_family_member`.
    pub name: String,
    pub up: String,
    pub down: String,
}

/// The dry run could not be carried out (not a finding about the migration).
#[derive(Debug)]
pub enum CheckerError {
    Io(io::Error),
}

impl std::fmt::Display for CheckerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "dry run could not prepare its temporary files: {e}"),
        }
    }
}

impl std::error::Error for CheckerError {}

impl From<io::Error> for CheckerError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

fn finding(code: &str, message: String) -> Vec<Finding> {
    vec![Finding {
        code: code.to_string(),
        message,
    }]
}

fn sql_error(step: &str, e: impl std::fmt::Display) -> Vec<Finding> {
    finding("sql_error", format!("SQLite error while {step}: {e}"))
}

fn copy_dir(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    if !from.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn mismatch(code: &str, what: &str, lines: Vec<String>) -> Vec<Finding> {
    finding(code, format!("{what}: {}", lines.join("; ")))
}

#[derive(QueryableByName)]
struct FkViolation {
    #[diesel(sql_type = Text)]
    table: String,
}

/// Run all earlier migrations plus `new` on a copy of `db`.
///
/// `before` is the spec the DB should match before `new`; `after` the spec it
/// should match afterwards. An empty result means the migration passed. The
/// real `db` and `migrations_dir` are never modified.
pub fn dry_run(
    db: Option<&Path>,
    migrations_dir: &Path,
    new: &NamedMigration,
    before: &SchemaSpec,
    after: &SchemaSpec,
) -> Result<Vec<Finding>, CheckerError> {
    let tmp = tempfile::tempdir()?;
    let db_path = tmp.path().join("data.sqlite");
    match db {
        Some(p) if p.exists() => {
            fs::copy(p, &db_path)?;
        }
        _ => {}
    }
    let mig_dir = tmp.path().join("migrations");
    copy_dir(migrations_dir, &mig_dir)?;

    let mut conn = match SqliteConnection::establish(&db_path.to_string_lossy()) {
        Ok(c) => c,
        Err(e) => return Ok(sql_error("opening the scratch database", e)),
    };

    // 2. Earlier migrations, stopping before the new one.
    let earlier = match FileBasedMigrations::from_path(&mig_dir) {
        Ok(m) => m,
        Err(e) => return Ok(sql_error("reading earlier migrations", e)),
    };
    if let Err(e) = conn.run_pending_migrations(earlier) {
        return Ok(sql_error("running earlier migrations", e));
    }

    // 3. The DB must match the spec before the new migration.
    let start = match introspect(&mut conn) {
        Ok(s) => s,
        Err(e) => return Ok(sql_error("reading the database structure", e)),
    };
    let problems = diff(&expected_shape(before), &start);
    if !problems.is_empty() {
        return Ok(mismatch(
            "db_does_not_match_spec",
            "The database after earlier migrations does not match the current schema",
            problems,
        ));
    }

    // 4. Add and run the new migration.
    let new_dir = mig_dir.join(&new.name);
    fs::create_dir_all(&new_dir)?;
    fs::write(new_dir.join("up.sql"), &new.up)?;
    fs::write(new_dir.join("down.sql"), &new.down)?;
    fs::write(new_dir.join("metadata.toml"), METADATA_TOML)?;
    let all = match FileBasedMigrations::from_path(&mig_dir) {
        Ok(m) => m,
        Err(e) => return Ok(sql_error("reading the new migration", e)),
    };

    if let Err(e) = conn.run_pending_migrations(all.clone()) {
        return Ok(sql_error("running the new migration", e));
    }
    let findings = check_after(&mut conn, after, "running the new migration");
    if !findings.is_empty() {
        return Ok(findings);
    }

    if let Err(e) = conn.revert_last_migration(all.clone()) {
        return Ok(sql_error("reverting the new migration", e));
    }
    let reverted = match introspect(&mut conn) {
        Ok(s) => s,
        Err(e) => return Ok(sql_error("reading the database structure", e)),
    };
    let problems = diff_shapes(&start, &reverted);
    if !problems.is_empty() {
        return Ok(mismatch(
            "not_reversible",
            "After reverting, the structure is not what it was before",
            problems,
        ));
    }

    if let Err(e) = conn.run_pending_migrations(all) {
        return Ok(sql_error("running the new migration a second time", e));
    }
    Ok(check_after(
        &mut conn,
        after,
        "running the new migration a second time",
    ))
}

fn check_after(conn: &mut SqliteConnection, after: &SchemaSpec, step: &str) -> Vec<Finding> {
    let shape: Shape = match introspect(conn) {
        Ok(s) => s,
        Err(e) => return sql_error("reading the database structure", e),
    };
    let problems = diff(&expected_shape(after), &shape);
    if !problems.is_empty() {
        return mismatch(
            "db_does_not_match_spec",
            &format!("After {step}, the database does not match the new schema"),
            problems,
        );
    }
    match diesel::sql_query("PRAGMA foreign_key_check").load::<FkViolation>(conn) {
        Ok(rows) if !rows.is_empty() => {
            let mut tables: Vec<String> = rows.into_iter().map(|r| r.table).collect();
            tables.sort();
            tables.dedup();
            finding(
                "foreign_key_violation",
                format!(
                    "After {step}, rows in {} point at rows that do not exist",
                    tables.join(", ")
                ),
            )
        }
        Ok(_) => Vec::new(),
        Err(e) => sql_error("checking foreign keys", e),
    }
}
