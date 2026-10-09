use nocodo_sqlgen::{ApplyError, Migration, apply, generate};
use nocodo_types::change::{ChangeSet, SchemaChange};
use nocodo_types::schema::{Column, ColumnType, Index, SPEC_VERSION, SchemaSpec, Table};

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

fn base() -> SchemaSpec {
    let mut notes = col("notes", ColumnType::Text);
    notes.nullable = true;
    let mut account = col("account_id", ColumnType::Integer);
    account.references = Some("accounts".into());
    let mut code = col("code", ColumnType::Text);
    code.unique = true;
    SchemaSpec {
        spec_version: SPEC_VERSION,
        tables: vec![
            Table {
                name: "accounts".into(),
                description: "Bank accounts.".into(),
                columns: vec![col("name", ColumnType::Text)],
                indexes: vec![],
            },
            Table {
                name: "people".into(),
                description: "People.".into(),
                columns: vec![col("name", ColumnType::Text)],
                indexes: vec![],
            },
            Table {
                name: "transactions".into(),
                description: "Money movements.".into(),
                columns: vec![
                    account,
                    col("amount", ColumnType::Money),
                    col("owner", ColumnType::Integer),
                    code,
                    notes,
                ],
                indexes: vec![Index {
                    columns: vec!["account_id".into(), "amount".into()],
                    unique: false,
                }],
            },
        ],
    }
}

fn cs(changes: Vec<SchemaChange>) -> ChangeSet {
    ChangeSet {
        name: "test_change".into(),
        summary: "Test.".into(),
        changes,
    }
}

fn snap(changes: Vec<SchemaChange>) -> String {
    let Migration { up, down } = generate(&base(), &cs(changes)).unwrap();
    format!("=== up.sql ===\n{up}\n=== down.sql ===\n{down}")
}

#[test]
fn create_table() {
    let mut person = col("person_id", ColumnType::Integer);
    person.references = Some("people".into());
    person.nullable = true;
    let mut email = col("email", ColumnType::Text);
    email.unique = true;
    insta::assert_snapshot!(snap(vec![SchemaChange::CreateTable {
        table: "loans".into(),
        description: "Money lent.".into(),
        columns: vec![
            person,
            email,
            col("settled", ColumnType::Boolean),
            col("lent_on", ColumnType::Date),
            col("at", ColumnType::DateTime),
        ],
    }]));
}

#[test]
fn add_column() {
    insta::assert_snapshot!(snap(vec![SchemaChange::AddColumn {
        table: "people".into(),
        column: {
            let mut c = col("email", ColumnType::Text);
            c.nullable = true;
            c.unique = true;
            c
        },
    }]));
}

#[test]
fn add_column_with_reference() {
    let mut c = col("person_id", ColumnType::Integer);
    c.nullable = true;
    c.references = Some("people".into());
    insta::assert_snapshot!(snap(vec![SchemaChange::AddColumn {
        table: "transactions".into(),
        column: c
    }]));
}

#[test]
fn add_index() {
    insta::assert_snapshot!(snap(vec![SchemaChange::AddIndex {
        table: "transactions".into(),
        columns: vec!["owner".into()],
        unique: false,
    }]));
}

#[test]
fn add_unique_index() {
    insta::assert_snapshot!(snap(vec![SchemaChange::AddIndex {
        table: "people".into(),
        columns: vec!["name".into()],
        unique: true,
    }]));
}

#[test]
fn add_foreign_key() {
    insta::assert_snapshot!(snap(vec![SchemaChange::AddForeignKey {
        table: "transactions".into(),
        column: "owner".into(),
        references: "people".into(),
    }]));
}

#[test]
fn rename_table() {
    insta::assert_snapshot!(snap(vec![SchemaChange::RenameTable {
        from: "transactions".into(),
        to: "entries".into(),
    }]));
}

#[test]
fn rename_column() {
    insta::assert_snapshot!(snap(vec![SchemaChange::RenameColumn {
        table: "transactions".into(),
        from: "account_id".into(),
        to: "bank_account_id".into(),
    }]));
}

#[test]
fn drop_column() {
    insta::assert_snapshot!(snap(vec![SchemaChange::DropColumn {
        table: "transactions".into(),
        column: "notes".into(),
    }]));
}

#[test]
fn drop_unique_column() {
    insta::assert_snapshot!(snap(vec![SchemaChange::DropColumn {
        table: "transactions".into(),
        column: "code".into(),
    }]));
}

