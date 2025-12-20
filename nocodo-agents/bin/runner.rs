use clap::Parser;
use nocodo_agents::{database::Database, factory::{create_agent_with_tools, AgentType}, tools::executor::ToolExecutor};
use nocodo_llm_sdk::glm::zai::ZaiGlmClient;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Agent name to run (e.g., "codebase-analysis")
    #[arg(short, long)]
    agent: String,

    /// User prompt for the agent
    #[arg(short, long)]
    prompt: String,

    /// Path to config file containing API keys
    #[arg(short, long)]
    config: PathBuf,

    /// Path to SQLite database for storing agent sessions
    #[arg(long, default_value = "~/.nocodo-agents/agent.db")]
    database_path: String,

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

fn get_database_path(path_str: &str) -> anyhow::Result<PathBuf> {
    let path = if path_str.starts_with('~') {
        let home = std::env::var("HOME").map_err(|_| anyhow::anyhow!("HOME environment variable not set"))?;
        PathBuf::from(path_str.replace('~', &home))
    } else {
        PathBuf::from(path_str)
    };
    
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    
    Ok(path)
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
    let args = Args::parse();

    // Load config
    let config = load_config(&args.config)?;

    // Get ZAI API key (try lowercase first, then uppercase)
    let zai_api_key = get_api_key(&config, "zai_api_key")
        .or_else(|_| get_api_key(&config, "ZAI_API_KEY"))?;

    // Check if coding plan mode is enabled (default to true)
    let coding_plan = get_bool_option(&config, "zai_coding_plan", true);

    // Create ZAI GLM client with coding plan mode
    let client = ZaiGlmClient::with_coding_plan(zai_api_key, coding_plan)?;
    let client: Arc<dyn nocodo_llm_sdk::client::LlmClient> = Arc::new(client);

    // Parse agent type
    let agent_type = match args.agent.to_lowercase().as_str() {
        "codebase-analysis" | "codebase_analysis" => AgentType::CodebaseAnalysis,
        _ => {
            anyhow::bail!("Unknown agent type: {}. Available: codebase-analysis", args.agent)
        }
    };

    // Initialize database and tool executor
    let database_path = get_database_path(&args.database_path)?;
    let database = Arc::new(Database::new(&database_path)?);

    let base_path = get_base_path(&args.base_path)?;
    let tool_executor = Arc::new(ToolExecutor::new(base_path));

    // Create agent with database and tool executor
    let agent = create_agent_with_tools(agent_type, client, database, tool_executor);

    println!("Running agent: {}", agent.objective());
    println!("User prompt: {}\n", args.prompt);

    // Execute agent
    let result = agent.execute(&args.prompt).await?;

    println!("\n--- Agent Result ---\n{}", result);

    Ok(())
}
