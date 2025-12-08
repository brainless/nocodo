use std::collections::HashMap;
use std::env;
use std::process::{Command, exit};
use serde::Deserialize;

#[derive(Deserialize)]
struct Config {
    #[serde(default)]
    api_keys: HashMap<String, String>,
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
        ("zen_grok_integration", ""),
        ("zen_glm_integration", ""),
    ];

    for (test, key) in tests {
        if key.is_empty() || keys.contains_key(key) {
            run_test(&["--test", test], &keys);
        }
    }
}

fn load_keys(path: &str) -> HashMap<String, String> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| toml::from_str::<Config>(&s).ok())
        .map(|c| c.api_keys)
        .unwrap_or_default()
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
