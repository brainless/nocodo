use std::fs;
use std::path::Path;

use nocodo_orchestrator::{OrchestratorError, Outcome, Project};
use nocodo_types::change::{ChangeSet, SchemaChange};
use nocodo_types::event::{Actor, EventKind};
use nocodo_types::schema::{Column, ColumnType};

fn col(name: &str, t: ColumnType) -> Column {
    Column {
        name: name.into(),
        description: format!("The {name}."),
        column_type: t,
        nullable: true,
        unique: false,
        references: None,
    }
}

fn cs(name: &str, changes: Vec<SchemaChange>) -> ChangeSet {
    ChangeSet {
        name: name.into(),
        summary: "Test.".into(),
        changes,
    }
}

fn initial() -> ChangeSet {
    cs(
        "initial",
        vec![SchemaChange::CreateTable {
            table: "expenses".into(),
            description: "Money spent.".into(),
            columns: vec![
                col("amount", ColumnType::Money),
                col("notes", ColumnType::Text),
            ],
        }],
    )
}

fn add_note() -> ChangeSet {
    cs(
        "add_payee",
        vec![SchemaChange::AddColumn {
            table: "expenses".into(),
            column: col("payee", ColumnType::Text),
        }],
    )
}

fn drop_notes() -> ChangeSet {
    cs(
        "drop_notes",
        vec![SchemaChange::DropColumn {
            table: "expenses".into(),
            column: "notes".into(),
        }],
    )
}

fn snapshot(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut out = vec![];
    let mut stack = vec![dir.join("migrations")];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(d).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                stack.push(p);
            } else {
                out.push((
                    p.strip_prefix(dir).unwrap().display().to_string(),
                    fs::read(&p).unwrap(),
                ));
            }
        }
    }
    out.push((
        "schema".into(),
        fs::read(dir.join("specs/schema.json")).unwrap(),
    ));
    out.sort();
    out
}

fn run_sequence(dir: &Path) -> Project {
    let p = Project::init(dir, "t").unwrap();
    for c in [initial(), add_note()] {
        assert!(matches!(
            p.propose(c, Actor::Developer, false).unwrap(),
            Outcome::Accepted { .. }
        ));
    }
    p
}

#[test]
fn init_creates_layout_and_refuses_non_empty() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("p");
    let p = Project::init(&dir, "demo").unwrap();
    for f in [
        "project.json",
        "specs/schema.json",
        "events.jsonl",
        "migrations",
        "inputs",
    ] {
        assert!(dir.join(f).exists(), "{f}");
    }
    let events = p.events().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].seq, 1);
    assert_eq!(events[0].actor, Actor::System);
    assert_eq!(Project::open(&dir).unwrap().config().name, "demo");
    assert!(matches!(
        Project::init(&dir, "demo"),
        Err(OrchestratorError::DirectoryNotEmpty(_))
    ));
}

#[test]
fn accept_writes_migrations_and_schema() {
    let tmp = tempfile::tempdir().unwrap();
    let p = run_sequence(&tmp.path().join("p"));
    for m in ["0001_initial", "0002_add_payee"] {
        for f in ["up.sql", "down.sql", "metadata.toml"] {
            assert!(p.dir().join("migrations").join(m).join(f).is_file());
        }
    }
    assert_eq!(p.current_schema().unwrap().tables[0].columns.len(), 3);
    let text = fs::read_to_string(p.dir().join("specs/schema.json")).unwrap();
    assert!(text.ends_with("}\n"));
    // project.json, proposed+accepted per change set
    assert_eq!(p.events().unwrap().len(), 5);
}

#[test]
fn destructive_needs_confirmation_and_rejection_writes_no_files() {
    let tmp = tempfile::tempdir().unwrap();
    let p = run_sequence(&tmp.path().join("p"));
    let before = snapshot(p.dir());
    let Outcome::Rejected { findings } = p.propose(drop_notes(), Actor::Developer, false).unwrap()
    else {
        panic!("expected rejection");
    };
    assert_eq!(findings[0].code, "destructive_not_confirmed");
    assert_eq!(before, snapshot(p.dir()));
    let events = p.events().unwrap();
    assert!(matches!(
        events.last().unwrap().kind,
        EventKind::ChangeSetRejected { .. }
    ));
    assert!(matches!(
        p.propose(drop_notes(), Actor::Developer, true).unwrap(),
        Outcome::Accepted { migration } if migration == "0003_drop_notes"
    ));
}

