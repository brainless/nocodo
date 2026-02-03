use clap::Parser;
use nocodo_agents::{
    config,
    factory::create_settings_management_agent,
    storage::AgentStorage,
    types::{Session, SessionStatus},
    Agent,
};
use std::path::PathBuf;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// User prompt describing the workflow that needs settings
    #[arg(short, long, default_value = "I want to analyze a SQLite database")]
    prompt: String,

    /// Path to config file containing API keys
    #[arg(short, long)]
    config: PathBuf,

    /// Path to settings file where collected settings will be saved
    #[arg(short, long, default_value = "workflow_settings.toml")]
    settings_file: PathBuf,
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

    let client = Arc::new(nocodo_llm_sdk::glm::zai::ZaiGlmClient::with_coding_plan(
        zai_config.api_key,
        zai_config.coding_plan,
    )?);

    // Collect settings schemas from available agents
    // Use static schemas to avoid circular dependency (can't instantiate SqliteReaderAgent
    // without db_path, which is what we're trying to collect)
    let agent_schemas = nocodo_agents::sqlite_reader::SqliteReaderAgent::<
        nocodo_agents::storage::InMemoryStorage,
    >::static_settings_schema()
    .map(|schema| vec![schema])
    .unwrap_or_default();

    let agent = create_settings_management_agent(client, args.settings_file.clone(), agent_schemas);

    tracing::debug!("System prompt:\n{}", agent.system_prompt());

    println!("Running agent: {}", agent.objective());
    println!("User prompt: {}\n", args.prompt);

    // Create session
    let storage = Arc::new(nocodo_agents::storage::InMemoryStorage::new());
    let session = Session {
        id: None,
        agent_name: "settings-management".to_string(),
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
    println!("\nSettings saved to: {}", args.settings_file.display());

    Ok(())
}
