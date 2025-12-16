use std::collections::HashMap;
use std::env;
use std::process::{Command, exit};
use serde::Deserialize;

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

    let (keys, has_coding_plan) = load_config(&args[1]);

    // Determine which provider to use based on preference hierarchy:
    // 1. z.ai/GLM 4.6 (ZAI_API_KEY, optionally with zai_coding_plan flag)
    // 2. zen/GLM 4.6 (no API key required, free)
    // 3. zen/Grok Code Fast 1 (no API key required, free)
    // 4. Anthropic/Claude Sonnet 4.5 (ANTHROPIC_API_KEY)

    let provider = if keys.contains_key("ZAI_API_KEY") {
        if has_coding_plan {
            println!("Using z.ai GLM 4.6 with coding plan");
        } else {
            println!("Using z.ai GLM 4.6");
        }
        "zai_glm"
    } else {
        // Zen GLM is always available (free)
        println!("Using Zen GLM 4.6 (free)");
        "zen_glm"
    };

    // If zen providers are not available for some reason, try Anthropic
    let provider = if provider == "zen_grok" && keys.contains_key("ANTHROPIC_API_KEY") {
        println!("Zen providers not available, using Anthropic Claude Sonnet 4.5");
        "anthropic_claude"
    } else {
        provider
    };

    // Run unit tests (no env vars needed)
    run_test(&[], &HashMap::new());

    // Run integration tests with the selected provider
    let mut test_env = keys.clone();
    test_env.insert("WORKFLOW_PROVIDER".to_string(), provider.to_string());
    
    // Set coding plan flag for zai if enabled
    if provider == "zai_glm" && has_coding_plan {
        test_env.insert("ZAI_CODING_PLAN".to_string(), "true".to_string());
    }

    run_test(&["--test", "workflow_integration", "--", "--include-ignored"], &test_env);
}

fn load_config(path: &str) -> (HashMap<String, String>, bool) {
    let content = std::fs::read_to_string(path).unwrap_or_default();
    let config: Config = toml::from_str(&content).unwrap_or(Config {
        api_keys: HashMap::new(),
    });

    // Check for zai_coding_plan boolean flag
    let has_coding_plan = config
        .api_keys
        .get("zai_coding_plan")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Convert lowercase keys to uppercase environment variable format
    // Only include string values (API keys), skip booleans and other types
    let keys = config
        .api_keys
        .into_iter()
        .filter_map(|(k, v)| v.as_str().map(|s| (k.to_uppercase(), s.to_string())))
        .collect();

    (keys, has_coding_plan)
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
