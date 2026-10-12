use std::fs;
use std::path::Path;

use nocodo_orchestrator::{
    AnswerOutcome, AnsweredQuestion, Applier, ApplierError, ContinueOutcome, Feedback,
    OrchestratorError, Outcome, Project, PromptOutcome, Proposal, Proposer, ProposerError, Reply,
};
use nocodo_types::apply::{AppliedMigration, ApplyReport, FailedMigration};
use nocodo_types::change::{ChangeSet, SchemaChange};
use nocodo_types::event::{Actor, EventKind};
use nocodo_types::project::ProjectBase;
use nocodo_types::question::{Answer, Question, tech_words_in};
use nocodo_types::schema::{Column, ColumnType, SchemaSpec};

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

/// A proposer for tests that must never reach the model.
struct NeverCalled;

impl Proposer for NeverCalled {
    async fn propose(
        &self,
        _current: &SchemaSpec,
        _prompt: &str,
        _answers: &[AnsweredQuestion],
        _feedback: Option<&Feedback>,
    ) -> Result<Proposal, ProposerError> {
        panic!("the model must not be called");
    }
}

fn answer(id: &str, choice: usize) -> Answer {
    Answer {
        question_id: id.into(),
        choice,
        by_default: false,
    }
}

/// Answers the one open question and resumes with a proposer that must not be called.
async fn resolve_async(p: &Project, id: &str, choice: usize) -> PromptOutcome {
    assert_eq!(
        p.answer(&[answer(id, choice)]).unwrap(),
        AnswerOutcome::ReadyToContinue
    );
    let ContinueOutcome::Continued(out) = p.continue_pending(&NeverCalled, false, 3).await.unwrap()
    else {
        panic!("expected the run to continue");
    };
    out
}

fn resolve(p: &Project, id: &str, choice: usize) -> PromptOutcome {
    block_on(resolve_async(p, id, choice))
}

fn block_on<F: std::future::Future>(f: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(f)
}

