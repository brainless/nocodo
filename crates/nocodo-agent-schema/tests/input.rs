use nocodo_agent_schema::{Feedback, build_input, preamble};
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
    let a = build_input(&spec(), "  track my spending  ", None);
    assert_eq!(a, build_input(&spec(), "  track my spending  ", None));
    let expected = "## Current schema\n\n```json\n{\n  \"spec_version\": 1,\n  \"tables\": []\n}\n```\n\n## User request\n\ntrack my spending\n";
    assert_eq!(a, expected);
}

#[test]
fn input_with_feedback_includes_previous_and_findings_verbatim() {
    let fb = Feedback {
        previous: ChangeSet {
            name: "add_x".into(),
            summary: "s".into(),
            changes: vec![SchemaChange::DropTable { table: "x".into() }],
        },
        findings: vec![Finding {
            code: "unknown_table".into(),
            message: "no table x".into(),
        }],
    };
    let out = build_input(&spec(), "p", Some(&fb));
    assert!(out.contains("## Previous attempt (rejected)"));
    assert!(out.contains("\"op\": \"drop_table\""));
    assert!(out.contains("\"code\": \"unknown_table\""));
    assert!(out.contains("\"message\": \"no table x\""));
    assert_eq!(out, build_input(&spec(), "p", Some(&fb)));
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