#[test]
fn same_changes_give_identical_output() {
    let tmp = tempfile::tempdir().unwrap();
    let a = run_sequence(&tmp.path().join("a"));
    let b = run_sequence(&tmp.path().join("b"));
    assert_eq!(snapshot(a.dir()), snapshot(b.dir()));
}

#[test]
fn hand_edited_schema_is_detected() {
    let tmp = tempfile::tempdir().unwrap();
    let p = run_sequence(&tmp.path().join("p"));
    let path = p.dir().join("specs/schema.json");
    let text = fs::read_to_string(&path).unwrap().replace("payee", "payer");
    fs::write(&path, text).unwrap();
    assert!(matches!(
        p.current_schema(),
        Err(OrchestratorError::SchemaEditedByHand)
    ));
    assert!(p.propose(add_note(), Actor::Developer, false).is_err());
}

// ---- real finance fixtures ----

fn fixture(name: &str) -> ChangeSet {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/change-sets/finance")
        .join(name);
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

const FINANCE: [&str; 3] = [
    "01-initial.json",
    "02-family-member.json",
    "03-rebuild.json",
];

fn run_finance(dir: &Path) -> Project {
    let p = Project::init(dir, "finance").unwrap();
    for f in FINANCE {
        let out = p.propose(fixture(f), Actor::Developer, false).unwrap();
        assert!(matches!(out, Outcome::Accepted { .. }), "{f}: {out:?}");
    }
    p
}

#[test]
fn finance_fixtures_01_to_03_then_04_rejected_then_confirmed() {
    let tmp = tempfile::tempdir().unwrap();
    let p = run_finance(&tmp.path().join("p"));
    let names: Vec<_> = fs::read_dir(p.dir().join("migrations"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names.len(), 3);
    assert!(
        names
            .iter()
            .any(|n| n == "0003_clarify_dates_and_transfers")
    );

    let before = snapshot(p.dir());
    let Outcome::Rejected { findings } = p
        .propose(fixture("04-drop-notes.json"), Actor::Developer, false)
        .unwrap()
    else {
        panic!("04 must be rejected without confirmation");
    };
    assert!(
        findings
            .iter()
            .any(|f| f.code == "destructive_not_confirmed")
    );
    assert_eq!(before, snapshot(p.dir()));

    let Outcome::Accepted { migration } = p
        .propose(fixture("04-drop-notes.json"), Actor::Developer, true)
        .unwrap()
    else {
        panic!("04 must be accepted with confirmation");
    };
    assert_eq!(migration, "0004_remove_payment_notes");
    let schema = p.current_schema().unwrap();
    let tx = schema
        .tables
        .iter()
        .find(|t| t.name == "transactions")
        .unwrap();
    assert!(tx.columns.iter().all(|c| c.name != "notes"));
}

#[test]
fn finance_fixtures_are_deterministic_across_projects() {
    let tmp = tempfile::tempdir().unwrap();
    let a = run_finance(&tmp.path().join("a"));
    let b = run_finance(&tmp.path().join("b"));
    for p in [&a, &b] {
        p.propose(fixture("04-drop-notes.json"), Actor::Developer, true)
            .unwrap();
    }
    let (sa, sb) = (snapshot(a.dir()), snapshot(b.dir()));
    assert!(sa.len() > 10);
    assert_eq!(sa, sb);
}

#[test]
fn finance_hand_edited_schema_is_detected() {
    let tmp = tempfile::tempdir().unwrap();
    let p = run_finance(&tmp.path().join("p"));
    let path = p.dir().join("specs/schema.json");
    let text = fs::read_to_string(&path)
        .unwrap()
        .replace("paid_on", "paid_at");
    fs::write(&path, text).unwrap();
    assert!(matches!(
        p.current_schema(),
        Err(OrchestratorError::SchemaEditedByHand)
    ));
}
