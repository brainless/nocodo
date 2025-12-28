use clap::Parser;
use manager_tools::ToolExecutor;
use nocodo_agents::{
    Agent,
    factory::create_codebase_analysis_agent,
};
use nocodo_llm_sdk::glm::zai::ZaiGlmClient;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// User prompt for the agent
    #[arg(short, long)]
    prompt: String,

    /// Path to config file containing API keys
    #[arg(short, long)]
    config: PathBuf,

    /// Base path for tool execution (file operations are relative to this)
    #[arg(long, default_value = ".")]
    base_path: String,
}

#[derive(Deserialize)]
struct Config {
    #[serde(default)]
    api_keys: HashMap<String, toml::Value>,
}

fn load_config(path: &PathBuf) -> anyhow::Result<Config> {
    let content = std::fs::read_to_string(path)?;
    let config: Config = toml::from_str(&content)?;
    Ok(config)
}

fn get_api_key(config: &Config, key_name: &str) -> anyhow::Result<String> {
    config
        .api_keys
        .get(key_name)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("API key '{}' not found in config", key_name))
}

fn get_bool_option(config: &Config, key_name: &str, default: bool) -> bool {
    config
        .api_keys
        .get(key_name)
        .and_then(|v| v.as_bool())
        .unwrap_or(default)
}

fn get_base_path(path_str: &str) -> anyhow::Result<PathBuf> {
    let path = if path_str == "." {
        std::env::current_dir()?
    } else {
        PathBuf::from(path_str)
    };

    if !path.exists() {
        std::fs::create_dir_all(&path)?;
    }

    Ok(path)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing subscriber with RUST_LOG env var support
    // Default to "info" level, but allow override via RUST_LOG
    // Example: RUST_LOG=debug cargo run ...
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(true) // Enable ANSI colors
                .with_target(false), // Hide target (module path) for cleaner output
        )
        .init();

    let args = Args::parse();

    // Load config
    let config = load_config(&args.config)?;

    // Get ZAI API key (try lowercase first, then uppercase)
    let zai_api_key =
        get_api_key(&config, "zai_api_key").or_else(|_| get_api_key(&config, "ZAI_API_KEY"))?;

    // Check if coding plan mode is enabled (default to true)
    let coding_plan = get_bool_option(&config, "zai_coding_plan", true);

    // Create ZAI GLM client with coding plan mode
    let client = ZaiGlmClient::with_coding_plan(zai_api_key, coding_plan)?;
    let client: Arc<dyn nocodo_llm_sdk::client::LlmClient> = Arc::new(client);

    let base_path = get_base_path(&args.base_path)?;
    // Create tool executor with manager-tools (supports more configuration)
    let tool_executor = Arc::new(
        ToolExecutor::new(base_path.clone()).with_max_file_size(10 * 1024 * 1024), // max_file_size: 10MB
    );

    // Create codebase analysis agent
    let agent = create_codebase_analysis_agent(client, tool_executor);

    println!("Running agent: {}", agent.objective());
    println!("User prompt: {}\n", args.prompt);

    // Execute agent
    let result = agent.execute(&args.prompt).await?;

    println!("\n--- Agent Result ---\n{}", result);

    Ok(())
}
