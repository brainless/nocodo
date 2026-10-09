//! Deterministic fake rows for testing. Derived only from the schema spec.
//!
//! Not a data-entry feature: no values come from the user.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use diesel::prelude::*;
use diesel_migrations::{FileBasedMigrations, MigrationHarness};
use nocodo_types::schema::{Column, ColumnType, SchemaSpec, Table};

use crate::apply::{Count, DB_FILE, RuntimeError, quote_ident, quote_str};

/// The SQL literal for row `n` (1-based) of a plain column.
fn value_sql(c: &Column, n: u64) -> String {
    let day = n.min(28);
    match c.column_type {
        ColumnType::Text if c.unique => quote_str(&format!("{} {n}", c.name)),
        ColumnType::Text => quote_str(&format!("sample {n}")),
        ColumnType::Integer | ColumnType::Money => n.to_string(),
        ColumnType::Decimal => format!("{n}.5"),
        ColumnType::Boolean => (n % 2).to_string(),
        ColumnType::Date => quote_str(&format!("2026-01-{day:02}")),
        ColumnType::DateTime => quote_str(&format!("2026-01-{day:02}T12:00:00Z")),
    }
}

/// Tables in an order that respects references. A nullable reference to a
/// table that cannot be filled first is left empty.
fn fill_order(spec: &SchemaSpec) -> Result<Vec<&Table>, RuntimeError> {
    let names: BTreeSet<&str> = spec.tables.iter().map(|t| t.name.as_str()).collect();
    for t in &spec.tables {
        for c in &t.columns {
            if let Some(r) = &c.references
                && !names.contains(r.as_str())
            {
                return Err(RuntimeError::Refused(format!(
                    "{}.{} points at {r}, which is not in the schema",
                    t.name, c.name
                )));
            }
        }
    }
    let mut done: BTreeSet<&str> = BTreeSet::new();
    let mut order = Vec::new();
    let mut remaining: Vec<&Table> = spec.tables.iter().collect();
    while !remaining.is_empty() {
        let refs_done = |t: &Table, only_required: bool| {
            t.columns.iter().all(|c| match &c.references {
                Some(r) if r != &t.name => {
                    done.contains(r.as_str()) || (only_required && c.nullable)
                }
                _ => true,
            })
        };
        let pos = remaining
            .iter()
            .position(|t| refs_done(t, false))
            .or_else(|| remaining.iter().position(|t| refs_done(t, true)))
            .ok_or_else(|| {
                RuntimeError::Refused(
                    "tables point at each other in a loop that cannot be filled".into(),
                )
            })?;
        let t = remaining.remove(pos);
        done.insert(t.name.as_str());
        order.push(t);
    }
    Ok(order)
}

fn count_table(conn: &mut SqliteConnection, table: &str) -> QueryResult<u64> {
    Ok(
        diesel::sql_query(format!("SELECT COUNT(*) AS n FROM {}", quote_ident(table)))
            .get_result::<Count>(conn)?
            .n as u64,
    )
}

#[derive(QueryableByName)]
struct FkViolation {
    #[diesel(sql_type = diesel::sql_types::Text)]
    table: String,
}

