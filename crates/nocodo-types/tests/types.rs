use nocodo_types::change::{ChangeSet, SchemaChange};
use nocodo_types::event::{Actor, Event, EventKind, Finding};
use nocodo_types::project::ProjectConfig;
use nocodo_types::schema::{Column, ColumnType, Index, SPEC_VERSION, SchemaSpec, Table};
use serde::{Serialize, de::DeserializeOwned};
use std::fmt::Debug;

fn roundtrip<T: Serialize + DeserializeOwned + PartialEq + Debug>(value: &T) {
    let json = serde_json::to_string(value).unwrap();
    let back: T = serde_json::from_str(&json).unwrap();
    assert_eq!(value, &back, "round trip failed for {json}");
}

fn fixture(name: &str) -> String {
    let path = format!(
        "{}/../../fixtures/examples/{name}",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {path}: {e}"))
}

fn col(name: &str, column_type: ColumnType) -> Column {
    Column {
        name: name.into(),
        description: format!("The {name}."),
        column_type,
        nullable: false,
        unique: false,
        references: None,
    }
}

fn change_set() -> ChangeSet {
    ChangeSet {
        name: "add_things".into(),
        summary: "Adds things.".into(),
        changes: vec![SchemaChange::DropTable {
            table: "old".into(),
        }],
    }
}

fn all_changes() -> Vec<SchemaChange> {
    let mut fk = col("owner_id", ColumnType::Integer);
    fk.references = Some("people".into());
    fk.nullable = true;
    vec![
        SchemaChange::CreateTable {
            table: "things".into(),
            description: "Things.".into(),
            columns: vec![col("name", ColumnType::Text), fk.clone()],
        },
        SchemaChange::AddColumn {
            table: "things".into(),
            column: fk,
        },
        SchemaChange::AddIndex {
            table: "things".into(),
            columns: vec!["name".into()],
            unique: true,
        },
        SchemaChange::AddForeignKey {
            table: "things".into(),
            column: "owner_id".into(),
            references: "people".into(),
        },
        SchemaChange::RenameTable {
            from: "a".into(),
            to: "b".into(),
        },
        SchemaChange::RenameColumn {
            table: "t".into(),
            from: "a".into(),
            to: "b".into(),
        },
        SchemaChange::DropColumn {
            table: "t".into(),
            column: "c".into(),
        },
        SchemaChange::DropTable { table: "t".into() },
    ]
}

#[test]
fn schema_change_variants_roundtrip() {
    for c in all_changes() {
        roundtrip(&c);
    }
}

#[test]
fn schema_change_uses_op_tag() {
    let json = serde_json::to_value(SchemaChange::DropTable { table: "t".into() }).unwrap();
    assert_eq!(json["op"], "drop_table");
}

#[test]
fn destructive_flags() {
    for c in all_changes() {
        let expected = matches!(
            c,
            SchemaChange::DropColumn { .. } | SchemaChange::DropTable { .. }
        );
        assert_eq!(c.is_destructive(), expected, "{c:?}");
    }
    assert!(SchemaChange::DropTable { table: "t".into() }.is_destructive());
    assert!(
        SchemaChange::DropColumn {
            table: "t".into(),
            column: "c".into()
        }
        .is_destructive()
    );
    assert!(change_set().is_destructive());
    let safe = ChangeSet {
        changes: all_changes().into_iter().take(6).collect(),
        ..change_set()
    };
    assert!(!safe.is_destructive());
}

#[test]
fn spec_changeset_project_roundtrip() {
    let spec = SchemaSpec {
        spec_version: SPEC_VERSION,
        tables: vec![Table {
            name: "things".into(),
            description: "Things.".into(),
            columns: vec![
                col("name", ColumnType::Text),
                col("price", ColumnType::Money),
            ],
            indexes: vec![Index {
                columns: vec!["name".into()],
                unique: true,
            }],
        }],
    };
    roundtrip(&spec);
    roundtrip(&ChangeSet {
        changes: all_changes(),
        ..change_set()
    });
    roundtrip(&ProjectConfig {
        spec_version: SPEC_VERSION,
        name: "demo".into(),
    });
    for t in [
        ColumnType::Text,
        ColumnType::Integer,
        ColumnType::Boolean,
        ColumnType::Date,
        ColumnType::DateTime,
        ColumnType::Money,
    ] {
        roundtrip(&t);
    }
}

#[test]
fn event_kinds_roundtrip() {
    let kinds = vec![
        EventKind::ProjectCreated {
            spec_version: 1,
            name: "demo".into(),
        },
        EventKind::ChangeSetProposed {
            change_set: change_set(),
        },
        EventKind::ChangeSetRejected {
            proposal_seq: 2,
            findings: vec![Finding {
                code: "c".into(),
                message: "m".into(),
            }],
        },
        EventKind::ChangeSetAccepted {
            proposal_seq: 2,
            migration: "0002_x".into(),
        },
    ];
    for (i, kind) in kinds.into_iter().enumerate() {
        for actor in [Actor::Developer, Actor::System] {
            roundtrip(&Event {
                seq: i as u64 + 1,
                at: "2026-10-09T09:00:00Z".parse().unwrap(),
                actor,
                kind: kind.clone(),
            });
        }
    }
}

#[test]
fn column_defaults_apply() {
    let c: Column =
        serde_json::from_str(r#"{"name":"n","description":"d","column_type":"text"}"#).unwrap();
    assert!(!c.nullable && !c.unique && c.references.is_none());
}

#[test]
fn fixtures_parse() {
    let spec: SchemaSpec = serde_json::from_str(&fixture("schema.json")).unwrap();
    assert_eq!(spec.spec_version, SPEC_VERSION);
    let cs: ChangeSet = serde_json::from_str(&fixture("change-set.json")).unwrap();
    assert!(!cs.changes.is_empty());
    let events: Vec<Event> = fixture("events.jsonl")
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(events.len(), 5);
    for (i, e) in events.iter().enumerate() {
        assert_eq!(e.seq, i as u64 + 1);
    }
}

#[test]
fn schema_fixture_is_byte_stable() {
    let raw = fixture("schema.json");
    let spec: SchemaSpec = serde_json::from_str(&raw).unwrap();
    let out = serde_json::to_string_pretty(&spec).unwrap() + "\n";
    assert_eq!(raw, out);
}

#[test]
fn unknown_fields_rejected_on_structs() {
    let err = serde_json::from_str::<Column>(
        r#"{"name":"n","description":"d","column_type":"text","nulable":true}"#,
    )
    .unwrap_err();
    assert!(err.to_string().contains("nulable"), "{err}");

    assert!(
        serde_json::from_str::<Table>(
            r#"{"name":"n","description":"d","columns":[],"indexes":[],"extra":1}"#
        )
        .is_err()
    );
    assert!(serde_json::from_str::<Index>(r#"{"columns":[],"unique":false,"extra":1}"#).is_err());
    assert!(
        serde_json::from_str::<SchemaSpec>(r#"{"spec_version":1,"tables":[],"extra":1}"#).is_err()
    );
    assert!(
        serde_json::from_str::<ChangeSet>(r#"{"name":"n","summary":"s","changes":[],"extra":1}"#)
            .is_err()
    );
    assert!(
        serde_json::from_str::<ProjectConfig>(r#"{"spec_version":1,"name":"n","extra":1}"#)
            .is_err()
    );
    assert!(serde_json::from_str::<Finding>(r#"{"code":"c","message":"m","extra":1}"#).is_err());
}

#[test]
fn unknown_field_nested_in_column_inside_change_is_rejected() {
    let json = r#"{"op":"add_column","table":"t","column":
        {"name":"n","description":"d","column_type":"text","nulable":true}}"#;
    assert!(serde_json::from_str::<SchemaChange>(json).is_err());
}

/// Probe: report whether internally tagged enums reject unknown fields.
#[test]
fn tagged_enum_unknown_field_behaviour() {
    let change =
        serde_json::from_str::<SchemaChange>(r#"{"op":"drop_table","table":"t","extra":1}"#);
    let actor = serde_json::from_str::<Actor>(r#"{"type":"developer","extra":1}"#);
    let kind = serde_json::from_str::<EventKind>(
        r#"{"type":"change_set_accepted","proposal_seq":1,"migration":"m","extra":1}"#,
    );
    println!("SchemaChange rejects: {}", change.is_err());
    println!("Actor rejects: {}", actor.is_err());
    println!("EventKind rejects: {}", kind.is_err());
    // SchemaChange carries deny_unknown_fields, so it must reject.
    assert!(change.is_err());
}
