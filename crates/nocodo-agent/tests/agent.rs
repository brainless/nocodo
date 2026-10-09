use nocodo_agent::{API_KEY_VAR, AgentLog, Error, build_model};
use nocodo_types::project::ModelConfig;
use serde_json::json;

#[test]
fn agent_log_writes_well_formed_jsonl() {
    let dir = std::env::temp_dir().join(format!("nocodo-agent-log-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let log = AgentLog::new(&dir, "run-1", "schema", "claude-sonnet-5-5");
    log.append("run_started", json!({ "preamble": "p", "input": "i" }));
    log.append("run_finished", json!({ "ok": true }));
    log.check().unwrap();

    let text = std::fs::read_to_string(dir.join(".agent-logs/run-1.jsonl")).unwrap();
    let lines: Vec<serde_json::Value> = text
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(lines.len(), 2);
    for line in &lines {
        assert_eq!(line["run_id"], "run-1");
        assert_eq!(line["role"], "schema");
        assert_eq!(line["model"], "claude-sonnet-5-5");
        assert!(line["at"].is_string());
    }
    assert_eq!(lines[0]["type"], "run_started");
    assert_eq!(lines[0]["input"], "i");
    assert_eq!(lines[1]["type"], "run_finished");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn missing_api_key_names_the_variable() {
    if std::env::var_os(API_KEY_VAR).is_some_and(|v| !v.is_empty()) {
        return; // a real key is set; the missing-key path cannot be exercised
    }
    let Err(err) = build_model(&ModelConfig::default()) else {
        panic!("should fail without a key");
    };
    assert!(matches!(err, Error::MissingApiKey(_)));
    assert!(err.to_string().contains("ANTHROPIC_API_KEY"));
}
