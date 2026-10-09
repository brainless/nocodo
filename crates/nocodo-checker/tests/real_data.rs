//! Dry run against a `data.sqlite` that holds rows.

use std::fs;
use std::path::{Path, PathBuf};

use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel_migrations::{FileBasedMigrations, MigrationHarness};
use nocodo_checker::{NamedMigration, check_db_matches_spec, dry_run, table_renames};
use nocodo_sqlgen::{METADATA_TOML, apply, generate};
use nocodo_types::change::ChangeSet;
use nocodo_types::schema::{SPEC_VERSION, SchemaSpec};

fn fixture(name: &str) -> ChangeSet {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/change-sets/finance")
        .join(name);
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn write(dir: &Path, m: &NamedMigration) {
    let d = dir.join(&m.name);
    fs::create_dir_all(&d).unwrap();
    fs::write(d.join("up.sql"), &m.up).unwrap();
    fs::write(d.join("down.sql"), &m.down).unwrap();
    fs::write(d.join("metadata.toml"), METADATA_TOML).unwrap();
}

fn named(n: usize, c: &ChangeSet, before: &SchemaSpec) -> NamedMigration {
    let m = generate(before, c).unwrap();
    NamedMigration {
        name: format!("{n:04}_{}", c.name),
        up: m.up,
        down: m.down,
        table_renames: table_renames(&c.changes),
    }
}

/// Applies `01` and `02` for real, puts a row in every user table, and
/// returns `(dir, spec after 02)`.
fn project_with_rows() -> (tempfile::TempDir, SchemaSpec) {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("migrations");
    fs::create_dir_all(&dir).unwrap();
    let mut spec = SchemaSpec {
        spec_version: SPEC_VERSION,
        tables: vec![],
    };
    for (i, f) in ["01-initial.json", "02-family-member.json"]
        .iter()
        .enumerate()
    {
        let c = fixture(f);
        let m = named(i + 1, &c, &spec);
        write(&dir, &m);
        spec = apply(&spec, &c.changes).unwrap();
    }
    let db = tmp.path().join("data.sqlite");
    let mut conn = SqliteConnection::establish(&db.to_string_lossy()).unwrap();
    conn.run_pending_migrations(FileBasedMigrations::from_path(&dir).unwrap())
        .unwrap();
    // One row per table, every column set to 1 (SQLite accepts it for any type).
    conn.batch_execute("PRAGMA foreign_keys = OFF").unwrap();
    for t in &spec.tables {
        let cols: Vec<_> = t
            .columns
            .iter()
            .map(|c| format!("\"{}\"", c.name))
            .collect();
        let vals: Vec<_> = t.columns.iter().map(|_| "1".to_string()).collect();
        let sql = format!(
            "INSERT INTO \"{}\" ({}) VALUES ({})",
            t.name,
            cols.join(", "),
            vals.join(", ")
        );
        conn.batch_execute(&sql).unwrap();
    }
    (tmp, spec)
}

#[test]
fn dry_run_on_real_rows_reports_the_copy_and_keeps_rows() {
    let (tmp, spec) = project_with_rows();
    let dir = tmp.path().join("migrations");
    let db = tmp.path().join("data.sqlite");
    let before_bytes = fs::read(&db).unwrap();
    let tables = spec.tables.len();

    let c = fixture("03-rebuild.json");
    let m = named(3, &c, &spec);
    let after = apply(&spec, &c.changes).unwrap();
    let r = dry_run(Some(&db), &dir, &m, &spec, &after).unwrap();
    assert!(r.findings.is_empty(), "{:?}", r.findings);
    let first = &r.steps[0];
    assert_eq!(first.check, "earlier_migrations_applied");
    assert!(
        first.detail.starts_with("On a copy of data.sqlite (")
            && first.detail.contains(&format!(
                "in {tables} lists): 2 earlier migrations already applied; ran 0 pending."
            )),
        "{}",
        first.detail
    );
    let up = r.steps.iter().position(|s| s.check == "up").unwrap();
    assert_eq!(r.steps[up + 1].check, "rows_kept");
    assert!(r.steps[up + 1].passed, "{}", r.steps[up + 1].detail);
    assert_eq!(fs::read(&db).unwrap(), before_bytes);
}

#[test]
fn migration_that_deletes_rows_fails_rows_kept() {
    let (tmp, spec) = project_with_rows();
    let dir = tmp.path().join("migrations");
    let db = tmp.path().join("data.sqlite");
    let before_bytes = fs::read(&db).unwrap();
    // A table nothing points at, so the delete itself is allowed.
    let table = &spec
        .tables
        .iter()
        .find(|t| {
            !spec
                .tables
                .iter()
                .flat_map(|o| &o.columns)
                .any(|c| c.references.as_deref() == Some(t.name.as_str()))
        })
        .unwrap()
        .name;

    let m = NamedMigration {
        name: "0003_wipe".into(),
        up: format!("DELETE FROM \"{table}\";"),
        down: String::new(),
        table_renames: vec![],
    };
    let r = dry_run(Some(&db), &dir, &m, &spec, &spec).unwrap();
    assert_eq!(r.findings.len(), 1);
    assert_eq!(r.findings[0].code, "rows_lost", "{:?}", r.findings);
    assert!(r.findings[0].message.contains(table));
    assert!(
        r.findings[0]
            .message
            .contains("1 row before and has 0 rows after")
    );
    assert_eq!(r.steps.last().unwrap().check, "rows_kept");
    assert!(!r.steps.last().unwrap().passed);
    assert_eq!(fs::read(&db).unwrap(), before_bytes);
}

#[test]
fn renamed_table_is_compared_under_its_new_name() {
    let (tmp, spec) = project_with_rows();
    let dir = tmp.path().join("migrations");
    let db = tmp.path().join("data.sqlite");
    let before_bytes = fs::read(&db).unwrap();

    let c: ChangeSet = serde_json::from_str(
        r#"{"name":"rename_transactions","summary":"Calls transactions something else.",
            "changes":[{"op":"rename_table","from":"transactions","to":"payments"}]}"#,
    )
    .unwrap();
    assert_eq!(
        table_renames(&c.changes),
        vec![("transactions".to_string(), "payments".to_string())]
    );
    let m = named(3, &c, &spec);
    let after = apply(&spec, &c.changes).unwrap();
    let r = dry_run(Some(&db), &dir, &m, &spec, &after).unwrap();
    assert!(r.findings.is_empty(), "{:?}", r.findings);
    let step = r.steps.iter().find(|s| s.check == "rows_kept").unwrap();
    assert!(step.passed, "{}", step.detail);
    assert!(
        step.detail
            .contains(&format!("{} lists compared", spec.tables.len())),
        "{}",
        step.detail
    );

    // Losing the renamed table's row is still caught, under the new name.
    let bad = NamedMigration {
        name: "0003_rename_and_wipe".into(),
        up: "ALTER TABLE \"transactions\" RENAME TO \"payments\";\nDELETE FROM \"payments\";"
            .into(),
        down: String::new(),
        table_renames: table_renames(&c.changes),
    };
    let r = dry_run(Some(&db), &dir, &bad, &spec, &after).unwrap();
    let lost: Vec<_> = r
        .findings
        .iter()
        .filter(|f| f.code == "rows_lost")
        .collect();
    assert_eq!(lost.len(), 1, "{:?}", r.findings);
    assert!(lost[0].message.contains("payments"));
    assert_eq!(fs::read(&db).unwrap(), before_bytes);
}

