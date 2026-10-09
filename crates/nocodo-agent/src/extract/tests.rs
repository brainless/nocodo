use nocodo_types::project::ModelConfig;
use rig_core::{
    message::{AssistantContent, ToolCall, ToolFunction, ToolName},
    test_utils::{MockCompletionModel, MockTurn},
};
use serde::Deserialize;
use serde_json::{Value, json};

use super::*;

#[derive(Debug, PartialEq, Deserialize, Serialize, JsonSchema)]
struct Person {
    name: String,
    tags: Vec<String>,
}

fn submit_call(arguments: Value) -> AssistantContent {
    AssistantContent::ToolCall(ToolCall::from_wire(
        "toolu_1",
        ToolFunction::new(ToolName::new("submit").unwrap(), arguments),
    ))
}

#[test]
fn follow_up_after_rejected_submit_is_an_error_tool_result() {
    let rejected = vec![submit_call(json!({ "name": "Ada", "tags": "[\"a\"]" }))];
    let error = "invalid type: string \"[\\\"a\\\"]\", expected a sequence at line 1 column 30";
    insta::assert_snapshot!(pretty(&follow_up(&rejected, error)));
}

#[test]
fn follow_up_after_no_submit_is_a_user_message() {
    let rejected = vec![AssistantContent::text("Sure, here you go.")];
    insta::assert_snapshot!(pretty(&follow_up(
        &rejected,
        "the model did not call the `submit` tool"
    )));
}

fn pretty(follow_up: &FollowUp) -> String {
    serde_json::to_string_pretty(&follow_up.as_json()).unwrap()
}

impl FollowUp {
    fn as_json(&self) -> Value {
        match self {
            FollowUp::ToolResult { assistant, prompt } => json!({
                "kind": "tool_result", "assistant": assistant, "prompt": prompt,
            }),
            FollowUp::UserMessage { assistant, prompt } => json!({
                "kind": "user_message", "assistant": assistant, "prompt": prompt,
            }),
        }
    }
}

