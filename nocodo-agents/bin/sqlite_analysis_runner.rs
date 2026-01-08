use clap::Parser;
use manager_tools::ToolExecutor;
use nocodo_agents::{factory::create_sqlite_analysis_agent, Agent};
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

    /// Path to SQLite database to analyze
    #[arg(long)]
    db_path: String,
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(true)
                .with_target(false),
        )
        .init();

    let args = Args::parse();

    let config = load_config(&args.config)?;

    let zai_api_key =
        get_api_key(&config, "zai_api_key").or_else(|_| get_api_key(&config, "ZAI_API_KEY"))?;

    let coding_plan = get_bool_option(&config, "zai_coding_plan", true);

    let client = ZaiGlmClient::with_coding_plan(zai_api_key, coding_plan)?;
    let client: Arc<dyn nocodo_llm_sdk::client::LlmClient> = Arc::new(client);

    let tool_executor =
        Arc::new(ToolExecutor::new(std::env::current_dir()?).with_max_file_size(10 * 1024 * 1024));

    let (agent, database) =
        create_sqlite_analysis_agent(client, tool_executor, args.db_path).await?;

    tracing::info!("System prompt:\n{}", agent.system_prompt());

    println!("Running agent: {}", agent.objective());
    println!("User prompt: {}\n", args.prompt);

    // For standalone runner, create a dummy session
    let session_id = database.create_session(
        "sqlite-analysis",
        "standalone",
        "standalone",
        Some(&agent.system_prompt()),
        &args.prompt,
        None,
    )?;

    let result = agent.execute(&args.prompt, session_id).await?;

    println!("\n--- Agent Result ---\n{}", result);

    Ok(())
}
