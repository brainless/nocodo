use nocodo_llm_sdk::client::LlmClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
}

/// Returns a list of all supported agents
/// Currently only SQLite agent is enabled
pub fn list_supported_agents() -> Vec<AgentInfo> {
    vec![AgentInfo {
        id: "sqlite".to_string(),
        name: "SQLite Analysis Agent".to_string(),
        description: "Agent for analyzing SQLite databases and running SQL queries".to_string(),
        enabled: true,
    }]
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