/// Add `per_table` fake rows to every table of `spec` in `<project>/data.sqlite`.
///
/// Returns the number of rows added per table. Refuses when a migration is
/// pending or the schema has no tables. All rows go in one transaction.
pub fn add_sample_rows(
    project_dir: &Path,
    spec: &SchemaSpec,
    per_table: u32,
) -> Result<BTreeMap<String, u64>, RuntimeError> {
    if spec.tables.is_empty() {
        return Err(RuntimeError::Refused(
            "there is nothing to fill yet: the schema has no lists".into(),
        ));
    }
    let db = project_dir.join(DB_FILE);
    if !db.exists() {
        return Err(RuntimeError::Refused(
            "data.sqlite does not exist yet: a migration is pending (run `apply`)".into(),
        ));
    }
    let migrations = FileBasedMigrations::from_path(project_dir.join("migrations"))
        .map_err(|e| RuntimeError::Migrations(e.to_string()))?;
    let mut conn = SqliteConnection::establish(&db.to_string_lossy())
        .map_err(|e| RuntimeError::Database(e.to_string()))?;
    let pending = conn
        .pending_migrations(migrations)
        .map_err(|e| RuntimeError::Database(e.to_string()))?;
    if !pending.is_empty() {
        return Err(RuntimeError::Refused(format!(
            "{} migration(s) are pending (run `apply` first)",
            pending.len()
        )));
    }
    let order = fill_order(spec)?;

    conn.transaction::<_, RuntimeError, _>(|conn| {
        let mut added = BTreeMap::new();
        let mut filled: BTreeSet<&str> = BTreeSet::new();
        for t in order {
            let existing = count_table(conn, &t.name)?;
            for i in 1..=u64::from(per_table) {
                let n = existing + i;
                let mut cols = Vec::new();
                let mut vals = Vec::new();
                for c in &t.columns {
                    let value = match &c.references {
                        None => value_sql(c, n),
                        // Nullable, or a target not filled yet: left empty.
                        Some(r) if c.nullable && (r == &t.name || !filled.contains(r.as_str())) => {
                            continue;
                        }
                        // A required self reference points at the row itself.
                        Some(r) if r == &t.name => format!(
                            "(SELECT COALESCE(MAX(id), 0) + 1 FROM {})",
                            quote_ident(&t.name)
                        ),
                        Some(r) => {
                            let total = count_table(conn, r)?;
                            if total == 0 {
                                return Err(RuntimeError::Refused(format!(
                                    "{} has no rows for {}.{} to point at",
                                    r, t.name, c.name
                                )));
                            }
                            format!(
                                "(SELECT id FROM {} ORDER BY id LIMIT 1 OFFSET {})",
                                quote_ident(r),
                                (n - 1) % total
                            )
                        }
                    };
                    cols.push(quote_ident(&c.name));
                    vals.push(value);
                }
                let sql = if cols.is_empty() {
                    format!("INSERT INTO {} DEFAULT VALUES", quote_ident(&t.name))
                } else {
                    format!(
                        "INSERT INTO {} ({}) VALUES ({})",
                        quote_ident(&t.name),
                        cols.join(", "),
                        vals.join(", ")
                    )
                };
                diesel::sql_query(sql).execute(conn)?;
            }
            filled.insert(t.name.as_str());
            added.insert(t.name.clone(), u64::from(per_table));
        }
        let bad = diesel::sql_query("PRAGMA foreign_key_check").load::<FkViolation>(conn)?;
        if let Some(v) = bad.first() {
            return Err(RuntimeError::Database(format!(
                "sample rows would break a reference in {}",
                v.table
            )));
        }
        Ok(added)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apply::{apply_pending, count_rows};
    use std::fs;

    fn col(name: &str, t: ColumnType, nullable: bool, unique: bool, r: Option<&str>) -> Column {
        Column {
            name: name.into(),
            description: String::new(),
            column_type: t,
            nullable,
            unique,
            references: r.map(Into::into),
        }
    }

    fn table(name: &str, columns: Vec<Column>) -> Table {
        Table {
            name: name.into(),
            description: String::new(),
            columns,
            indexes: vec![],
        }
    }

    fn spec() -> SchemaSpec {
        SchemaSpec {
            spec_version: 1,
            tables: vec![
                table(
                    "notes",
                    vec![
                        col(
                            "person_id",
                            ColumnType::Integer,
                            false,
                            false,
                            Some("people"),
                        ),
                        col("body", ColumnType::Text, false, false, None),
                        col("seen", ColumnType::Boolean, false, false, None),
                    ],
                ),
                table(
                    "people",
                    vec![
                        col("name", ColumnType::Text, false, true, None),
                        col("born", ColumnType::Date, true, false, None),
                        col("score", ColumnType::Decimal, false, false, None),
                    ],
                ),
            ],
        }
    }

    const UP: &str = "CREATE TABLE people (id INTEGER PRIMARY KEY, \
        created_at TEXT NOT NULL DEFAULT 'x', updated_at TEXT NOT NULL DEFAULT 'x', \
        name TEXT NOT NULL UNIQUE, born TEXT, score REAL NOT NULL);\n\
        CREATE TABLE notes (id INTEGER PRIMARY KEY, \
        created_at TEXT NOT NULL DEFAULT 'x', updated_at TEXT NOT NULL DEFAULT 'x', \
        person_id INTEGER NOT NULL REFERENCES people(id), body TEXT NOT NULL, \
        seen INTEGER NOT NULL);";

    fn project(applied: bool) -> tempfile::TempDir {
        let p = tempfile::tempdir().unwrap();
        let m = p.path().join("migrations/0001_a");
        fs::create_dir_all(&m).unwrap();
        fs::write(m.join("up.sql"), UP).unwrap();
        fs::write(m.join("down.sql"), "SELECT 1;").unwrap();
        fs::write(m.join("metadata.toml"), "run_in_transaction = false\n").unwrap();
        if applied {
            apply_pending(p.path()).unwrap();
        }
        p
    }

    fn dump(p: &Path) -> Vec<(String, String)> {
        #[derive(QueryableByName)]
        struct R {
            #[diesel(sql_type = diesel::sql_types::Text)]
            a: String,
            #[diesel(sql_type = diesel::sql_types::Text)]
            b: String,
        }
        let mut c = SqliteConnection::establish(&p.join(DB_FILE).to_string_lossy()).unwrap();
        diesel::sql_query(
            "SELECT p.name AS a, n.body || n.seen || p.score || COALESCE(p.born,'-') AS b \
             FROM notes n JOIN people p ON p.id = n.person_id ORDER BY n.id",
        )
        .load::<R>(&mut c)
        .unwrap()
        .into_iter()
        .map(|r| (r.a, r.b))
        .collect()
    }

    #[test]
    fn deterministic_and_valid() {
        let (a, b) = (project(true), project(true));
        let added = add_sample_rows(a.path(), &spec(), 3).unwrap();
        add_sample_rows(b.path(), &spec(), 3).unwrap();
        assert_eq!(added["people"], 3);
        assert_eq!(added["notes"], 3);
        assert_eq!(dump(a.path()), dump(b.path()));
        assert_eq!(
            dump(a.path())[0],
            ("name 1".into(), "sample 111.52026-01-01".into())
        );
        // A second run continues numbering, keeping unique names distinct.
        add_sample_rows(a.path(), &spec(), 3).unwrap();
        let mut c = SqliteConnection::establish(&a.path().join(DB_FILE).to_string_lossy()).unwrap();
        assert_eq!(count_rows(&mut c).unwrap()["people"], 6);
    }

    #[test]
    fn refuses_when_pending_or_empty() {
        let p = project(false);
        assert!(matches!(
            add_sample_rows(p.path(), &spec(), 3),
            Err(RuntimeError::Refused(_))
        ));
        let p = project(true);
        let empty = SchemaSpec {
            spec_version: 1,
            tables: vec![],
        };
        assert!(add_sample_rows(p.path(), &empty, 3).is_err());
        let m = p.path().join("migrations/0002_b");
        fs::create_dir_all(&m).unwrap();
        fs::write(m.join("up.sql"), "SELECT 1;").unwrap();
        fs::write(m.join("down.sql"), "SELECT 1;").unwrap();
        assert!(matches!(
            add_sample_rows(p.path(), &spec(), 3),
            Err(RuntimeError::Refused(_))
        ));
    }
}
