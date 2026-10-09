//! Check the real `data.sqlite` against the current spec after an apply run.

use std::path::Path;

use diesel::prelude::*;
use diesel::sql_types::Text;
use nocodo_types::event::CheckStep;
use nocodo_types::schema::SchemaSpec;

use crate::introspect::{diff, expected_shape, introspect};

#[derive(QueryableByName)]
struct FkViolation {
    #[diesel(sql_type = Text)]
    table: String,
}

const CODE: &str = "db_matches_spec_after_apply";

fn failed(detail: String) -> CheckStep {
    CheckStep {
        check: CODE.into(),
        passed: false,
        detail,
    }
}

/// Percent-encode the characters that matter in a SQLite `file:` URI.
fn file_uri(path: &Path) -> String {
    let mut out = String::from("file:");
    for c in path.to_string_lossy().chars() {
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

/// Open `db` read-only, compare its structure with `spec` and run
/// `PRAGMA foreign_key_check`. Never writes to `db`.
pub fn check_db_matches_spec(db: &Path, spec: &SchemaSpec) -> CheckStep {
    let mut conn = match SqliteConnection::establish(&file_uri(db)) {
        Ok(c) => c,
        Err(e) => return failed(format!("data.sqlite could not be opened for checking: {e}")),
    };
    let shape = match introspect(&mut conn) {
        Ok(s) => s,
        Err(e) => {
            return failed(format!(
                "SQLite error while reading the structure of data.sqlite: {e}"
            ));
        }
    };
    let problems = diff(&expected_shape(spec), &shape);
    if !problems.is_empty() {
        return failed(format!(
            "data.sqlite does not match the current schema: {}",
            problems.join("; ")
        ));
    }
    match diesel::sql_query("PRAGMA foreign_key_check").load::<FkViolation>(&mut conn) {
        Ok(rows) if !rows.is_empty() => {
            let mut tables: Vec<String> = rows.into_iter().map(|r| r.table).collect();
            tables.sort();
            tables.dedup();
            failed(format!(
                "data.sqlite matches the current schema, but rows in {} point at rows that do not exist",
                tables.join(", ")
            ))
        }
        Ok(_) => CheckStep {
            check: CODE.into(),
            passed: true,
            detail: "data.sqlite matches the current schema, and every row that points at another row points at one that exists.".into(),
        },
        Err(e) => failed(format!("SQLite error while checking foreign keys: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nocodo_types::schema::SPEC_VERSION;

    #[test]
    fn empty_db_matches_empty_spec_and_is_not_written() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("data.sqlite");
        {
            let mut c = SqliteConnection::establish(&db.to_string_lossy()).unwrap();
            diesel::sql_query("CREATE TABLE extra (id INTEGER)")
                .execute(&mut c)
                .unwrap();
        }
        let before = std::fs::read(&db).unwrap();
        let spec = SchemaSpec {
            spec_version: SPEC_VERSION,
            tables: vec![],
        };
        let step = check_db_matches_spec(&db, &spec);
        assert!(!step.passed);
        assert_eq!(step.check, CODE);
        assert!(step.detail.contains("extra"), "{}", step.detail);
        assert_eq!(std::fs::read(&db).unwrap(), before);
    }
}
