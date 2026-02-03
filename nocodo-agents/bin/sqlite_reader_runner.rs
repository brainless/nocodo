use clap::Parser;
use nocodo_agents::{
    config,
    factory::create_sqlite_reader_agent,
    storage::AgentStorage,
    types::{Session, SessionStatus},
    Agent,
};
use nocodo_llm_sdk::glm::zai::ZaiGlmClient;
use nocodo_tools::ToolExecutor;
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

    let config = config::load_config(&args.config)?;
    let zai_config = config::get_zai_config(&config)?;

    let client = ZaiGlmClient::with_coding_plan(zai_config.api_key, zai_config.coding_plan)?;
    let client: Arc<dyn nocodo_llm_sdk::client::LlmClient> = Arc::new(client);

    let tool_executor =
        Arc::new(ToolExecutor::new(std::env::current_dir()?).with_max_file_size(10 * 1024 * 1024));

    let storage = Arc::new(nocodo_agents::storage::InMemoryStorage::new());
    let agent = create_sqlite_reader_agent(client, tool_executor, args.db_path).await?;

    tracing::info!("System prompt:\n{}", agent.system_prompt());

    println!("Running agent: {}", agent.objective());
    println!("User prompt: {}\n", args.prompt);

    // Create session
    let session = Session {
        id: None,
        agent_name: "sqlite-reader".to_string(),
        provider: "standalone".to_string(),
        model: "standalone".to_string(),
        system_prompt: Some(agent.system_prompt()),
        user_prompt: args.prompt.clone(),
        config: serde_json::json!({}),
        status: SessionStatus::Running,
        started_at: chrono::Utc::now().timestamp(),
        ended_at: None,
        result: None,
        error: None,
    };
    let session_id_str = storage.create_session(session).await?;

    // Parse session ID as i64 for agent.execute()
    let session_id = session_id_str.parse::<i64>().unwrap_or_else(|_| 1);

    let result = agent.execute(&args.prompt, session_id).await?;

    println!("\n--- Agent Result ---\n{}", result);

    Ok(())
}
