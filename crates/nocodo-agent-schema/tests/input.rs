use nocodo_agent_schema::{AnswerSoFar, Feedback, SchemaReply, build_input, preamble};
use nocodo_types::{
    change::{ChangeSet, SchemaChange},
    event::Finding,
    schema::{SPEC_VERSION, SchemaSpec},
};

fn spec() -> SchemaSpec {
    SchemaSpec {
        spec_version: SPEC_VERSION,
        tables: vec![],
    }
}

#[test]
fn input_without_feedback_is_deterministic() {
    let a = build_input(&spec(), "  track my spending  ", &[], None);
    assert_eq!(a, build_input(&spec(), "  track my spending  ", &[], None));
    let expected = "## Current schema\n\n```json\n{\n  \"spec_version\": 1,\n  \"tables\": []\n}\n```\n\n## User request\n\ntrack my spending\n";
    assert_eq!(a, expected);
}

#[test]
fn input_with_feedback_includes_previous_and_findings_verbatim() {
    let fb = Feedback {
        previous: SchemaReply {
            questions: vec![],
            change_set: Some(ChangeSet {
                name: "add_x".into(),
                summary: "s".into(),
                changes: vec![SchemaChange::DropTable { table: "x".into() }],
            }),
            assumptions: vec![],
        },
        findings: vec![Finding {
            code: "unknown_table".into(),
            message: "no table x".into(),
        }],
    };
    let out = build_input(&spec(), "p", &[], Some(&fb));
    assert!(out.contains("## Previous attempt (rejected)"));
    assert!(out.contains("\"op\": \"drop_table\""));
    assert!(out.contains("\"code\": \"unknown_table\""));
    assert!(out.contains("\"message\": \"no table x\""));
    assert_eq!(out, build_input(&spec(), "p", &[], Some(&fb)));
}

#[test]
fn input_with_answers_snapshot() {
    let answers = vec![
        AnswerSoFar {
            question: "Can one spend be for the whole family?".into(),
            chosen: "Yes, a spend can be for the whole family".into(),
            by_default: false,
        },
        AnswerSoFar {
            question: "Do you lend money to more than one person?".into(),
            chosen: "Only my brother".into(),
            by_default: true,
        },
    ];
    let out = build_input(&spec(), "track my family's money", &answers, None);
    assert!(out.contains("## Answers so far"));
    insta::assert_snapshot!(out);
}

#[test]
fn preamble_is_loaded() {
    assert!(preamble().contains("Schema agent"));
}

#[test]
fn preamble_has_modeling_rules() {
    let preamble = preamble();
    for rule in [
        "Store facts, not computed results.",
        "Model only what was asked.",
        "New entries are data, not schema.",
        "Every link is a real reference.",
        "Do not describe how things are stored",
        "## Asking the user",
        "at most 8 questions",
        "never ask again",
        "never what will be worked out or added for the user",
        "it never replaces or removes existing ones",
    ] {
        assert!(preamble.contains(rule), "missing: {rule}");
    }
}

#[test]
fn preamble_has_no_unfilled_placeholder() {
    assert!(!preamble().contains("{{"));
}

#[test]
fn preamble_snapshot() {
    insta::assert_snapshot!(preamble());
}