fn log_lines(dir: &std::path::Path) -> Vec<Value> {
    let file = std::fs::read_dir(dir.join(".agent-logs"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    std::fs::read_to_string(file)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn context(name: &str, retries: u64) -> AgentContext {
    let dir = std::env::temp_dir().join(format!("nocodo-extract-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut ctx = AgentContext::new(dir, "test", ModelConfig::default());
    ctx.retries = retries;
    ctx
}

#[tokio::test]
async fn rejected_output_is_retried_in_the_same_conversation() {
    let model = MockCompletionModel::from_turns([
        MockTurn::tool_call(
            "toolu_1",
            "submit",
            json!({ "name": "Ada", "tags": "a, b" }),
        ),
        MockTurn::tool_call("toolu_2", "submit", json!({ "name": "Ada", "tags": ["a"] })),
    ]);
    let ctx = context("retry", 2);
    let (person, _) =
        extract_with_model::<Person>(model.clone(), false, &ctx, "Be careful.", "Ada: a")
            .await
            .unwrap();
    assert_eq!(person.tags, vec!["a".to_string()]);

    let requests = model.requests();
    assert_eq!(requests.len(), 2);
    let history = serde_json::to_value(&requests[1].chat_history).unwrap();
    let text = history.to_string();
    assert_eq!(requests[1].chat_history.len(), 4); // system, input, rejected call, error result
    assert!(text.contains("Ada: a"));
    assert!(text.contains("toolu_1"));
    assert!(text.contains("invalid type: string"));

    let lines = log_lines(&ctx.project_dir);
    let invalid: Vec<_> = lines
        .iter()
        .filter(|l| l["type"] == "invalid_output")
        .collect();
    assert_eq!(invalid.len(), 1);
    assert_eq!(invalid[0]["call"], 1);
    assert!(
        invalid[0]["error"]
            .as_str()
            .unwrap()
            .contains("invalid type: string")
    );
    let finished = lines.last().unwrap();
    assert_eq!(finished["type"], "run_finished");
    assert_eq!(
        (
            finished["calls"].clone(),
            finished["invalid_outputs"].clone()
        ),
        (json!(2), json!(1))
    );
    let _ = std::fs::remove_dir_all(&ctx.project_dir);
}

#[tokio::test]
async fn retries_are_bounded_and_the_failure_is_logged() {
    let bad = || MockTurn::tool_call("toolu_x", "submit", json!({ "name": 1 }));
    let model = MockCompletionModel::from_turns([bad(), bad()]);
    let ctx = context("bounded", 1);
    let err = extract_with_model::<Person>(model.clone(), false, &ctx, "p", "in")
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Extraction(_)));
    assert_eq!(model.request_count(), 2);
    let lines = log_lines(&ctx.project_dir);
    let finished = lines.last().unwrap();
    assert_eq!(finished["ok"], false);
    assert_eq!(
        (
            finished["calls"].clone(),
            finished["invalid_outputs"].clone()
        ),
        (json!(2), json!(2))
    );
    let _ = std::fs::remove_dir_all(&ctx.project_dir);
}

/// The shape of the Schema agent's output, without depending on its crate.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct Proposal {
    change_set: nocodo_types::change::ChangeSet,
    assumptions: Vec<String>,
}

/// Arguments of the first rejected `submit` call of two failed GLM runs
/// (`.projects/cp-finance-nw`): `change_set` (and, in the first, `assumptions`)
/// sent as strings holding JSON.
const FIXTURES: [(&str, &str); 2] = [
    (
        "initial",
        include_str!("fixtures/glm_stringified_change_set_initial.json"),
    ),
    (
        "follow_up",
        include_str!("fixtures/glm_stringified_change_set_follow_up.json"),
    ),
];

#[test]
fn logged_stringified_arguments_fail_to_parse_as_sent() {
    for (name, raw) in FIXTURES {
        let args: Value = serde_json::from_str(raw).unwrap();
        assert!(serde_json::from_value::<Proposal>(args).is_err(), "{name}");
    }
}

#[test]
fn logged_stringified_arguments_are_repaired() {
    let args: Value = serde_json::from_str(FIXTURES[0].1).unwrap();
    let (proposal, repaired) = repair_arguments::<Proposal>(&args).unwrap();
    assert!(repaired["change_set"].is_object());
    assert!(repaired["assumptions"].is_array());
    assert!(!proposal.change_set.changes.is_empty());
    assert!(!proposal.assumptions.is_empty());
}

/// The follow-up call also left out `assumptions`, which repair cannot invent:
/// as a `Proposal` it stays rejected (and goes through the normal retry), but
/// its stringified `change_set` itself is repaired.
#[test]
fn repair_does_not_invent_missing_fields() {
    #[derive(Debug, Deserialize)]
    struct ChangeOnly {
        change_set: nocodo_types::change::ChangeSet,
    }
    let args: Value = serde_json::from_str(FIXTURES[1].1).unwrap();
    assert!(repair_arguments::<Proposal>(&args).is_none());
    let (only, _) = repair_arguments::<ChangeOnly>(&args).unwrap();
    assert!(!only.change_set.changes.is_empty());
}

#[test]
fn unstringify_only_touches_json_objects_and_arrays() {
    assert_eq!(
        unstringify(&json!({ "a": "text", "b": "12", "c": "{oops" })),
        None
    );
    assert_eq!(
        unstringify(&json!({ "a": "{\"x\": \"[1]\"}", "b": ["[\"k\"]", "plain"] })),
        Some(json!({ "a": { "x": [1] }, "b": [["k"], "plain"] }))
    );
}

#[test]
fn unrepairable_arguments_are_not_accepted() {
    let args = json!({ "name": "Ada", "tags": "not json" });
    assert!(repair_arguments::<Person>(&args).is_none());
    // Repairable shape, but still the wrong type after repair.
    let args = json!({ "name": "Ada", "tags": "[1]" });
    assert!(repair_arguments::<Person>(&args).is_none());
}

#[tokio::test]
async fn stringified_arguments_are_repaired_without_another_call() {
    let args: Value = serde_json::from_str(FIXTURES[0].1).unwrap();
    let model = MockCompletionModel::from_turns([MockTurn::tool_call("toolu_1", "submit", args)]);
    let ctx = context("repair", 2);
    let (proposal, _) = extract_with_model::<Proposal>(model.clone(), false, &ctx, "p", "in")
        .await
        .unwrap();
    assert!(!proposal.change_set.changes.is_empty());
    assert_eq!(model.request_count(), 1);

    let lines = log_lines(&ctx.project_dir);
    let types: Vec<_> = lines.iter().map(|l| l["type"].as_str().unwrap()).collect();
    assert!(types.contains(&"repaired_output"));
    assert!(!types.contains(&"invalid_output"));
    let repaired = lines
        .iter()
        .find(|l| l["type"] == "repaired_output")
        .unwrap();
    assert!(repaired["original"]["change_set"].is_string());
    assert!(repaired["repaired"]["change_set"].is_object());
    let finished = lines.last().unwrap();
    assert_eq!(finished["ok"], true);
    assert_eq!(
        (
            finished["calls"].clone(),
            finished["invalid_outputs"].clone(),
            finished["repaired_outputs"].clone()
        ),
        (json!(1), json!(0), json!(1))
    );
    let _ = std::fs::remove_dir_all(&ctx.project_dir);
}

/// The follow-up call sent `change_set` as a string and left out `assumptions`.
/// Repair cannot fix that, but the model must be told about the missing field,
/// not the original "invalid type: string" error.
#[tokio::test]
async fn retry_after_failed_repair_names_the_real_problem() {
    let args: Value = serde_json::from_str(FIXTURES[1].1).unwrap();
    let good = json!({
        "change_set": serde_json::from_str::<Value>(args["change_set"].as_str().unwrap()).unwrap(),
        "assumptions": ["a"],
    });
    let model = MockCompletionModel::from_turns([
        MockTurn::tool_call("toolu_1", "submit", args.clone()),
        MockTurn::tool_call("toolu_2", "submit", good),
    ]);
    let ctx = context("repair-real-error", 2);
    extract_with_model::<Proposal>(model.clone(), false, &ctx, "p", "in")
        .await
        .unwrap();

    let requests = model.requests();
    assert_eq!(requests.len(), 2);
    let text = serde_json::to_value(&requests[1].chat_history)
        .unwrap()
        .to_string();
    assert!(text.contains("missing field `assumptions`"), "{text}");
    assert!(!text.contains("expected struct ChangeSet"), "{text}");

    let lines = log_lines(&ctx.project_dir);
    let invalid = lines
        .iter()
        .find(|l| l["type"] == "invalid_output")
        .unwrap();
    assert!(invalid["error"].as_str().unwrap().contains("assumptions"));
    assert!(
        invalid["original_error"]
            .as_str()
            .unwrap()
            .contains("invalid type: string")
    );
    assert!(
        serde_json::from_str::<Value>(invalid["output"].as_str().unwrap()).unwrap()["change_set"]
            .is_string()
    );
    let _ = std::fs::remove_dir_all(&ctx.project_dir);
}
