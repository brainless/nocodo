//! Random valid change sequences must all pass the dry run
//! (up -> down -> up matches the spec at every step).

use std::fs;

use nocodo_checker::{NamedMigration, check_change_set, dry_run};
use nocodo_sqlgen::{METADATA_TOML, apply, generate};
use nocodo_types::change::{ChangeSet, SchemaChange};
use nocodo_types::schema::{Column, ColumnType, SPEC_VERSION, SchemaSpec};
use proptest::prelude::*;

const TYPES: [ColumnType; 6] = [
    ColumnType::Text,
    ColumnType::Integer,
    ColumnType::Boolean,
    ColumnType::Date,
    ColumnType::DateTime,
    ColumnType::Money,
];

fn column(name: String, t: ColumnType, nullable: bool, unique: bool) -> Column {
    Column {
        description: format!("The {name}."),
        name,
        column_type: t,
        nullable,
        unique,
        references: None,
    }
}

fn change_set(name: String, change: SchemaChange) -> ChangeSet {
    ChangeSet {
        name,
        summary: "Generated.".into(),
        changes: vec![change],
    }
}

/// Turns three random numbers into one change that is valid for `spec`,
/// or `None` if the op does not apply right now.
fn build_change(spec: &SchemaSpec, n: usize, (op, a, b): (u8, u8, u8)) -> Option<SchemaChange> {
    let (a, b) = (a as usize, b as usize);
    let table = spec.tables.get(a % spec.tables.len().max(1));
    match op % 7 {
        0 => {
            let mut cols = vec![column(format!("c{n}"), TYPES[b % 6], b % 2 == 0, false)];
            if let Some(t) = spec.tables.get(b % spec.tables.len().max(1)) {
                let mut fk = column(format!("ref{n}"), ColumnType::Integer, true, false);
                fk.references = Some(t.name.clone());
                cols.push(fk);
            }
            Some(SchemaChange::CreateTable {
                table: format!("t{n}"),
                description: format!("The t{n}."),
                columns: cols,
            })
        }
        1 => {
            let t = table?;
            let mut c = column(format!("a{n}"), TYPES[b % 6], true, b % 5 == 0);
            if c.column_type == ColumnType::Integer && b % 3 == 0 {
                c.references = Some(spec.tables[b % spec.tables.len()].name.clone());
            }
            Some(SchemaChange::AddColumn {
                table: t.name.clone(),
                column: c,
            })
        }
        2 => {
            let t = table?;
            let first = &t.columns.get(b % t.columns.len().max(1))?.name;
            let mut columns = vec![first.clone()];
            if let Some(second) = t.columns.get((b / 7) % t.columns.len())
                && second.name != *first
            {
                columns.push(second.name.clone());
            }
            let unique = b % 4 == 0;
            if t.indexes
                .iter()
                .any(|i| i.columns == columns && i.unique == unique)
            {
                return None;
            }
            Some(SchemaChange::AddIndex {
                table: t.name.clone(),
                columns,
                unique,
            })
        }
        3 => {
            let t = table?;
            let c = t
                .columns
                .iter()
                .filter(|c| c.column_type == ColumnType::Integer && c.references.is_none())
                .nth(b)
                .or_else(|| {
                    t.columns
                        .iter()
                        .find(|c| c.column_type == ColumnType::Integer && c.references.is_none())
                })?;
            Some(SchemaChange::AddForeignKey {
                table: t.name.clone(),
                column: c.name.clone(),
                references: spec.tables[b % spec.tables.len()].name.clone(),
            })
        }
        4 => {
            let t = table?;
            let c = t.columns.get(b % t.columns.len().max(1))?;
            Some(SchemaChange::RenameColumn {
                table: t.name.clone(),
                from: c.name.clone(),
                to: format!("r{n}"),
            })
        }
        5 => Some(SchemaChange::RenameTable {
            from: table?.name.clone(),
            to: format!("rt{n}"),
        }),
        _ => {
            let t = table?;
            if b % 4 == 0 {
                Some(SchemaChange::DropTable {
                    table: t.name.clone(),
                })
            } else {
                let c = t.columns.get(b % t.columns.len().max(1))?;
                Some(SchemaChange::DropColumn {
                    table: t.name.clone(),
                    column: c.name.clone(),
                })
            }
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 64, ..ProptestConfig::default() })]

    #[test]
    fn random_valid_sequences_pass_the_dry_run(
        ops in proptest::collection::vec((any::<u8>(), any::<u8>(), any::<u8>()), 1..10)
    ) {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("migrations");
        fs::create_dir_all(&dir).unwrap();

        // Small starting schema.
        let mut spec = SchemaSpec { spec_version: SPEC_VERSION, tables: vec![] };
        let seed = vec![
            SchemaChange::CreateTable {
                table: "people".into(),
                description: "People.".into(),
                columns: vec![column("name".into(), ColumnType::Text, false, true)],
            },
            SchemaChange::CreateTable {
                table: "payments".into(),
                description: "Payments.".into(),
                columns: vec![
                    column("amount".into(), ColumnType::Money, false, false),
                    column("payer".into(), ColumnType::Integer, true, false),
                ],
            },
        ];
        let mut sets = vec![ChangeSet {
            name: "seed".into(),
            summary: "Seed.".into(),
            changes: seed,
        }];
        let mut planned = apply(&spec, &sets[0].changes).unwrap();
        for (i, op) in ops.iter().enumerate() {
            let Some(change) = build_change(&planned, i, *op) else { continue };
            let c = change_set(format!("step_{i}"), change);
            // Only keep change sets the static rules accept (destructive allowed).
            if !check_change_set(&planned, &c, true).is_empty() {
                continue;
            }
            let Ok(next) = apply(&planned, &c.changes) else { continue };
            planned = next;
            sets.push(c);
        }

        for (i, c) in sets.iter().enumerate() {
            let after = apply(&spec, &c.changes).unwrap();
            let m = generate(&spec, c).unwrap();
            let named = NamedMigration {
                name: format!("{:04}_{}", i + 1, c.name),
                up: m.up,
                down: m.down,
            };
            let findings = dry_run(None, &dir, &named, &spec, &after).unwrap();
            prop_assert!(findings.is_empty(), "{:?}: {:?}", c, findings);
            let d = dir.join(&named.name);
            fs::create_dir_all(&d).unwrap();
            fs::write(d.join("up.sql"), &named.up).unwrap();
            fs::write(d.join("down.sql"), &named.down).unwrap();
            fs::write(d.join("metadata.toml"), METADATA_TOML).unwrap();
            spec = after;
        }
    }
}
