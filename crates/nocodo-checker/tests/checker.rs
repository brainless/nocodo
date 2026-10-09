use std::fs;
use std::path::Path;

use nocodo_checker::{NamedMigration, check_change_set, dry_run};
use nocodo_sqlgen::{apply, generate};
use nocodo_types::change::{ChangeSet, SchemaChange};
use nocodo_types::schema::{Column, ColumnType, SPEC_VERSION, SchemaSpec};

fn col(name: &str, t: ColumnType) -> Column {
    Column {
        name: name.into(),
        description: format!("The {name}."),
        column_type: t,
        nullable: false,
        unique: false,
        references: None,
    }
}

fn empty() -> SchemaSpec {
    SchemaSpec {
        spec_version: SPEC_VERSION,
        tables: vec![],
    }
}

fn create(table: &str, columns: Vec<Column>) -> SchemaChange {
    SchemaChange::CreateTable {
        table: table.into(),
        description: format!("The {table}."),
        columns,
    }
}

fn cs(name: &str, changes: Vec<SchemaChange>) -> ChangeSet {
    ChangeSet {
        name: name.into(),
        summary: "Test.".into(),
        changes,
    }
}

fn base() -> SchemaSpec {
    apply(
        &empty(),
        &[
            create("accounts", vec![col("name", ColumnType::Text)]),
            create(
                "transactions",
                vec![
                    col("amount", ColumnType::Money),
                    col("owner", ColumnType::Integer),
                    col("memo", ColumnType::Text),
                ],
            ),
        ],
    )
    .unwrap()
}

fn codes(spec: &SchemaSpec, c: &ChangeSet, confirm: bool) -> Vec<String> {
    check_change_set(spec, c, confirm)
        .into_iter()
        .map(|f| f.code)
        .collect()
}

fn assert_code(spec: &SchemaSpec, c: &ChangeSet, code: &str) {
    let got = codes(spec, c, false);
    assert!(
        got.iter().any(|g| g == code),
        "expected {code}, got {got:?}"
    );
}

#[test]
fn valid_change_set_passes() {
    let c = cs(
        "add_notes",
        vec![create("notes", vec![col("body", ColumnType::Text)])],
    );
    assert!(check_change_set(&base(), &c, false).is_empty());
}

#[test]
fn empty_change_set() {
    assert_code(&base(), &cs("nothing", vec![]), "empty_change_set");
}

#[test]
fn invalid_name_variants() {
    for bad in ["Accounts", "1abc", "has-dash", "sqlite_thing", "select", ""] {
        let c = cs("x", vec![create(bad, vec![col("name", ColumnType::Text)])]);
        assert_code(&empty(), &c, "invalid_name");
    }
    let long = "a".repeat(64);
    let c = cs("x", vec![create(&long, vec![])]);
    assert_code(&empty(), &c, "invalid_name");
    // Column, rename target and change-set name.
    let c = cs("x", vec![create("t", vec![col("order", ColumnType::Text)])]);
    assert_code(&empty(), &c, "invalid_name");
    let c = cs(
        "x",
        vec![SchemaChange::RenameTable {
            from: "accounts".into(),
            to: "Bad".into(),
        }],
    );
    assert_code(&base(), &c, "invalid_name");
    let c = cs("Bad Name", vec![create("t", vec![])]);
    assert_code(&empty(), &c, "invalid_name");
}

#[test]
fn reserved_column() {
    for name in ["id", "created_at", "updated_at"] {
        let c = cs("x", vec![create("t", vec![col(name, ColumnType::Text)])]);
        assert_code(&empty(), &c, "reserved_column");
    }
}

#[test]
fn missing_description() {
    let mut c1 = col("name", ColumnType::Text);
    c1.description = "  ".into();
    assert_code(
        &empty(),
        &cs("x", vec![create("t", vec![c1])]),
        "missing_description",
    );
    let c = cs(
        "x",
        vec![SchemaChange::CreateTable {
            table: "t".into(),
            description: String::new(),
            columns: vec![],
        }],
    );
    assert_code(&empty(), &c, "missing_description");
}

