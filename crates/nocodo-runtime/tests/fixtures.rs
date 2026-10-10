//! Runtime over the real finance fixtures: apply, backups, sample rows, summary.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use diesel::prelude::*;
use diesel::sql_types::{BigInt, Text};
use nocodo_runtime::{add_sample_rows, apply_pending, inspect, summary};
use nocodo_sqlgen::{METADATA_TOML, apply, generate};
use nocodo_types::change::ChangeSet;
use nocodo_types::schema::{SPEC_VERSION, SchemaSpec};

const STEPS: [&str; 3] = ["01-initial", "02-family-member", "03-rebuild"];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn fixture(name: &str) -> ChangeSet {
    let path = root()
        .join("change-sets/finance")
        .join(format!("{name}.json"));
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn empty() -> SchemaSpec {
    SchemaSpec {
        spec_version: SPEC_VERSION,
        tables: vec![],
    }
}

/// Writes migration `n` (1-based) for fixture `name` and returns the new spec.
fn add_step(project: &Path, n: usize, name: &str, before: &SchemaSpec) -> SchemaSpec {
    let c = fixture(name);
    let m = generate(before, &c).unwrap();
    let d = project
        .join("migrations")
        .join(format!("{n:04}_{}", c.name));
    fs::create_dir_all(&d).unwrap();
    fs::write(d.join("up.sql"), m.up).unwrap();
    fs::write(d.join("down.sql"), m.down).unwrap();
    fs::write(d.join("metadata.toml"), METADATA_TOML).unwrap();
    apply(before, &c.changes).unwrap()
}

fn conn(path: &Path) -> SqliteConnection {
    SqliteConnection::establish(&path.to_string_lossy()).unwrap()
}

#[derive(QueryableByName)]
struct N {
    #[diesel(sql_type = BigInt)]
    n: i64,
}

#[derive(QueryableByName)]
struct S {
    #[diesel(sql_type = Text)]
    s: String,
}

fn count(db: &Path, table: &str) -> i64 {
    diesel::sql_query(format!("SELECT COUNT(*) AS n FROM \"{table}\""))
        .get_result::<N>(&mut conn(db))
        .unwrap()
        .n
}

/// Every spec column of every table as text, in id order.
fn contents(db: &Path, spec: &SchemaSpec) -> BTreeMap<String, Vec<String>> {
    let mut out = BTreeMap::new();
    for t in &spec.tables {
        let cols: Vec<String> = std::iter::once("id".to_string())
            .chain(t.columns.iter().map(|c| c.name.clone()))
            .map(|c| format!("COALESCE(CAST(\"{c}\" AS TEXT), 'NULL')"))
            .collect();
        let sql = format!(
            "SELECT {} AS s FROM \"{}\" ORDER BY id",
            cols.join(" || '|' || "),
            t.name
        );
        let rows = diesel::sql_query(sql).load::<S>(&mut conn(db)).unwrap();
        out.insert(t.name.clone(), rows.into_iter().map(|r| r.s).collect());
    }
    out
}

fn fk_violations(db: &Path) -> usize {
    #[derive(QueryableByName)]
    struct V {
        #[diesel(sql_type = Text)]
        #[allow(dead_code)]
        table: String,
    }
    diesel::sql_query("PRAGMA foreign_key_check")
        .load::<V>(&mut conn(db))
        .unwrap()
        .len()
}

fn project_at(step: usize) -> (tempfile::TempDir, SchemaSpec) {
    let p = tempfile::tempdir().unwrap();
    let mut spec = empty();
    for (i, name) in STEPS.iter().take(step).enumerate() {
        spec = add_step(p.path(), i + 1, name, &spec);
    }
    (p, spec)
}

#[test]
fn initial_migration_creates_database_without_backup() {
    let (p, spec) = project_at(1);
    let r = apply_pending(p.path()).unwrap();
    assert_eq!(r.failed, None);
    assert_eq!(r.applied.len(), 1);
    assert_eq!(r.applied[0].backup, None);
    assert!(p.path().join("data.sqlite").exists());
    assert_eq!(fs::read_dir(p.path().join("backups")).unwrap().count(), 0);
    let info = inspect(p.path()).unwrap();
    assert_eq!(info.tables.len(), spec.tables.len());
    assert!(info.tables.values().all(|n| *n == 0));
    assert!(info.pending.is_empty());
}

#[test]
fn inspect_works_when_the_path_has_uri_characters() {
    let base = tempfile::tempdir().unwrap();
    let p = base.path().join("a#b?c%d");
    fs::create_dir_all(&p).unwrap();
    let spec = add_step(&p, 1, STEPS[0], &empty());
    apply_pending(&p).unwrap();
    let info = inspect(&p).unwrap();
    assert_eq!(info.tables.len(), spec.tables.len());
    assert!(info.pending.is_empty());
}

#[test]
fn follow_ups_back_up_and_keep_rows_including_a_rebuild() {
    let p = tempfile::tempdir().unwrap();
    let db = p.path().join("data.sqlite");
    let spec1 = add_step(p.path(), 1, STEPS[0], &empty());
    apply_pending(p.path()).unwrap();
    let added = add_sample_rows(p.path(), &spec1, 3).unwrap();
    assert!(added.values().all(|n| *n == 3));
    let rows_1 = contents(&db, &spec1);

    // 02: additive column + index.
    let spec2 = add_step(p.path(), 2, STEPS[1], &spec1);
    let r = apply_pending(p.path()).unwrap();
    assert_eq!(r.failed, None);
    assert_eq!(r.applied.len(), 1);
    let a = &r.applied[0];
    assert_eq!(a.migration, "0002_add_family_member");
    assert_eq!(
        a.backup.as_deref(),
        Some("backups/before_0002_add_family_member.sqlite")
    );
    assert_eq!(a.rows_before, a.rows_after);
    assert!(a.rows_before.values().all(|n| *n == 3));
    let backup = p.path().join(a.backup.as_ref().unwrap());
    assert_eq!(contents(&backup, &spec1), rows_1);

    // 03: rename column + add foreign key (table rebuild).
    let rows_2 = contents(&db, &spec2);
    let _spec3 = add_step(p.path(), 3, STEPS[2], &spec2);
    let r = apply_pending(p.path()).unwrap();
    assert_eq!(r.failed, None);
    let a = &r.applied[0];
    assert_eq!(a.migration, "0003_clarify_dates_and_transfers");
    assert_eq!(
        a.backup.as_deref(),
        Some("backups/before_0003_clarify_dates_and_transfers.sqlite")
    );
    assert_eq!(a.rows_before, a.rows_after);
    let backup = p.path().join(a.backup.as_ref().unwrap());
    assert_eq!(contents(&backup, &spec2), rows_2);
    assert_eq!(fs::read_dir(p.path().join("backups")).unwrap().count(), 2);

    // The rebuilt table kept its sample values under the new column name.
    let paid_on: Vec<String> =
        diesel::sql_query("SELECT paid_on AS s FROM transactions ORDER BY id")
            .load::<S>(&mut conn(&db))
            .unwrap()
            .into_iter()
            .map(|r| r.s)
            .collect();
    assert_eq!(paid_on, ["2026-01-01", "2026-01-02", "2026-01-03"]);
    let notes: Vec<String> = diesel::sql_query("SELECT notes AS s FROM transactions ORDER BY id")
        .load::<S>(&mut conn(&db))
        .unwrap()
        .into_iter()
        .map(|r| r.s)
        .collect();
    assert_eq!(notes, ["sample 1", "sample 2", "sample 3"]);
    assert_eq!(count(&db, "transactions"), 3);
    assert_eq!(fk_violations(&db), 0);
    assert!(inspect(p.path()).unwrap().pending.is_empty());
}

#[test]
fn sample_rows_are_deterministic_valid_and_unique() {
    let (a, spec) = project_at(3);
    let (b, _) = project_at(3);
    for p in [&a, &b] {
        apply_pending(p.path()).unwrap();
        add_sample_rows(p.path(), &spec, 4).unwrap();
    }
    let (da, db) = (a.path().join("data.sqlite"), b.path().join("data.sqlite"));
    let ca = contents(&da, &spec);
    assert_eq!(ca, contents(&db, &spec));
    assert!(ca.values().all(|rows| rows.len() == 4));
    assert_eq!(fk_violations(&da), 0);
    for t in &spec.tables {
        for (i, c) in t.columns.iter().enumerate().filter(|(_, c)| c.unique) {
            // Column position in the dump is shifted by the leading id.
            let mut seen: Vec<&str> = ca[&t.name]
                .iter()
                .map(|r| r.split('|').nth(i + 1).unwrap())
                .collect();
            seen.sort();
            seen.dedup();
            assert_eq!(seen.len(), 4, "{}.{} not distinct", t.name, c.name);
        }
    }
    // A second run continues the numbering and stays valid.
    add_sample_rows(a.path(), &spec, 2).unwrap();
    assert_eq!(fk_violations(&da), 0);
    assert_eq!(count(&da, &spec.tables[0].name), 6);
}

#[test]
fn sample_rows_refuse_while_a_migration_is_pending() {
    let (p, spec) = project_at(1);
    apply_pending(p.path()).unwrap();
    let spec2 = add_step(p.path(), 2, STEPS[1], &spec);
    assert!(add_sample_rows(p.path(), &spec2, 3).is_err());
    assert_eq!(count(&p.path().join("data.sqlite"), "transactions"), 0);
}

#[test]
fn summary_of_finance_after_family_member() {
    let (_, spec) = project_at(2);
    let rows: BTreeMap<String, u64> = spec
        .tables
        .iter()
        .enumerate()
        .map(|(i, t)| (t.name.clone(), i as u64))
        .collect();
    insta::assert_snapshot!(summary(&spec, &rows));
}

#[test]
fn generated_summary_text_has_no_technical_words() {
    let banned = ["table", "column", "row", "sql", "database", "schema", "id"];
    let mut specs: Vec<SchemaSpec> = (1..=3).map(|n| project_at(n).1).collect();
    let example = root().join("examples/schema.json");
    specs.push(serde_json::from_str(&fs::read_to_string(example).unwrap()).unwrap());
    for mut spec in specs {
        // Descriptions come from the agent and are shown as written.
        for t in &mut spec.tables {
            t.description = String::new();
        }
        let text = summary(&spec, &BTreeMap::new()).to_lowercase();
        for word in text.split(|c: char| !c.is_alphanumeric()) {
            let stem = word.trim_end_matches('s');
            assert!(
                !banned.contains(&word) && !banned.contains(&stem),
                "banned word {word:?} in summary:\n{text}"
            );
        }
    }
}
