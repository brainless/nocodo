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
            id: "requirements-gathering".to_string(),
            name: "Requirements Gathering Agent".to_string(),
            description:
                "Agent for analyzing user requests and determining if clarification questions are needed"
                    .to_string(),
            enabled: true,
        },
        AgentInfo {
            id: "settings-management".to_string(),
            name: "Settings Management Agent".to_string(),
            description:
                "Agent for collecting and managing settings required for workflow automation"
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
        nocodo_tools::ToolExecutor::new(std::env::current_dir()?)
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
        nocodo_tools::ToolExecutor::new(std::env::current_dir()?)
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
        nocodo_tools::ToolExecutor::new(std::env::current_dir()?)
            .with_max_file_size(10 * 1024 * 1024),
    );

    let agent = nocodo_agents::requirements_gathering::UserClarificationAgent::new(
        llm_client.clone(),
        database.clone(),
        tool_executor,
    );

    Ok(agent)
}

/// Creates a Settings Management agent
///
/// # Arguments
///
/// * `llm_client` - The LLM client to use for the agent
/// * `database` - Shared database for session persistence
/// * `settings_file_path` - Path to the TOML settings file
/// * `agent_schemas` - List of agent settings schemas
///
/// # Returns
///
/// A Settings Management agent instance
pub fn create_settings_management_agent(
    llm_client: &Arc<dyn LlmClient>,
    database: &Arc<nocodo_agents::database::Database>,
    settings_file_path: &str,
    agent_schemas: Vec<nocodo_agents::AgentSettingsSchema>,
) -> anyhow::Result<nocodo_agents::settings_management::SettingsManagementAgent> {
    let tool_executor = Arc::new(
        nocodo_tools::ToolExecutor::new(std::env::current_dir()?)
            .with_max_file_size(10 * 1024 * 1024),
    );

    let agent = nocodo_agents::settings_management::SettingsManagementAgent::new(
        llm_client.clone(),
        database.clone(),
        tool_executor,
        std::path::PathBuf::from(settings_file_path),
        agent_schemas,
    );

    Ok(agent)
}