#[test]
fn apply_errors_are_mapped() {
    let drop = |t: &str| SchemaChange::DropTable { table: t.into() };
    assert_code(&base(), &cs("x", vec![drop("nope")]), "unknown_table");
    let c = cs(
        "x",
        vec![SchemaChange::DropColumn {
            table: "accounts".into(),
            column: "nope".into(),
        }],
    );
    assert_code(&base(), &c, "unknown_column");
    let c = cs("x", vec![create("accounts", vec![])]);
    assert_code(&base(), &c, "duplicate_name");
}

#[test]
fn bad_reference() {
    let mut r = col("ghost_id", ColumnType::Integer);
    r.references = Some("ghosts".into());
    assert_code(
        &base(),
        &cs("x", vec![create("t", vec![r])]),
        "bad_reference",
    );

    let mut r = col("acc", ColumnType::Text);
    r.references = Some("accounts".into());
    assert_code(
        &base(),
        &cs("x", vec![create("t", vec![r])]),
        "bad_reference",
    );

    let fk = |column: &str| SchemaChange::AddForeignKey {
        table: "transactions".into(),
        column: column.into(),
        references: "accounts".into(),
    };
    assert_code(&base(), &cs("x", vec![fk("memo")]), "bad_reference");
    assert!(codes(&base(), &cs("x", vec![fk("owner")]), false).is_empty());
    let c = cs(
        "x",
        vec![SchemaChange::AddForeignKey {
            table: "transactions".into(),
            column: "owner".into(),
            references: "ghosts".into(),
        }],
    );
    assert_code(&base(), &c, "bad_reference");
}

#[test]
fn added_column_not_nullable() {
    let add = |table: &str, nullable: bool| {
        let mut c = col("extra", ColumnType::Text);
        c.nullable = nullable;
        SchemaChange::AddColumn {
            table: table.into(),
            column: c,
        }
    };
    assert_code(
        &base(),
        &cs("x", vec![add("accounts", false)]),
        "added_column_not_nullable",
    );
    assert!(codes(&base(), &cs("x", vec![add("accounts", true)]), false).is_empty());
    // A table created in the same change set has no rows yet.
    let c = cs("x", vec![create("t", vec![]), add("t", false)]);
    assert!(codes(&base(), &c, false).is_empty());
}

#[test]
fn destructive_not_confirmed() {
    let c = cs(
        "x",
        vec![SchemaChange::DropColumn {
            table: "transactions".into(),
            column: "memo".into(),
        }],
    );
    assert_code(&base(), &c, "destructive_not_confirmed");
    assert!(codes(&base(), &c, true).is_empty());
}

// ---- dry run ----

fn write_migration(dir: &Path, m: &NamedMigration) {
    let d = dir.join(&m.name);
    fs::create_dir_all(&d).unwrap();
    fs::write(d.join("up.sql"), &m.up).unwrap();
    fs::write(d.join("down.sql"), &m.down).unwrap();
    fs::write(d.join("metadata.toml"), nocodo_sqlgen::METADATA_TOML).unwrap();
}

fn named(n: usize, c: &ChangeSet, before: &SchemaSpec) -> NamedMigration {
    let m = generate(before, c).unwrap();
    NamedMigration {
        name: format!("{n:04}_{}", c.name),
        up: m.up,
        down: m.down,
    }
}

