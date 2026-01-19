use clap::Parser;
use nocodo_agents::config;
use nocodo_agents::settings_management::create_settings_management_agent;
use nocodo_agents::Agent;
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
    // Use static schemas to avoid circular dependency (can't instantiate SqliteAnalysisAgent
    // without db_path, which is what we're trying to collect)
    let agent_schemas =
        nocodo_agents::sqlite_analysis::SqliteAnalysisAgent::static_settings_schema()
            .map(|schema| vec![schema])
            .unwrap_or_default();

    let (agent, database) =
        create_settings_management_agent(client, args.settings_file.clone(), agent_schemas)?;

    tracing::debug!("System prompt:\n{}", agent.system_prompt());

    println!("Running agent: {}", agent.objective());
    println!("User prompt: {}\n", args.prompt);

    let session_id = database.create_session(
        "settings-management",
        "standalone",
        "standalone",
        Some(&agent.system_prompt()),
        &args.prompt,
        None,
    )?;

    let result = agent.execute(&args.prompt, session_id).await?;

    println!("\n--- Agent Result ---\n{}", result);
    println!("\nSettings saved to: {}", args.settings_file.display());

    Ok(())
}
