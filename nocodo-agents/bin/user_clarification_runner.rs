use clap::Parser;
use nocodo_agents::config;
use nocodo_agents::user_clarification::create_user_clarification_agent;
use nocodo_agents::Agent;
use std::path::PathBuf;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// User prompt for the agent (default: "Build me a website")
    #[arg(short, long, default_value = "Build me a website")]
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

    let (agent, database) = create_user_clarification_agent(client)?;

    tracing::debug!("System prompt:\n{}", agent.system_prompt());

    println!("Running agent: {}", agent.objective());
    println!("User prompt: {}\n", args.prompt);

    let session_id = database.create_session(
        "user-clarification",
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
