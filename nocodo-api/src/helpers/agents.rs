use nocodo_llm_sdk::client::LlmClient;
use shared_types::AgentInfo;
use std::sync::Arc;

/// Returns a list of all supported agents
pub fn list_supported_agents() -> Vec<AgentInfo> {
    vec![
        AgentInfo {
            id: "sqlite".to_string(),
            name: "SQLite Analysis Agent".to_string(),
            description: "Agent for analyzing SQLite databases and running SQL queries".to_string(),
            enabled: true,
        },
        AgentInfo {
            id: "codebase-analysis".to_string(),
            name: "Codebase Analysis Agent".to_string(),
            description:
                "Agent for analyzing codebase structure and identifying architectural patterns"
                    .to_string(),
            enabled: true,
        },
        AgentInfo {
            id: "tesseract".to_string(),
            name: "Tesseract OCR Agent".to_string(),
            description:
                "Agent for extracting text from images using Tesseract OCR with AI-powered cleaning"
                    .to_string(),
            enabled: true,
        },
        AgentInfo {
            id: "workflow-creation".to_string(),
            name: "Workflow Creation Agent".to_string(),
            description:
                "Agent for generating workflow and workflow step structures from natural language descriptions"
                    .to_string(),
            enabled: true,
        },
        AgentInfo {
            id: "user-clarification".to_string(),
            name: "User Clarification Agent".to_string(),
            description:
                "Agent for analyzing user requests and determining if clarification questions are needed"
                    .to_string(),
            enabled: true,
        },
    ]
}

/// Creates a SQLite analysis agent using the shared database
///
/// # Arguments
///
/// * `llm_client` - The LLM client to use for the agent
/// * `database` - Shared database for session persistence
/// * `db_path` - Path to the SQLite database to analyze
///
/// # Returns
///
/// A SQLite analysis agent instance
pub async fn create_sqlite_agent(
    llm_client: &Arc<dyn LlmClient>,
    database: &Arc<nocodo_agents::database::Database>,
    db_path: &str,
) -> anyhow::Result<nocodo_agents::sqlite_analysis::SqliteAnalysisAgent> {
    let tool_executor = Arc::new(
        manager_tools::ToolExecutor::new(std::env::current_dir()?)
            .with_max_file_size(10 * 1024 * 1024),
    );

    let agent = nocodo_agents::sqlite_analysis::SqliteAnalysisAgent::new(
        llm_client.clone(),
        database.clone(),
        tool_executor,
        db_path.to_string(),
    )
    .await?;

    Ok(agent)
}

/// Creates a Tesseract OCR agent
///
/// # Arguments
///
/// * `llm_client` - The LLM client to use for the agent
/// * `database` - Shared database for session persistence
/// * `image_path` - Path to the image file to process
///
/// # Returns
///
/// A Tesseract OCR agent instance
pub async fn create_tesseract_agent(
    llm_client: &Arc<dyn LlmClient>,
    database: &Arc<nocodo_agents::database::Database>,
    image_path: &str,
) -> anyhow::Result<nocodo_agents::tesseract::TesseractAgent> {
    let agent = nocodo_agents::tesseract::TesseractAgent::new(
        llm_client.clone(),
        database.clone(),
        std::path::PathBuf::from(image_path),
    )?;

    Ok(agent)
}

/// Creates a Structured JSON agent
///
/// # Arguments
///
/// * `llm_client` - The LLM client to use for the agent
/// * `database` - Shared database for session persistence
/// * `type_names` - List of TypeScript type names to use for validation
/// * `domain_description` - Description of the domain context
///
/// # Returns
///
/// A Structured JSON agent instance
pub fn create_structured_json_agent(
    llm_client: &Arc<dyn LlmClient>,
    database: &Arc<nocodo_agents::database::Database>,
    type_names: Vec<String>,
    domain_description: String,
) -> anyhow::Result<nocodo_agents::structured_json::StructuredJsonAgent> {
    let tool_executor = Arc::new(
        manager_tools::ToolExecutor::new(std::env::current_dir()?)
            .with_max_file_size(10 * 1024 * 1024),
    );

    let config = nocodo_agents::structured_json::StructuredJsonAgentConfig {
        type_names,
        domain_description,
    };

    let agent = nocodo_agents::structured_json::StructuredJsonAgent::new(
        llm_client.clone(),
        database.clone(),
        tool_executor,
        config,
    )?;

    Ok(agent)
}

/// Creates a User Clarification agent
///
/// # Arguments
///
/// * `llm_client` - The LLM client to use for the agent
/// * `database` - Shared database for session persistence
///
/// # Returns
///
/// A User Clarification agent instance
pub fn create_user_clarification_agent(
    llm_client: &Arc<dyn LlmClient>,
    database: &Arc<nocodo_agents::database::Database>,
) -> anyhow::Result<nocodo_agents::requirements_gathering::UserClarificationAgent> {
    let tool_executor = Arc::new(
        manager_tools::ToolExecutor::new(std::env::current_dir()?)
            .with_max_file_size(10 * 1024 * 1024),
    );

    let agent = nocodo_agents::requirements_gathering::UserClarificationAgent::new(
        llm_client.clone(),
        database.clone(),
        tool_executor,
    );

    Ok(agent)
}

/// Returns the path to the nocodo-api database based on the operating system
///
/// # Returns
///
/// A PathBuf pointing to the API database file
///
/// # Platform-specific paths
///
/// - **macOS**: `~/Library/Application Support/nocodo/nocodo-api.db`
/// - **Linux**: `~/.local/share/nocodo/nocodo-api.db`
/// - **Windows**: `{FOLDERPATH}\nocodo\nocodo-api.db` (where FOLDERPATH is typically `C:\Users\<username>\AppData\Local`)
pub fn get_api_db_path() -> anyhow::Result<std::path::PathBuf> {
    let home =
        home::home_dir().ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;

    let db_path = if cfg!(target_os = "macos") {
        home.join("Library/Application Support/nocodo/nocodo-api.db")
    } else if cfg!(target_os = "linux") {
        home.join(".local/share/nocodo/nocodo-api.db")
    } else if cfg!(windows) {
        home.join("AppData/Local/nocodo-api.db")
    } else {
        anyhow::bail!("Unsupported operating system");
    };

    Ok(db_path)
}
