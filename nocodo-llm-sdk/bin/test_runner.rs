use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::process::{exit, Command};

#[derive(Deserialize)]
struct Config {
    #[serde(default)]
    api_keys: HashMap<String, toml::Value>,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <config.toml>", args[0]);
        exit(1);
    }

    let keys = load_keys(&args[1]);

    // Run unit tests (no env vars needed)
    run_test(&[], &HashMap::new());

    // Integration tests with conditional execution
    let tests = [
        ("claude_integration", "ANTHROPIC_API_KEY"),
        ("gpt_integration", "OPENAI_API_KEY"),
        ("grok_integration", "XAI_API_KEY"),
        ("glm_integration", "CEREBRAS_API_KEY"),
        ("zai_integration", "ZAI_API_KEY"),
        ("zen_grok_integration", ""),
        ("zen_glm_integration", ""),
        ("tool_calling_integration", ""), // Runs for all available providers
        ("multi_turn_tool_use_integration", ""), // Runs for all available providers
    ];

    for (test, key) in tests {
        if key.is_empty() || keys.contains_key(key) {
            run_test(&["--test", test, "--", "--include-ignored"], &keys);
        }
    }
}

fn load_keys(path: &str) -> HashMap<String, String> {
    let content = std::fs::read_to_string(path).unwrap_or_default();
    let config: Config = toml::from_str(&content).unwrap_or(Config {
        api_keys: HashMap::new(),
    });

    // Convert lowercase keys to uppercase environment variable format
    // Only include string values (API keys), skip booleans and other types
    config
        .api_keys
        .into_iter()
        .filter_map(|(k, v)| v.as_str().map(|s| (k.to_uppercase(), s.to_string())))
        .collect()
}

fn run_test(args: &[&str], env_vars: &HashMap<String, String>) {
    let mut cmd = Command::new("cargo");
    cmd.arg("test").args(args).arg("--quiet");

    for (k, v) in env_vars {
        cmd.env(k, v);
    }

    if !cmd.status().expect("cargo test failed").success() {
        exit(1);
    }
}