#[test]
fn destructive_asks_the_user_and_writes_no_files_until_yes() {
    let tmp = tempfile::tempdir().unwrap();
    let p = run_sequence(&tmp.path().join("p"));
    let before = snapshot(p.dir());
    let Outcome::Waiting { questions } = p.propose(drop_notes(), Actor::Developer, false).unwrap()
    else {
        panic!("expected a confirmation question");
    };
    assert_eq!(before, snapshot(p.dir()));
    assert_eq!(questions.len(), 1);
    let q = &questions[0];
    assert_eq!((q.id.as_str(), q.asked_by.as_str()), ("q1", "system"));
    assert!(q.blocking && q.default.is_none());
    assert_eq!(q.choices, ["Yes, forget it", "No, keep it"]);
    assert_eq!(
        q.text,
        "This will forget the notes you have entered on each expense (The notes). Continue?"
    );
    assert!(tech_words_in(&q.text).is_empty());
    let events = p.events().unwrap();
    let EventKind::QuestionsAsked {
        prompt_seq,
        proposal_seq,
        ..
    } = &events.last().unwrap().kind
    else {
        panic!("last event must be the question");
    };
    assert_eq!(*prompt_seq, None);
    assert_eq!(*proposal_seq, Some(events[events.len() - 2].seq));
    assert_eq!(events.last().unwrap().actor, Actor::System);

    // Nothing else starts while it waits, even with confirmation.
    assert!(matches!(
        p.propose(drop_notes(), Actor::Developer, true),
        Err(OrchestratorError::QuestionsWaiting(_))
    ));

    // "No": rejected with user_declined, still no files.
    let PromptOutcome::Done {
        outcome: Outcome::Rejected { findings, .. },
        ..
    } = resolve(&p, "q1", 2)
    else {
        panic!("expected a rejection");
    };
    assert_eq!(findings[0].code, "user_declined");
    assert_eq!(before, snapshot(p.dir()));

    // --confirm-destructive skips the question.
    assert!(matches!(
        p.propose(drop_notes(), Actor::Developer, true).unwrap(),
        Outcome::Accepted { migration, .. } if migration == "0003_drop_notes"
    ));
    assert!(
        !p.events()
            .unwrap()
            .iter()
            .skip(events.len() + 3)
            .any(|e| matches!(e.kind, EventKind::QuestionsAsked { .. }))
    );
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
    let Outcome::Waiting { questions } = p
        .propose(fixture("04-drop-notes.json"), Actor::Developer, false)
        .unwrap()
    else {
        panic!("04 must ask for confirmation");
    };
    assert_eq!(before, snapshot(p.dir()));
    let text = &questions[0].text;
    assert!(text.starts_with("This will forget the notes you have entered on each transaction ("));
    assert!(tech_words_in(text).is_empty(), "{text}");

    let seen_before = p.events().unwrap().len();
    let PromptOutcome::Done {
        outcome: Outcome::Accepted { migration, .. },
        ..
    } = resolve(&p, "q1", 1)
    else {
        panic!("04 must be accepted after yes");
    };
    // The accepted event points at the original proposal; no second proposal.
    let events = p.events().unwrap();
    let proposals: Vec<_> = events
        .iter()
        .filter(|e| matches!(&e.kind, EventKind::ChangeSetProposed { change_set, .. } if change_set.name == "remove_payment_notes"))
        .collect();
    assert_eq!(proposals.len(), 1);
    assert!(events.iter().skip(seen_before).any(|e| matches!(
        &e.kind,
        EventKind::ChangeSetAccepted { proposal_seq, .. } if *proposal_seq == proposals[0].seq
    )));
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

    use super::*;

    /// What the proposer saw on one call.
    struct Call {
        answers: Vec<AnsweredQuestion>,
        feedback: Option<Feedback>,
    }

    struct Scripted {
        queue: Mutex<Vec<Result<Reply, String>>>,
        calls: Mutex<Vec<Call>>,
    }

    impl Scripted {
        fn new(items: Vec<ChangeSet>) -> Self {
            Self::script(
                items
                    .into_iter()
                    .map(|c| {
                        Ok(Reply::change_set(
                            c,
                            vec!["Money is in one currency.".into()],
                        ))
                    })
                    .collect(),
            )
        }

        fn script(items: Vec<Result<Reply, String>>) -> Self {
            Scripted {
                queue: Mutex::new(items),
                calls: Mutex::new(vec![]),
            }
        }

        fn feedback_seen(&self) -> Vec<bool> {
            self.calls
                .lock()
                .unwrap()
                .iter()
                .map(|c| c.feedback.is_some())
                .collect()
        }
    }

    impl Proposer for Scripted {
        async fn propose(
            &self,
            _current: &SchemaSpec,
            _prompt: &str,
            answers: &[AnsweredQuestion],
            feedback: Option<&Feedback>,
        ) -> Result<Proposal, ProposerError> {
            self.calls.lock().unwrap().push(Call {
                answers: answers.to_vec(),
                feedback: feedback.cloned(),
            });
            let reply = self.queue.lock().unwrap().remove(0)?;
            Ok(Proposal {
                reply,
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
        let PromptOutcome::Done {
            outcome, attempts, ..
        } = r
        else {
            panic!()
        };
        assert!(matches!(outcome, Outcome::Accepted { .. }));
        assert_eq!(attempts, 2);
        assert_eq!(proposer.feedback_seen(), vec![false, true]);
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
        let PromptOutcome::Done {
            outcome, attempts, ..
        } = r
        else {
            panic!()
        };
        assert!(matches!(outcome, Outcome::Rejected { .. }));
        assert_eq!(attempts, 3);
        assert_eq!(
            fs::read_dir(project.dir().join("migrations"))
                .unwrap()
                .count(),
            0
        );
        assert_eq!(project.current_schema().unwrap().tables.len(), 0);
    }

    struct Failing(Option<nocodo_orchestrator::ProposerFailure>);

    impl Proposer for Failing {
        async fn propose(
            &self,
            _current: &SchemaSpec,
            _prompt: &str,
            _answers: &[AnsweredQuestion],
            _feedback: Option<&Feedback>,
        ) -> Result<Proposal, ProposerError> {
            Err(match &self.0 {
                Some(f) => Box::new(f.clone()),
                None => "boom".into(),
            })
        }
    }

    #[tokio::test]
    async fn failed_model_call_records_prompt_failed() {
        let dir = tempfile::tempdir().unwrap();
        let project = Project::init(dir.path().join("p"), "p").unwrap();
        let failing = Failing(Some(nocodo_orchestrator::ProposerFailure {
            message: "429 concurrent_budget_exceeded".into(),
            run_id: Some("run-9".into()),
            provider_retries: 3,
        }));
        let err = project
            .run_prompt("x", &failing, false, 3)
            .await
            .unwrap_err();
        assert!(matches!(err, OrchestratorError::Proposer(_)));
        let events = project.events().unwrap();
        assert_eq!(events.len(), 3);
        match &events[2].kind {
            EventKind::PromptFailed {
                prompt_seq,
                error,
                run_id,
                detail,
            } => {
                assert_eq!(*prompt_seq, 2);
                assert_eq!(error, "429 concurrent_budget_exceeded");
                assert_eq!(run_id.as_deref(), Some("run-9"));
                assert_eq!(
                    detail,
                    "The model call for prompt 2 failed: 429 concurrent_budget_exceeded \
                     (retried 3 times). Nothing was proposed; see .agent-logs/run-9.jsonl. \
                     Run `nocodo continue` to try again."
                );
            }
            other => panic!("unexpected {other:?}"),
        }
        assert_eq!(events[2].actor, Actor::System);

        // Any other error is recorded with its message only.
        project
            .run_prompt("y", &Failing(None), false, 3)
            .await
            .unwrap_err();
        let events = project.events().unwrap();
        assert!(matches!(
            &events[4].kind,
            EventKind::PromptFailed { error, run_id: None, .. } if error == "boom"
        ));
    }

    fn q(text: &str, blocking: bool) -> Question {
        Question {
            id: String::new(),
            asked_by: String::new(),
            text: text.into(),
            choices: vec!["Yes".into(), "No".into()],
            default: (!blocking).then_some(1),
            blocking,
            why: "test".into(),
        }
    }

    fn ask(questions: Vec<Question>) -> Result<Reply, String> {
        Ok(Reply {
            questions,
            change_set: None,
            assumptions: vec![],
        })
    }

    fn give(cs: ChangeSet) -> Result<Reply, String> {
        Ok(Reply::change_set(cs, vec![]))
    }

    fn kinds_of(p: &Project) -> Vec<&'static str> {
        p.events()
            .unwrap()
            .iter()
            .map(|e| match &e.kind {
                EventKind::ProjectCreated { .. } => "project_created",
                EventKind::UserPrompt { .. } => "user_prompt",
                EventKind::PromptFailed { .. } => "prompt_failed",
                EventKind::QuestionsAsked { .. } => "questions_asked",
                EventKind::QuestionAnswered { .. } => "question_answered",
                EventKind::QuestionsRejected { .. } => "questions_rejected",
                EventKind::QuestionsDropped { .. } => "questions_dropped",
                EventKind::ChangeSetProposed { .. } => "change_set_proposed",
                EventKind::ChangeSetAccepted { .. } => "change_set_accepted",
                EventKind::ChangeSetRejected { .. } => "change_set_rejected",
                _ => "other",
            })
            .collect()
    }

    fn new_project() -> (tempfile::TempDir, Project) {
        let dir = tempfile::tempdir().unwrap();
        let project = Project::init(dir.path().join("p"), "p").unwrap();
        (dir, project)
    }

    async fn resume(p: &Project, proposer: &Scripted) -> PromptOutcome {
        match p.continue_pending(proposer, false, 3).await.unwrap() {
            ContinueOutcome::Continued(out) => out,
            other => panic!("expected the run to continue, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn questions_then_answers_then_change_set() {
        let (_dir, project) = new_project();
        let proposer = Scripted::script(vec![
            ask(vec![
                q("Is a spend for the whole family?", false),
                q("Which bank?", true),
            ]),
            give(initial()),
        ]);
        let PromptOutcome::Waiting { questions } = project
            .run_prompt("track spending", &proposer, false, 3)
            .await
            .unwrap()
        else {
            panic!("expected questions");
        };
        // Blocking first, IDs q1, q2, asked by the agent.
        assert_eq!(questions[0].text, "Which bank?");
        assert_eq!(
            questions
                .iter()
                .map(|q| (q.id.as_str(), q.asked_by.as_str()))
                .collect::<Vec<_>>(),
            [("q1", "schema"), ("q2", "schema")]
        );
        assert_eq!(project.open_questions().unwrap(), questions);

        // Answers over two calls: nothing resumes, and the model is not called, until the last.
        assert!(matches!(
            project.answer(&[answer("q1", 2)]).unwrap(),
            AnswerOutcome::StillWaiting { open } if open.len() == 1
        ));
        assert_eq!(proposer.calls.lock().unwrap().len(), 1);
        assert_eq!(
            project.answer(&[answer("q2", 1)]).unwrap(),
            AnswerOutcome::ReadyToContinue
        );
        assert_eq!(proposer.calls.lock().unwrap().len(), 1);
        let PromptOutcome::Done {
            outcome, attempts, ..
        } = resume(&project, &proposer).await
        else {
            panic!()
        };
        assert!(matches!(outcome, Outcome::Accepted { .. }));
        assert_eq!(attempts, 2);

        assert_eq!(
            kinds_of(&project),
            [
                "project_created",
                "user_prompt",
                "questions_asked",
                "question_answered",
                "question_answered",
                "change_set_proposed",
                "change_set_accepted"
            ]
        );
        let events = project.events().unwrap();
        let EventKind::QuestionAnswered { detail, .. } = &events[3].kind else {
            panic!()
        };
        assert_eq!(detail, "q1 answered: \"No\" (choice 2 of 2).");
        assert_eq!(events[3].actor, Actor::User);
        assert_eq!(
            events[2].actor,
            Actor::SchemaAgent {
                run_id: "run-1".into(),
                model: "test:model".into()
            }
        );
        // The second call sees both answers, as part of this prompt.
        let calls = proposer.calls.lock().unwrap();
        assert!(calls[0].answers.is_empty());
        assert_eq!(calls[1].answers.len(), 2);
        assert!(calls[1].answers.iter().all(|a| a.in_this_prompt));
        // The proposal belongs to the original prompt.
        assert!(matches!(
            &events[5].kind,
            EventKind::ChangeSetProposed {
                prompt_seq: Some(2),
                attempt: 2,
                ..
            }
        ));
        assert!(project.open_questions().unwrap().is_empty());
    }

    #[tokio::test]
    async fn bad_answers_are_refused_and_record_nothing() {
        let (_dir, project) = new_project();
        let proposer = Scripted::script(vec![ask(vec![q("One?", false), q("Two?", false)])]);
        project.run_prompt("x", &proposer, false, 3).await.unwrap();
        let before = project.events().unwrap().len();
        for bad in [
            vec![answer("q9", 1)],
            vec![answer("q1", 0)],
            vec![answer("q1", 3)],
            vec![answer("q2", 1), answer("q1", 3)],
            vec![answer("q1", 1), answer("q1", 2)],
        ] {
            assert!(
                matches!(
                    project.answer(&bad),
                    Err(OrchestratorError::InvalidAnswer(_))
                ),
                "{bad:?}"
            );
        }
        assert_eq!(project.events().unwrap().len(), before);
        project.answer(&[answer("q1", 1)]).unwrap();
        let err = project.answer(&[answer("q1", 2)]).unwrap_err();
        assert!(err.to_string().contains("already answered"));
    }

    #[tokio::test]
    async fn choices_are_numbered_from_one() {
        let (_dir, project) = new_project();
        let proposer = Scripted::script(vec![ask(vec![q("One?", false)])]);
        project.run_prompt("x", &proposer, false, 3).await.unwrap();
        project.answer(&[answer("q1", 1)]).unwrap();
        let events = project.events().unwrap();
        let EventKind::QuestionAnswered { answer, detail } = &events.last().unwrap().kind else {
            panic!()
        };
        assert_eq!(answer.choice, 1);
        assert_eq!(detail, "q1 answered: \"Yes\" (choice 1 of 2).");
    }

    #[tokio::test]
    async fn technical_word_round_is_rejected_repaired_and_recorded() {
        let (_dir, project) = new_project();
        let proposer = Scripted::script(vec![
            ask(vec![q("Should each payment get its own table?", false)]),
            ask(vec![q("Is each payment kept separately?", false)]),
        ]);
        let out = project.run_prompt("x", &proposer, false, 3).await.unwrap();
        assert!(matches!(out, PromptOutcome::Waiting { .. }));
        assert_eq!(
            kinds_of(&project),
            [
                "project_created",
                "user_prompt",
                "questions_rejected",
                "questions_asked"
            ]
        );
        let events = project.events().unwrap();
        let EventKind::QuestionsRejected {
            prompt_seq,
            findings,
        } = &events[2].kind
        else {
            panic!()
        };
        assert_eq!(*prompt_seq, 2);
        assert_eq!(findings[0].code, "technical_word");
        // The rejected round went back to the agent with its questions.
        let calls = proposer.calls.lock().unwrap();
        let fb = calls[1].feedback.as_ref().unwrap();
        assert_eq!(fb.previous.questions.len(), 1);
        assert_eq!(fb.findings[0].code, "technical_word");
    }

    #[tokio::test]
    async fn wrong_reply_shape_is_rejected() {
        let (_dir, project) = new_project();
        let both = Ok(Reply {
            questions: vec![q("One?", false)],
            change_set: Some(initial()),
            assumptions: vec![],
        });
        let neither = ask(vec![]);
        let proposer = Scripted::script(vec![both, neither]);
        let out = project.run_prompt("x", &proposer, false, 2).await.unwrap();
        let PromptOutcome::Done {
            outcome: Outcome::Rejected { findings, .. },
            change_set,
            ..
        } = out
        else {
            panic!()
        };
        assert_eq!(findings[0].code, "reply_shape");
        assert!(change_set.is_none());
        assert_eq!(
            kinds_of(&project),
            [
                "project_created",
                "user_prompt",
                "questions_rejected",
                "questions_rejected"
            ]
        );
    }

    #[tokio::test]
    async fn questions_after_answers_are_rejected() {
        let (_dir, project) = new_project();
        let proposer = Scripted::script(vec![
            ask(vec![q("One?", false)]),
            ask(vec![q("Another?", false)]),
            give(initial()),
        ]);
        project.run_prompt("x", &proposer, false, 3).await.unwrap();
        project.answer(&[answer("q1", 1)]).unwrap();
        let PromptOutcome::Done {
            outcome, attempts, ..
        } = resume(&project, &proposer).await
        else {
            panic!()
        };
        assert!(matches!(outcome, Outcome::Accepted { .. }));
        assert_eq!(attempts, 3);
        let events = project.events().unwrap();
        assert!(events.iter().any(|e| matches!(
            &e.kind,
            EventKind::QuestionsRejected { findings, .. }
                if findings.iter().any(|f| f.code == "questions_after_answers")
        )));
        assert!(project.open_questions().unwrap().is_empty());
    }

    #[tokio::test]
    async fn defaults_skip_blocking_and_prompt_refuses_while_waiting() {
        let (_dir, project) = new_project();
        let proposer = Scripted::script(vec![ask(vec![
            q("One?", false),
            q("Two?", true),
            q("Three?", false),
        ])]);
        project.run_prompt("x", &proposer, false, 3).await.unwrap();
        // Blocking first: q1 is "Two?".
        let given = vec![answer("q3", 2)];
        let mut all = given.clone();
        all.extend(project.default_answers(&given).unwrap());
        assert_eq!(
            all.iter()
                .map(|a| (a.question_id.as_str(), a.choice, a.by_default))
                .collect::<Vec<_>>(),
            [("q3", 2, false), ("q2", 1, true)]
        );
        let AnswerOutcome::StillWaiting { open } = project.answer(&all).unwrap() else {
            panic!("the blocking question must stay open")
        };
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].text, "Two?");
        let events = project.events().unwrap();
        let EventKind::QuestionAnswered { detail, .. } = &events.last().unwrap().kind else {
            panic!()
        };
        assert!(detail.starts_with("q2 answered by the default"));

        let err = project
            .run_prompt("y", &proposer, false, 3)
            .await
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "1 question is waiting (q1). Answer them with `nocodo answer`, or see them with `nocodo questions`."
        );
        assert!(matches!(
            project.propose(initial(), Actor::Developer, false),
            Err(OrchestratorError::QuestionsWaiting(_))
        ));
        assert!(matches!(
            project.continue_pending(&proposer, false, 3).await.unwrap(),
            ContinueOutcome::StillWaiting { .. }
        ));
    }

    #[tokio::test]
    async fn failed_call_is_continued_without_a_second_prompt() {
        let (_dir, project) = new_project();
        let proposer = Scripted::script(vec![Err("429 busy".into()), give(initial())]);
        project
            .run_prompt("x", &proposer, false, 3)
            .await
            .unwrap_err();
        assert_eq!(
            kinds_of(&project),
            ["project_created", "user_prompt", "prompt_failed"]
        );
        let PromptOutcome::Done { outcome, .. } = resume(&project, &proposer).await else {
            panic!()
        };
        assert!(matches!(outcome, Outcome::Accepted { .. }));
        let k = kinds_of(&project);
        assert_eq!(k.iter().filter(|k| **k == "user_prompt").count(), 1);
        assert_eq!(k.iter().filter(|k| **k == "change_set_proposed").count(), 1);
        // Nothing left to continue.
        assert_eq!(
            project.continue_pending(&proposer, false, 3).await.unwrap(),
            ContinueOutcome::NothingToContinue
        );
    }

    #[tokio::test]
    async fn failure_after_answers_is_continued_with_the_answers() {
        let (_dir, project) = new_project();
        let proposer = Scripted::script(vec![
            ask(vec![q("One?", false)]),
            Err("boom".into()),
            give(initial()),
        ]);
        project.run_prompt("x", &proposer, false, 3).await.unwrap();
        project.answer(&[answer("q1", 1)]).unwrap();
        let err = project
            .continue_pending(&proposer, false, 3)
            .await
            .unwrap_err();
        assert!(matches!(err, OrchestratorError::Proposer(_)));
        assert_eq!(kinds_of(&project).last(), Some(&"prompt_failed"));
        let PromptOutcome::Done { outcome, .. } = resume(&project, &proposer).await else {
            panic!()
        };
        assert!(matches!(outcome, Outcome::Accepted { .. }));
        let k = kinds_of(&project);
        assert_eq!(k.iter().filter(|k| **k == "user_prompt").count(), 1);
        let calls = proposer.calls.lock().unwrap();
        assert_eq!(calls[2].answers.len(), 1);
        assert!(calls[2].answers[0].in_this_prompt);
    }

    #[tokio::test]
    async fn destructive_in_a_prompt_run_asks_after_the_agent_round() {
        let (_dir, project) = new_project();
        project.propose(initial(), Actor::Developer, false).unwrap();
        let proposer = Scripted::script(vec![
            ask(vec![q("Do you still want the notes?", false)]),
            give(drop_notes()),
        ]);
        project
            .run_prompt("drop notes", &proposer, false, 3)
            .await
            .unwrap();
        project.answer(&[answer("q1", 2)]).unwrap();
        let PromptOutcome::Waiting { questions } = resume(&project, &proposer).await else {
            panic!("expected the confirmation round")
        };
        assert_eq!(
            (questions[0].id.as_str(), questions[0].asked_by.as_str()),
            ("q2", "system")
        );
        assert!(tech_words_in(&questions[0].text).is_empty());
        assert_eq!(proposer.calls.lock().unwrap().len(), 2);
        // Nothing was written yet.
        assert_eq!(
            fs::read_dir(project.dir().join("migrations"))
                .unwrap()
                .count(),
            1
        );

        // Yes: the same proposal is accepted; the proposer is not called again.
        let PromptOutcome::Done { outcome, .. } = resolve_async(&project, "q2", 1).await else {
            panic!()
        };
        assert!(matches!(outcome, Outcome::Accepted { .. }));
        let events = project.events().unwrap();
        let proposals: Vec<u64> = events
            .iter()
            .filter(|e| matches!(&e.kind, EventKind::ChangeSetProposed { change_set, .. } if change_set.name == "drop_notes"))
            .map(|e| e.seq)
            .collect();
        assert_eq!(proposals.len(), 1);
        assert!(events.iter().any(|e| matches!(
            &e.kind,
            EventKind::ChangeSetAccepted { proposal_seq, .. } if *proposal_seq == proposals[0]
        )));
        assert_eq!(proposer.calls.lock().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn destructive_with_another_finding_is_rejected_without_a_question() {
        let (_dir, project) = new_project();
        project.propose(initial(), Actor::Developer, false).unwrap();
        let mut bad = drop_notes();
        bad.changes.push(SchemaChange::AddColumn {
            table: "missing".into(),
            column: col("x", ColumnType::Text),
        });
        let proposer = Scripted::new(vec![bad, add_note()]);
        let out = project.run_prompt("x", &proposer, false, 2).await.unwrap();
        let PromptOutcome::Done { outcome, .. } = out else {
            panic!("no question may be asked")
        };
        assert!(matches!(outcome, Outcome::Accepted { .. }));
        let calls = proposer.calls.lock().unwrap();
        let fb = calls[1].feedback.as_ref().unwrap();
        assert!(!fb.findings.is_empty());
        assert!(
            fb.findings
                .iter()
                .all(|f| f.code != "destructive_not_confirmed")
        );
        assert!(!kinds_of(&project).contains(&"questions_asked"));
    }

    #[tokio::test]
    async fn confirm_destructive_flag_skips_the_question_in_a_prompt_run() {
        let (_dir, project) = new_project();
        project.propose(initial(), Actor::Developer, false).unwrap();
        let proposer = Scripted::new(vec![drop_notes()]);
        let out = project.run_prompt("x", &proposer, true, 1).await.unwrap();
        assert!(matches!(
            out,
            PromptOutcome::Done {
                outcome: Outcome::Accepted { .. },
                ..
            }
        ));
    }
}

const ALL_STEPS: [&str; 11] = [
    "static_rules",
    "earlier_migrations_applied",
    "db_matches_spec_before",
    "up",
    "rows_kept",
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

/// Returns a canned report; with `create_db` it also creates an empty (valid) `data.sqlite`.
struct FakeApplier {
    report: ApplyReport,
    create_db: bool,
}

impl Applier for FakeApplier {
    fn apply_pending(&self, dir: &Path) -> Result<ApplyReport, ApplierError> {
        if self.create_db {
            fs::write(dir.join("data.sqlite"), b"")?;
        }
        Ok(self.report.clone())
    }
}

fn applied(name: &str) -> AppliedMigration {
    AppliedMigration {
        migration: name.into(),
        backup: Some(format!("backups/before_{name}.sqlite")),
        rows_before: [("expenses".to_string(), 4)].into(),
        rows_after: [("expenses".to_string(), 4)].into(),
    }
}

fn kinds(p: &Project) -> Vec<&'static str> {
    p.events()
        .unwrap()
        .iter()
        .map(|e| match &e.kind {
            EventKind::MigrationApplied { .. } => "migration_applied",
            EventKind::MigrationFailed { .. } => "migration_failed",
            EventKind::DatabaseChecked { .. } => "database_checked",
            _ => "other",
        })
        .filter(|k| *k != "other")
        .collect()
}

#[test]
fn apply_records_events_and_checks_database() {
    let tmp = tempfile::tempdir().unwrap();
    let p = Project::init(tmp.path().join("p"), "t").unwrap();
    let fake = FakeApplier {
        report: ApplyReport {
            applied: vec![applied("0001_a"), applied("0002_b")],
            failed: None,
        },
        create_db: true,
    };
    let out = p.apply_pending(&fake).unwrap();
    assert_eq!(
        kinds(&p),
        ["migration_applied", "migration_applied", "database_checked"]
    );
    let check = out.check.unwrap();
    assert!(check.passed, "{}", check.detail);
    assert_eq!(check.check, "db_matches_spec_after_apply");
    assert!(out.details[0].contains("backups/before_0001_a.sqlite"));
    assert!(out.details[0].contains("4 before, 4 after"));
}

#[test]
fn apply_failure_records_failure_and_no_check() {
    let tmp = tempfile::tempdir().unwrap();
    let p = Project::init(tmp.path().join("p"), "t").unwrap();
    let fake = FakeApplier {
        report: ApplyReport {
            applied: vec![applied("0001_a")],
            failed: Some(FailedMigration {
                migration: "0002_b".into(),
                backup: Some("backups/before_0002_b.sqlite".into()),
                error: "near \"X\": syntax error".into(),
                restored: true,
            }),
        },
        create_db: false,
    };
    let out = p.apply_pending(&fake).unwrap();
    assert_eq!(kinds(&p), ["migration_applied", "migration_failed"]);
    assert!(out.check.is_none());
    let last = out.details.last().unwrap();
    assert!(last.contains("syntax error") && last.contains("exactly as it was before"));
}

#[test]
fn apply_nothing_pending_appends_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let p = Project::init(tmp.path().join("p"), "t").unwrap();
    let before = p.events().unwrap().len();
    let fake = FakeApplier {
        report: ApplyReport::default(),
        create_db: false,
    };
    let out = p.apply_pending(&fake).unwrap();
    assert!(out.check.is_none() && out.details.is_empty());
    assert_eq!(p.events().unwrap().len(), before);
}

#[test]
fn apply_check_mismatch_is_recorded_as_failed_check() {
    let tmp = tempfile::tempdir().unwrap();
    let p = Project::init(tmp.path().join("p"), "t").unwrap();
    p.propose(initial(), Actor::Developer, false).unwrap();
    let fake = FakeApplier {
        report: ApplyReport {
            applied: vec![applied("0001_initial")],
            failed: None,
        },
        create_db: true, // empty database: does not match the spec
    };
    let out = p.apply_pending(&fake).unwrap();
    let check = out.check.unwrap();
    assert!(!check.passed);
    assert!(kinds(&p).ends_with(&["database_checked"]));
}

#[test]
fn old_event_log_still_loads() {
    let tmp = tempfile::tempdir().unwrap();
    let project = Project::init(tmp.path().join("p"), "old").unwrap();
    let old = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/examples/events.jsonl"),
    )
    .unwrap();
    std::fs::write(project.dir().join("events.jsonl"), &old).unwrap();
    assert_eq!(project.events().unwrap().len(), old.lines().count());
}
