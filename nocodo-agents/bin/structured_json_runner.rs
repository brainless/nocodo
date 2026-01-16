use clap::Parser;
use nocodo_agents::config;
use nocodo_agents::factory::AgentFactory;
use nocodo_agents::structured_json::StructuredJsonAgentConfig;
use nocodo_agents::Agent;
use nocodo_llm_sdk::glm::zai::ZaiGlmClient;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "structured-json-runner")]
#[command(about = "Run the Structured JSON agent to generate type-safe JSON responses")]
struct Args {
    /// The user prompt to send to the agent
    #[arg(short, long)]
    prompt: String,

    /// TypeScript type names to include in the prompt
    #[arg(short, long)]
    types: Vec<String>,

    /// Domain description for the agent
    #[arg(short, long, default_value = "Structured data generation")]
    domain: String,

    /// Path to configuration file
    #[arg(short, long)]
    config: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config_path = PathBuf::from(&args.config);
    let config = config::load_config(&config_path)?;
    let zai_config = config::get_zai_config(&config)?;

    let client = Arc::new(ZaiGlmClient::with_coding_plan(
        zai_config.api_key,
        zai_config.coding_plan,
    )?);

    let db_path = config_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
        .join("agent_sessions.db");

    let database = Arc::new(nocodo_agents::database::Database::new(&db_path)?);

    let tool_executor = Arc::new(
        manager_tools::ToolExecutor::new(std::env::current_dir()?)
            .with_max_file_size(10 * 1024 * 1024),
    );

    let factory = AgentFactory::new(client.clone(), database.clone(), tool_executor);

    let type_names = if args.types.is_empty() {
        vec![
            "PMProject".to_string(),
            "Workflow".to_string(),
            "WorkflowStep".to_string(),
        ]
    } else {
        args.types.clone()
    };

    println!("Executing StructuredJsonAgent with types: {:?}", type_names);

    let agent_config = StructuredJsonAgentConfig {
        type_names,
        domain_description: args.domain,
    };

    let agent = factory.create_structured_json_agent(agent_config)?;

    let session_id = database.create_session(
        "structured-json",
        "cli",
        &args.prompt,
        Some(&agent.system_prompt()),
        "structured-json-runner",
        None,
    )?;

    match agent.execute(&args.prompt, session_id).await {
        Ok(result) => {
            println!("\nResult:\n{}", result);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
