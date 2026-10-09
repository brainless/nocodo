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
            json!({ "name": "Ada", "tags": "[\"a\"]" }),
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