#[test]
fn drop_table() {
    insta::assert_snapshot!(snap(vec![SchemaChange::DropTable {
        table: "transactions".into()
    }]));
}

#[test]
fn multiple_changes_reverse_in_down() {
    insta::assert_snapshot!(snap(vec![
        SchemaChange::RenameColumn {
            table: "people".into(),
            from: "name".into(),
            to: "full_name".into()
        },
        SchemaChange::AddIndex {
            table: "people".into(),
            columns: vec!["full_name".into()],
            unique: false
        },
        SchemaChange::DropTable {
            table: "accounts".into()
        },
    ]));
}

#[test]
fn output_is_byte_stable() {
    let c = cs(vec![SchemaChange::DropColumn {
        table: "transactions".into(),
        column: "notes".into(),
    }]);
    assert_eq!(
        generate(&base(), &c).unwrap(),
        generate(&base(), &c).unwrap()
    );
}

// ---- apply error tests, one per variant ----

fn err(changes: Vec<SchemaChange>) -> ApplyError {
    apply(&base(), &changes).unwrap_err()
}

#[test]
fn unknown_table() {
    let e = err(vec![SchemaChange::DropTable {
        table: "nope".into(),
    }]);
    assert_eq!(
        e,
        ApplyError::UnknownTable {
            table: "nope".into()
        }
    );
    let e = err(vec![SchemaChange::AddForeignKey {
        table: "people".into(),
        column: "name".into(),
        references: "nope".into(),
    }]);
    assert_eq!(
        e,
        ApplyError::UnknownTable {
            table: "nope".into()
        }
    );
}

#[test]
fn unknown_column() {
    let e = err(vec![SchemaChange::DropColumn {
        table: "people".into(),
        column: "nope".into(),
    }]);
    assert_eq!(
        e,
        ApplyError::UnknownColumn {
            table: "people".into(),
            column: "nope".into()
        }
    );
    let e = err(vec![SchemaChange::AddIndex {
        table: "people".into(),
        columns: vec!["nope".into()],
        unique: false,
    }]);
    assert!(matches!(e, ApplyError::UnknownColumn { .. }));
}

#[test]
fn duplicate_table() {
    let e = err(vec![SchemaChange::RenameTable {
        from: "people".into(),
        to: "accounts".into(),
    }]);
    assert_eq!(
        e,
        ApplyError::DuplicateTable {
            table: "accounts".into()
        }
    );
    let e = err(vec![SchemaChange::CreateTable {
        table: "people".into(),
        description: "x".into(),
        columns: vec![],
    }]);
    assert!(matches!(e, ApplyError::DuplicateTable { .. }));
}

#[test]
fn duplicate_column() {
    let e = err(vec![SchemaChange::RenameColumn {
        table: "transactions".into(),
        from: "owner".into(),
        to: "amount".into(),
    }]);
    assert_eq!(
        e,
        ApplyError::DuplicateColumn {
            table: "transactions".into(),
            column: "amount".into()
        }
    );
    let e = err(vec![SchemaChange::AddColumn {
        table: "people".into(),
        column: col("name", ColumnType::Text),
    }]);
    assert!(matches!(e, ApplyError::DuplicateColumn { .. }));
}

#[test]
fn duplicate_index() {
    let e = err(vec![SchemaChange::AddIndex {
        table: "transactions".into(),
        columns: vec!["account_id".into(), "amount".into()],
        unique: false,
    }]);
    assert_eq!(
        e,
        ApplyError::DuplicateIndex {
            table: "transactions".into(),
            index: "idx_transactions_account_id_amount".into()
        }
    );
}

#[test]
fn apply_follows_renames_and_drops() {
    let s = apply(
        &base(),
        &[
            SchemaChange::RenameTable {
                from: "accounts".into(),
                to: "banks".into(),
            },
            SchemaChange::DropColumn {
                table: "transactions".into(),
                column: "amount".into(),
            },
        ],
    )
    .unwrap();
    let t = &s.tables[2];
    assert_eq!(t.columns[0].references.as_deref(), Some("banks"));
    assert!(t.indexes.is_empty(), "index on dropped column is removed");
}

#[test]
fn decimal_column() {
    let mut nullable = col("weight_kg", ColumnType::Decimal);
    nullable.nullable = true;
    insta::assert_snapshot!(snap(vec![
        SchemaChange::CreateTable {
            table: "measurements".into(),
            description: "Measurements.".into(),
            columns: vec![col("score", ColumnType::Decimal)],
        },
        SchemaChange::AddColumn {
            table: "people".into(),
            column: nullable,
        },
    ]));
}
