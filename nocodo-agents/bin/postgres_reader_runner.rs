use clap::Parser;
use nocodo_agents::{
    config,
    factory::create_postgres_reader_agent,
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

    /// PostgreSQL connection string (postgresql://user:password@host:port/database)
    /// Can also be set via POSTGRES_CONNECTION_STRING environment variable
    #[arg(long)]
    connection_string: Option<String>,
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

    // Get connection string from args or environment variable
    let connection_string = args
        .connection_string
        .or_else(|| std::env::var("POSTGRES_CONNECTION_STRING").ok())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "PostgreSQL connection string must be provided via --connection-string or POSTGRES_CONNECTION_STRING environment variable"
            )
        })?;

    let config = config::load_config(&args.config)?;
    let zai_config = config::get_zai_config(&config)?;

    let client = ZaiGlmClient::with_coding_plan(zai_config.api_key, zai_config.coding_plan)?;
    let client: Arc<dyn nocodo_llm_sdk::client::LlmClient> = Arc::new(client);

    let tool_executor =
        Arc::new(ToolExecutor::new(std::env::current_dir()?).with_max_file_size(10 * 1024 * 1024));

    let storage = Arc::new(nocodo_agents::storage::InMemoryStorage::new());
    let agent = create_postgres_reader_agent(client, tool_executor, connection_string).await?;

    tracing::info!("System prompt:\n{}", agent.system_prompt());

    println!("Running agent: {}", agent.objective());
    println!("User prompt: {}\n", args.prompt);

    // Create session
    let session = Session {
        id: None,
        agent_name: "postgres-reader".to_string(),
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
    let session_id = storage.create_session(session).await?;

    let result = agent.execute(&args.prompt, session_id).await?;

    println!("\n--- Agent Result ---\n{}", result);

    Ok(())
}