#[test]
fn dry_run_passes_for_generated_sequence() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("migrations");
    fs::create_dir_all(&dir).unwrap();

    let mut fk = col("account_id", ColumnType::Integer);
    fk.references = Some("accounts".into());
    let mut nullable = col("notes", ColumnType::Text);
    nullable.nullable = true;
    let mut uniq = col("code", ColumnType::Text);
    uniq.unique = true;
    uniq.nullable = true;

    let sets = [
        cs(
            "initial",
            vec![
                create("accounts", vec![col("name", ColumnType::Text)]),
                create(
                    "transactions",
                    vec![
                        fk,
                        col("owner", ColumnType::Integer),
                        col("amount", ColumnType::Money),
                    ],
                ),
            ],
        ),
        cs(
            "extras",
            vec![
                SchemaChange::AddColumn {
                    table: "transactions".into(),
                    column: nullable,
                },
                SchemaChange::AddColumn {
                    table: "accounts".into(),
                    column: uniq,
                },
                SchemaChange::AddIndex {
                    table: "transactions".into(),
                    columns: vec!["account_id".into(), "amount".into()],
                    unique: false,
                },
            ],
        ),
        cs(
            "rebuild",
            vec![
                SchemaChange::RenameColumn {
                    table: "transactions".into(),
                    from: "owner".into(),
                    to: "person_id".into(),
                },
                create("people", vec![col("name", ColumnType::Text)]),
                SchemaChange::AddForeignKey {
                    table: "transactions".into(),
                    column: "person_id".into(),
                    references: "people".into(),
                },
                SchemaChange::DropColumn {
                    table: "transactions".into(),
                    column: "notes".into(),
                },
            ],
        ),
    ];

    let mut spec = empty();
    for (i, c) in sets.iter().enumerate() {
        let after = apply(&spec, &c.changes).unwrap();
        let m = named(i + 1, c, &spec);
        let findings = dry_run(None, &dir, &m, &spec, &after).unwrap();
        assert!(findings.is_empty(), "{}: {findings:?}", c.name);
        write_migration(&dir, &m);
        spec = after;
    }
}

fn setup_one() -> (tempfile::TempDir, SchemaSpec, SchemaSpec, NamedMigration) {
    let tmp = tempfile::tempdir().unwrap();
    fs::create_dir_all(tmp.path().join("migrations")).unwrap();
    let c = cs(
        "initial",
        vec![create("accounts", vec![col("name", ColumnType::Text)])],
    );
    let after = apply(&empty(), &c.changes).unwrap();
    let m = named(1, &c, &empty());
    (tmp, empty(), after, m)
}

fn run(tmp: &tempfile::TempDir, m: &NamedMigration, b: &SchemaSpec, a: &SchemaSpec) -> Vec<String> {
    dry_run(None, &tmp.path().join("migrations"), m, b, a)
        .unwrap()
        .into_iter()
        .map(|f| f.code)
        .collect()
}

#[test]
fn dry_run_sql_error() {
    let (tmp, before, after, mut m) = setup_one();
    m.up = "PRAGMA foreign_keys = OFF;\nBEGIN;\nCREATE TABL oops;\nCOMMIT;\nPRAGMA foreign_keys = ON;\n".into();
    assert_eq!(run(&tmp, &m, &before, &after), ["sql_error"]);
}

#[test]
fn dry_run_not_reversible() {
    let (tmp, before, after, mut m) = setup_one();
    m.down = "-- forgot to drop the table\nSELECT 1;\n".into();
    assert_eq!(run(&tmp, &m, &before, &after), ["not_reversible"]);
}

#[test]
fn dry_run_spec_mismatch() {
    let (tmp, before, mut after, m) = setup_one();
    after.tables[0].columns[0].column_type = ColumnType::Integer;
    assert_eq!(run(&tmp, &m, &before, &after), ["db_does_not_match_spec"]);
}

#[test]
fn dry_run_detects_drift_from_earlier_migrations() {
    let (tmp, _, after, m) = setup_one();
    // Claim the DB should already hold a table that earlier migrations never made.
    let wrong_before = after.clone();
    assert_eq!(
        run(&tmp, &m, &wrong_before, &after),
        ["db_does_not_match_spec"]
    );
}

#[test]
fn dry_run_foreign_key_violation() {
    let (tmp, before, after, m) = setup_one();
    // A migration that leaves a row pointing at a missing parent.
    assert!(run(&tmp, &m, &before, &after).is_empty());

    let mut fk = col("parent_id", ColumnType::Integer);
    fk.references = Some("accounts".into());
    let c = cs("kids", vec![create("kids", vec![fk])]);
    let after2 = apply(&after, &c.changes).unwrap();
    let mut m2 = named(2, &c, &after);
    m2.up = m2.up.replace(
        "COMMIT;",
        "INSERT INTO kids (parent_id) VALUES (99);\nCOMMIT;",
    );
    m2.down = nocodo_sqlgen::generate(&after, &c).unwrap().down;
    write_migration(&tmp.path().join("migrations"), &m);
    assert_eq!(run(&tmp, &m2, &after, &after2), ["foreign_key_violation"]);
}
