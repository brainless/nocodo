use clap::Parser;
use nocodo_agents::{
    config,
    factory::create_user_clarification_agent,
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
    /// User prompt for the agent (default: "Build me a website")
    #[arg(
        short,
        long,
        default_value = "I want to create a simple workflow to handle orders coming from emails.
My b2b customers email me their requirements, mostly plain text with line item and counts.
I check and block inventory in our inventory system, generate an invoice and email them.
They pay (bank transfer). After I get notified from bank, I start shipment process.
I want a workflow for the order handling part, checking with our inventory system, generate invoice"
    )]
    prompt: String,

    /// Path to config file containing API keys
    #[arg(short, long)]
    config: PathBuf,
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

    let agent = create_user_clarification_agent(client);

    tracing::debug!("System prompt:\n{}", agent.system_prompt());

    println!("Running agent: {}", agent.objective());
    println!("User prompt: {}\n", args.prompt);

    // Create session
    let storage = Arc::new(nocodo_agents::storage::InMemoryStorage::new());
    let session = Session {
        id: None,
        agent_name: "user-clarification".to_string(),
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
