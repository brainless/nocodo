use clap::Parser;
use nocodo_agents::storage::InMemoryStorage;
use nocodo_agents::{
    config, pdftotext::PdfToTextAgent, Agent, AgentStorage, Session, SessionStatus,
};
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::glm::zai::ZaiGlmClient;
use std::path::PathBuf;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the PDF file to process
    #[arg(long)]
    pdf: PathBuf,

    /// User prompt for the agent
    #[arg(long)]
    prompt: String,

    /// Path to config file containing API keys
    #[arg(short, long)]
    config: PathBuf,

    /// Optional: Allowed working directories (comma-separated). Defaults to parent directory of PDF file
    #[arg(long, value_delimiter = ',')]
    allowed_dirs: Option<Vec<String>>,
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
    let client: Arc<dyn LlmClient> = Arc::new(client);

    let storage = Arc::new(InMemoryStorage::new());

    let pdf_path = args.pdf.canonicalize()?;

    let parent_dir = pdf_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("PDF file has no parent directory"))?
        .to_path_buf();

    let allowed_working_dirs = args
        .allowed_dirs
        .unwrap_or_else(|| vec![parent_dir.to_string_lossy().to_string()]);

    let agent = PdfToTextAgent::new(
        client.clone(),
        storage.clone(),
        pdf_path.clone(),
        Some(allowed_working_dirs.clone()),
    )?;

    println!("Running agent: {}", agent.objective());
    println!("PDF file: {}", pdf_path.display());
    println!("Allowed working dirs: {:?}", allowed_working_dirs);
    println!("User prompt: {}\n", args.prompt);

    let session = Session {
        id: None,
        agent_name: "pdftotext".to_string(),
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
