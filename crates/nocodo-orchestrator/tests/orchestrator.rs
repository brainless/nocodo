use std::fs;
use std::path::Path;

use nocodo_orchestrator::{OrchestratorError, Outcome, Project};
use nocodo_types::change::{ChangeSet, SchemaChange};
use nocodo_types::event::{Actor, EventKind};
use nocodo_types::project::ProjectBase;
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
fn init_copies_base_settings_into_project_json() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("p");
    let mut base = ProjectBase::default();
    base.models.schema.model = "claude-haiku-5-5".into();
    Project::init_with_base(&dir, "demo", &base).unwrap();
    let config = Project::open(&dir).unwrap().config().clone();
    assert_eq!(config.name, "demo");
    assert_eq!(config.models, base.models);
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
    let Outcome::Rejected { findings, .. } =
        p.propose(drop_notes(), Actor::Developer, false).unwrap()
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
        Outcome::Accepted { migration, .. } if migration == "0003_drop_notes"
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
    let Outcome::Rejected { findings, .. } = p
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

    let Outcome::Accepted { migration, .. } = p
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

mod run_prompt {
    use std::sync::Mutex;

    use nocodo_orchestrator::{Feedback, Proposal, Proposer, ProposerError};
    use nocodo_types::schema::SchemaSpec;

    use super::*;

    struct Scripted {
        queue: Mutex<Vec<ChangeSet>>,
        feedback_seen: Mutex<Vec<bool>>,
    }

    impl Scripted {
        fn new(items: Vec<ChangeSet>) -> Self {
            Scripted {
                queue: Mutex::new(items),
                feedback_seen: Mutex::new(vec![]),
            }
        }
    }

    impl Proposer for Scripted {
        async fn propose(
            &self,
            _current: &SchemaSpec,
            _prompt: &str,
            feedback: Option<&Feedback>,
        ) -> Result<Proposal, ProposerError> {
            self.feedback_seen.lock().unwrap().push(feedback.is_some());
            let change_set = self.queue.lock().unwrap().remove(0);
            Ok(Proposal {
                change_set,
                assumptions: vec!["Money is in one currency.".into()],
                actor: Actor::SchemaAgent {
                    run_id: "run-1".into(),
                    model: "test:model".into(),
                },
            })
        }
    }

    fn invalid() -> ChangeSet {
        cs(
            "bad",
            vec![SchemaChange::AddColumn {
                table: "missing".into(),
                column: col("x", ColumnType::Text),
            }],
        )
    }

    #[tokio::test]
    async fn repairs_on_second_attempt() {
        let dir = tempfile::tempdir().unwrap();
        let project = Project::init(dir.path().join("p"), "p").unwrap();
        let proposer = Scripted::new(vec![invalid(), initial()]);
        let r = project
            .run_prompt("track spending", &proposer, false, 3)
            .await
            .unwrap();
        assert!(matches!(r.outcome, Outcome::Accepted { .. }));
        assert_eq!(r.attempts, 2);
        assert_eq!(*proposer.feedback_seen.lock().unwrap(), vec![false, true]);
        let events = project.events().unwrap();
        let proposed: Vec<_> = events
            .iter()
            .filter_map(|e| match &e.kind {
                EventKind::ChangeSetProposed {
                    prompt_seq,
                    attempt,
                    ..
                } => Some((*prompt_seq, *attempt)),
                _ => None,
            })
            .collect();
        assert_eq!(proposed, vec![(Some(2), 1), (Some(2), 2)]);
        assert!(matches!(events[1].kind, EventKind::UserPrompt { .. }));
        assert_eq!(events[1].actor, Actor::User);
        assert_eq!(
            fs::read_dir(project.dir().join("migrations"))
                .unwrap()
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn stops_at_max_attempts_without_files() {
        let dir = tempfile::tempdir().unwrap();
        let project = Project::init(dir.path().join("p"), "p").unwrap();
        let proposer = Scripted::new(vec![invalid(), invalid(), invalid(), initial()]);
        let r = project.run_prompt("x", &proposer, false, 3).await.unwrap();
        assert!(matches!(r.outcome, Outcome::Rejected { .. }));
        assert_eq!(r.attempts, 3);
        assert_eq!(
            fs::read_dir(project.dir().join("migrations"))
                .unwrap()
                .count(),
            0
        );
        assert_eq!(project.current_schema().unwrap().tables.len(), 0);
    }

    #[tokio::test]
    async fn destructive_without_confirmation_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let project = Project::init(dir.path().join("p"), "p").unwrap();
        project.propose(initial(), Actor::Developer, false).unwrap();
        let proposer = Scripted::new(vec![drop_notes()]);
        let r = project.run_prompt("x", &proposer, false, 1).await.unwrap();
        assert!(matches!(r.outcome, Outcome::Rejected { .. }));
        assert_eq!(
            fs::read_dir(project.dir().join("migrations"))
                .unwrap()
                .count(),
            1
        );
    }
}

const ALL_STEPS: [&str; 10] = [
    "static_rules",
    "earlier_migrations_applied",
    "db_matches_spec_before",
    "up",
    "db_matches_spec_after",
    "foreign_key_check",
    "down",
    "db_matches_spec_after_down",
    "up_again",
    "db_matches_spec_after_up_again",
];

#[test]
fn accepted_proposal_records_all_check_steps_in_order() {
    let tmp = tempfile::tempdir().unwrap();
    let p = Project::init(tmp.path().join("p"), "t").unwrap();
    let Outcome::Accepted { checks, .. } = p.propose(initial(), Actor::Developer, false).unwrap()
    else {
        panic!("expected acceptance");
    };
    let names: Vec<_> = checks.iter().map(|c| c.check.as_str()).collect();
    assert_eq!(names, ALL_STEPS);
    assert!(checks.iter().all(|c| c.passed && !c.detail.is_empty()));
    // The same steps are in the event log.
    let events = p.events().unwrap();
    let EventKind::ChangeSetAccepted { checks: logged, .. } = &events.last().unwrap().kind else {
        panic!("last event must be the acceptance");
    };
    assert_eq!(logged, &checks);
}

#[test]
fn broken_migration_rejection_records_the_failing_step() {
    let tmp = tempfile::tempdir().unwrap();
    let p = run_sequence(&tmp.path().join("p"));
    // Tamper with an applied migration so the database no longer matches the spec.
    let up = p.dir().join("migrations/0001_initial/up.sql");
    let text = fs::read_to_string(&up).unwrap();
    assert!(text.contains("amount"));
    fs::write(&up, text.replace("amount", "total")).unwrap();

    let out = p
        .propose(
            cs(
                "add_tag",
                vec![SchemaChange::AddColumn {
                    table: "expenses".into(),
                    column: col("tag", ColumnType::Text),
                }],
            ),
            Actor::Developer,
            false,
        )
        .unwrap();
    let Outcome::Rejected { findings, checks } = out else {
        panic!("expected rejection");
    };
    assert_eq!(findings[0].code, "db_does_not_match_spec");
    let names: Vec<_> = checks
        .iter()
        .map(|c| (c.check.as_str(), c.passed))
        .collect();
    assert_eq!(
        names,
        [
            ("static_rules", true),
            ("earlier_migrations_applied", true),
            ("db_matches_spec_before", false),
        ]
    );
    let events = p.events().unwrap();
    let EventKind::ChangeSetRejected { checks: logged, .. } = &events.last().unwrap().kind else {
        panic!("last event must be the rejection");
    };
    assert_eq!(logged, &checks);
}
