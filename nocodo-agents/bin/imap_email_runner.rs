use clap::Parser;
use nocodo_agents::{config, database::Database, imap_email::ImapEmailAgent, Agent};
use nocodo_llm_sdk::glm::zai::ZaiGlmClient;
use nocodo_tools::ToolExecutor;
use std::collections::HashMap;
use std::io::{self, Write};
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

    /// IMAP server hostname (e.g., imap.gmail.com)
    #[arg(long)]
    host: String,

    /// IMAP server port (default: 993 for TLS)
    #[arg(long, default_value = "993")]
    port: u16,

    /// Email address for IMAP login
    #[arg(long)]
    username: String,

    /// Interactive mode - allows multiple queries in a session
    #[arg(short, long)]
    interactive: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing subscriber
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(
            tracing_subscriber::fmt::layer()
                .with_ansi(true)
                .with_target(false),
        )
        .init();

    let args = Args::parse();

    // Prompt for password securely (not echoed to terminal)
    print!("Enter IMAP password: ");
    io::stdout().flush()?;
    let password = rpassword::read_password()?;

    if password.is_empty() {
        anyhow::bail!("Password cannot be empty");
    }

    println!("\n🔐 Password received (hidden for security)\n");

    // Load LLM config
    let config = config::load_config(&args.config)?;
    let zai_config = config::get_zai_config(&config)?;

    let client = ZaiGlmClient::with_coding_plan(zai_config.api_key, zai_config.coding_plan)?;
    let client: Arc<dyn nocodo_llm_sdk::client::LlmClient> = Arc::new(client);

    // Create tool executor
    let tool_executor =
        Arc::new(ToolExecutor::new(std::env::current_dir()?).with_max_file_size(10 * 1024 * 1024));

    // Create database for session management
    let database = Arc::new(Database::new(&PathBuf::from(":memory:"))?);

    // Test IMAP connection before creating agent/loading LLM
    println!("🔍 Testing IMAP connection...");
    match test_imap_connection(&args.host, args.port, &args.username, &password).await {
        Ok(mailbox_count) => {
            println!(
                "✅ Connection successful! Found {} mailboxes\n",
                mailbox_count
            );
        }
        Err(e) => {
            eprintln!("❌ IMAP connection test failed: {}\n", e);
            eprintln!("Common issues:");
            eprintln!("  - Wrong password (try app-specific password for Gmail/iCloud)");
            eprintln!("  - IMAP not enabled on your email account");
            eprintln!("  - Wrong server hostname or port");
            eprintln!("  - Network/firewall blocking connection");
            std::process::exit(1);
        }
    }

    // Create agent with settings
    let mut settings = HashMap::new();
    settings.insert("host".to_string(), args.host.clone());
    settings.insert("port".to_string(), args.port.to_string());
    settings.insert("username".to_string(), args.username.clone());
    settings.insert("password".to_string(), password);

    let agent = ImapEmailAgent::from_settings(
        client.clone(),
        database.clone(),
        tool_executor.clone(),
        &settings,
    )?;

    println!("🚀 Running IMAP Email Agent");
    println!("📧 IMAP Server: {}:{}", args.host, args.port);
    println!("👤 Username: {}", args.username);
    println!("🎯 Objective: {}\n", agent.objective());

    if args.interactive {
        // Interactive mode - multiple queries in same session
        run_interactive_mode(&agent, &database, &args.prompt).await?;
    } else {
        // Single query mode
        run_single_query(&agent, &database, &args.prompt).await?;
    }

    Ok(())
}

async fn run_single_query(
    agent: &ImapEmailAgent,
    database: &Arc<Database>,
    prompt: &str,
) -> anyhow::Result<()> {
    println!("💬 User prompt: {}\n", prompt);
    println!("⏳ Processing...\n");

    // Create session
    let session_id = database.create_session(
        "imap-email",
        "standalone",
        "standalone",
        Some(&agent.system_prompt()),
        prompt,
        None,
    )?;

    // Execute agent
    let result = agent.execute(prompt, session_id).await?;

    println!("\n--- 📬 Agent Result ---\n{}", result);

    Ok(())
}

async fn run_interactive_mode(
    agent: &ImapEmailAgent,
    database: &Arc<Database>,
    initial_prompt: &str,
) -> anyhow::Result<()> {
    // Create a single session for the entire interaction
    let session_id = database.create_session(
        "imap-email",
        "standalone",
        "standalone",
        Some(&agent.system_prompt()),
        initial_prompt,
        None,
    )?;

    println!("🔄 Interactive mode enabled - session ID: {}", session_id);
    println!("💡 Type your queries. Type 'quit' or 'exit' to end the session.\n");

    // Process initial prompt
    if !initial_prompt.is_empty() {
        println!("💬 Initial query: {}\n", initial_prompt);
        println!("⏳ Processing...\n");

        let result = agent.execute(initial_prompt, session_id).await?;
        println!("\n--- 📬 Agent Result ---\n{}\n", result);
    }

    // Interactive loop
    loop {
        print!("📧 Your query> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        if input == "quit" || input == "exit" {
            println!("👋 Ending session. Goodbye!");
            break;
        }

        println!("\n⏳ Processing...\n");

        match agent.execute(input, session_id).await {
            Ok(result) => {
                println!("\n--- 📬 Agent Result ---\n{}\n", result);
            }
            Err(e) => {
                println!("\n❌ Error: {:?}\n", e);
            }
        }
    }

    Ok(())
}

/// Test IMAP connection before proceeding with agent
async fn test_imap_connection(
    host: &str,
    port: u16,
    username: &str,
    password: &str,
) -> anyhow::Result<usize> {
    use rustls_connector::RustlsConnector;
    use std::net::TcpStream;

    // Establish TCP connection
    let tcp_stream = TcpStream::connect((host, port))
        .map_err(|e| anyhow::anyhow!("Failed to connect to {}:{} - {}", host, port, e))?;

    // Wrap with TLS
    let tls_connector = RustlsConnector::new_with_native_certs()
        .map_err(|e| anyhow::anyhow!("Failed to create TLS connector: {}", e))?;

    let tls_stream = tls_connector
        .connect(host, tcp_stream)
        .map_err(|e| anyhow::anyhow!("TLS handshake failed: {}", e))?;

    // Create IMAP client and login
    let client = imap::Client::new(tls_stream);

    let mut session = client
        .login(username, password)
        .map_err(|e| anyhow::anyhow!("Authentication failed: {}", e.0))?;

    // Try to list mailboxes as a connection test
    let mailboxes = session
        .list(None, Some("*"))
        .map_err(|e| anyhow::anyhow!("Failed to list mailboxes: {}", e))?;

    let count = mailboxes.len();

    // Logout
    let _ = session.logout();

    Ok(count)
}