#[test]
fn check_db_matches_spec_passes_then_fails_after_out_of_band_drop() {
    let (tmp, spec) = project_with_rows();
    let db = tmp.path().join("data.sqlite");

    let step = check_db_matches_spec(&db, &spec);
    assert!(step.passed, "{}", step.detail);
    assert_eq!(step.check, "db_matches_spec_after_apply");
    assert_eq!(
        step.detail,
        "data.sqlite matches the current schema, and every row that points at another row points at one that exists."
    );

    // A plain column nothing depends on.
    {
        let mut conn = SqliteConnection::establish(&db.to_string_lossy()).unwrap();
        conn.batch_execute("ALTER TABLE \"transactions\" DROP COLUMN \"notes\"")
            .unwrap();
    }
    let step = check_db_matches_spec(&db, &spec);
    assert!(!step.passed);
    assert!(step.detail.contains("notes"), "{}", step.detail);
}

#[test]
fn check_db_matches_spec_reports_broken_references() {
    let (tmp, spec) = project_with_rows();
    let db = tmp.path().join("data.sqlite");
    {
        let mut conn = SqliteConnection::establish(&db.to_string_lossy()).unwrap();
        conn.batch_execute(
            "PRAGMA foreign_keys = OFF; UPDATE \"transactions\" SET account_id = 99",
        )
        .unwrap();
    }
    let step = check_db_matches_spec(&db, &spec);
    assert!(!step.passed);
    assert!(step.detail.contains("transactions"), "{}", step.detail);
}
