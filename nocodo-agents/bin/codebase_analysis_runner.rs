use clap::Parser;
use nocodo_agents::{config, factory::create_codebase_analysis_agent, Agent};
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

    /// Base path for tool execution (file operations are relative to this)
    #[arg(long, default_value = ".")]
    base_path: String,
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

    let config = config::load_config(&args.config)?;
    let zai_config = config::get_zai_config(&config)?;

    let client = ZaiGlmClient::with_coding_plan(zai_config.api_key, zai_config.coding_plan)?;
    let client: Arc<dyn nocodo_llm_sdk::client::LlmClient> = Arc::new(client);

    let base_path = get_base_path(&args.base_path)?;
    // Create tool executor with nocodo-tools (supports more configuration)
    let tool_executor = Arc::new(
        ToolExecutor::new(base_path.clone()).with_max_file_size(10 * 1024 * 1024), // max_file_size: 10MB
    );

    // Create codebase analysis agent
    let (agent, database) = create_codebase_analysis_agent(client, tool_executor);

    println!("Running agent: {}", agent.objective());
    println!("User prompt: {}\n", args.prompt);

    // Create session
    let session_id = database.create_session(
        "codebase-analysis",
        "standalone",
        "standalone",
        Some(&agent.system_prompt()),
        &args.prompt,
        None,
    )?;

    // Execute agent
    let result = agent.execute(&args.prompt, session_id).await?;

    println!("\n--- Agent Result ---\n{}", result);

    Ok(())
}
