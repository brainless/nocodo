use crate::codebase_analysis::CodebaseAnalysisAgent;
use crate::database::Database;
use crate::requirements_gathering::UserClarificationAgent;
use crate::sqlite_analysis::SqliteAnalysisAgent;
use crate::structured_json::StructuredJsonAgent;
use crate::tesseract::TesseractAgent;
use crate::Agent;
use manager_tools::ToolExecutor;
use nocodo_llm_sdk::client::LlmClient;
use std::sync::Arc;

/// Enum representing the available agent types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentType {
    /// Agent for analyzing codebase structure and architecture
    CodebaseAnalysis,
    /// Agent for extracting text from images using Tesseract OCR
    Tesseract,
    /// Agent for generating structured JSON conforming to TypeScript types
    StructuredJson,
    /// Agent for analyzing user requests and determining if clarification is needed
    UserClarification,
}

/// Factory for creating AI agents with shared dependencies
pub struct AgentFactory {
    llm_client: Arc<dyn LlmClient>,
    database: Arc<Database>,
    tool_executor: Arc<ToolExecutor>,
}

impl AgentFactory {
    /// Create a new AgentFactory with the given dependencies
    pub fn new(
        llm_client: Arc<dyn LlmClient>,
        database: Arc<Database>,
        tool_executor: Arc<ToolExecutor>,
    ) -> Self {
        Self {
            llm_client,
            database,
            tool_executor,
        }
    }

    /// Create a CodebaseAnalysisAgent
    pub fn create_codebase_analysis_agent(&self) -> CodebaseAnalysisAgent {
        CodebaseAnalysisAgent::new(
            self.llm_client.clone(),
            self.database.clone(),
            self.tool_executor.clone(),
        )
    }

    /// Create a TesseractAgent for OCR tasks
    ///
    /// # Arguments
    /// * `base_path` - Working directory for file operations
    ///
    /// # Examples
    /// ```rust
    /// let factory = AgentFactory::new(/* config */)?;
    /// let agent = factory.create_tesseract_agent(PathBuf::from("/path/to/images"))?;
    /// ```
    pub fn create_tesseract_agent(
        &self,
        base_path: std::path::PathBuf,
    ) -> anyhow::Result<TesseractAgent> {
        TesseractAgent::new(self.llm_client.clone(), self.database.clone(), base_path)
    }

    /// Create a StructuredJsonAgent for generating type-safe JSON
    ///
    /// # Arguments
    /// * `type_names` - List of TypeScript type names to include in the prompt
    /// * `domain_description` - Description of the domain for the agent
    ///
    /// # Examples
    /// ```rust
    /// let factory = AgentFactory::new(/* config */)?;
    /// let config = nocodo_agents::structured_json::StructuredJsonAgentConfig {
    ///     type_names: vec!["PMProject".to_string(), "Workflow".to_string()],
    ///     domain_description: "Project management".to_string(),
    /// };
    /// let agent = factory.create_structured_json_agent(config)?;
    /// ```
    pub fn create_structured_json_agent(
        &self,
        config: crate::structured_json::StructuredJsonAgentConfig,
    ) -> anyhow::Result<StructuredJsonAgent> {
        StructuredJsonAgent::new(
            self.llm_client.clone(),
            self.database.clone(),
            self.tool_executor.clone(),
            config,
        )
    }

    /// Create a UserClarificationAgent for analyzing user requests
    ///
    /// This agent determines if a user's request needs clarification
    /// before proceeding with task.
    pub fn create_user_clarification_agent(&self) -> UserClarificationAgent {
        UserClarificationAgent::new(
            self.llm_client.clone(),
            self.database.clone(),
            self.tool_executor.clone(),
        )
    }
}

/// Factory function to create an agent of the specified type
///
/// # Arguments
///
/// * `agent_type` - The type of agent to create
/// * `client` - The LLM client to use for the agent
///
/// # Returns
///
/// A boxed trait object implementing the Agent trait
///
/// # Example
///
/// ```no_run
/// use nocodo_agents::factory::{AgentType, create_agent};
/// use nocodo_llm_sdk::claude::ClaudeClient;
/// use std::sync::Arc;
///
/// # async fn example() -> anyhow::Result<()> {
/// let client = Arc::new(ClaudeClient::new("api-key")?);
/// let agent = create_agent(AgentType::CodebaseAnalysis, client);
///
/// println!("Agent objective: {}", agent.objective());
/// let session_id = database.create_session("codebase-analysis", "example", "example", Some(&agent.system_prompt()), "Analyze this codebase", None)?;
/// let result = agent.execute("Analyze this codebase", session_id).await?;
/// # Ok(())
/// # }
/// ```
pub fn create_agent(agent_type: AgentType, client: Arc<dyn LlmClient>) -> Box<dyn Agent> {
    // This is a legacy function - for now create dummy components
    let database = Arc::new(Database::new(&std::path::PathBuf::from(":memory:")).unwrap());
    let tool_executor = Arc::new(
        ToolExecutor::new(std::env::current_dir().unwrap()).with_max_file_size(10 * 1024 * 1024), // 10MB
    );

    create_agent_with_tools(agent_type, client, database, tool_executor)
}

/// Factory function to create an agent with database and tool executor support
///
/// # Arguments
///
/// * `agent_type` - The type of agent to create
/// * `client` - The LLM client to use for agent
/// * `database` - Database for session persistence
/// * `tool_executor` - Tool executor for running tools
///
/// # Returns
///
/// A boxed trait object implementing the Agent trait
pub fn create_agent_with_tools(
    agent_type: AgentType,
    client: Arc<dyn LlmClient>,
    database: Arc<Database>,
    tool_executor: Arc<ToolExecutor>,
) -> Box<dyn Agent> {
    match agent_type {
        AgentType::CodebaseAnalysis => {
            Box::new(CodebaseAnalysisAgent::new(client, database, tool_executor))
        }
        AgentType::Tesseract => {
            // For Tesseract, we need a specific base path. Use current directory as default
            let base_path = std::env::current_dir().unwrap_or_default();
            Box::new(TesseractAgent::new(client, database, base_path).unwrap())
        }
        AgentType::StructuredJson => {
            // For StructuredJson, use default types
            let config = crate::structured_json::StructuredJsonAgentConfig {
                type_names: vec![
                    "PMProject".to_string(),
                    "Workflow".to_string(),
                    "WorkflowStep".to_string(),
                ],
                domain_description: "Structured data generation".to_string(),
            };
            Box::new(StructuredJsonAgent::new(client, database, tool_executor, config).unwrap())
        }
        AgentType::UserClarification => {
            Box::new(UserClarificationAgent::new(client, database, tool_executor))
        }
    }
}

/// Create a CodebaseAnalysisAgent with tool executor support
///
/// Uses an in-memory database by default for session persistence
///
/// # Arguments
///
/// * `client` - The LLM client to use for the agent
/// * `tool_executor` - Tool executor for running tools
///
/// # Returns
///
/// A CodebaseAnalysisAgent instance
pub fn create_codebase_analysis_agent(
    client: Arc<dyn LlmClient>,
    tool_executor: Arc<ToolExecutor>,
) -> (CodebaseAnalysisAgent, Arc<Database>) {
    let database = Arc::new(Database::new(&std::path::PathBuf::from(":memory:")).unwrap());
    let agent = CodebaseAnalysisAgent::new(client, database.clone(), tool_executor);
    (agent, database)
}

/// Create a SqliteAnalysisAgent with tool executor support
///
/// Uses an in-memory database by default for session persistence
///
/// # Arguments
///
/// * `client` - The LLM client to use for the agent
/// * `tool_executor` - Tool executor for running tools
/// * `db_path` - Path to the SQLite database to analyze
///
/// # Returns
///
/// A SqliteAnalysisAgent instance
pub async fn create_sqlite_analysis_agent(
    client: Arc<dyn LlmClient>,
    tool_executor: Arc<ToolExecutor>,
    db_path: String,
) -> anyhow::Result<(SqliteAnalysisAgent, Arc<Database>)> {
    let database = Arc::new(Database::new(&std::path::PathBuf::from(":memory:"))?);
    let agent = SqliteAnalysisAgent::new(client, database.clone(), tool_executor, db_path).await?;
    Ok((agent, database))
}

/// Create a TesseractAgent with tool executor support
///
/// Uses an in-memory database by default for session persistence
///
/// # Arguments
///
/// * `client` - The LLM client to use for the agent
/// * `base_path` - Working directory for file operations
///
/// # Returns
///
/// A TesseractAgent instance
pub fn create_tesseract_agent(
    client: Arc<dyn LlmClient>,
    base_path: std::path::PathBuf,
) -> anyhow::Result<(TesseractAgent, Arc<Database>)> {
    let database = Arc::new(Database::new(&std::path::PathBuf::from(":memory:"))?);
    let agent = TesseractAgent::new(client, database.clone(), base_path)?;
    Ok((agent, database))
}

/// Create a UserClarificationAgent with tool executor support
///
/// Uses an in-memory database by default for session persistence
///
/// # Arguments
///
/// * `client` - The LLM client to use for the agent
///
/// # Returns
///
/// A UserClarificationAgent instance and its database
pub fn create_user_clarification_agent(
    client: Arc<dyn LlmClient>,
) -> anyhow::Result<(UserClarificationAgent, Arc<Database>)> {
    let database = Arc::new(Database::new(&std::path::PathBuf::from(":memory:"))?);
    let tool_executor = Arc::new(ToolExecutor::new(std::path::PathBuf::from(".")));
    let agent = UserClarificationAgent::new(client, database.clone(), tool_executor);
    Ok((agent, database))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nocodo_llm_sdk::client::LlmClient;
    use nocodo_llm_sdk::error::LlmError;
    use nocodo_llm_sdk::types::{CompletionRequest, CompletionResponse, ContentBlock, Role, Usage};

    struct MockLlmClient;

    #[async_trait::async_trait]
    impl LlmClient for MockLlmClient {
        async fn complete(
            &self,
            _request: CompletionRequest,
        ) -> Result<CompletionResponse, LlmError> {
            Ok(CompletionResponse {
                content: vec![ContentBlock::Text {
                    text: "Mock response".to_string(),
                }],
                role: Role::Assistant,
                usage: Usage {
                    input_tokens: 10,
                    output_tokens: 20,
                },
                stop_reason: Some("end_turn".to_string()),
                tool_calls: None,
            })
        }

        fn provider_name(&self) -> &str {
            "mock"
        }

        fn model_name(&self) -> &str {
            "mock-model"
        }
    }

    #[test]
    fn test_create_codebase_analysis_agent() {
        let client: Arc<dyn LlmClient> = Arc::new(MockLlmClient);
        let agent = create_agent(AgentType::CodebaseAnalysis, client);

        assert_eq!(
            agent.objective(),
            "Analyze codebase structure and identify architectural patterns"
        );
    }

    #[test]
    fn test_agent_type_values() {
        let agent_types = vec![AgentType::CodebaseAnalysis];

        for agent_type in agent_types {
            let client: Arc<dyn LlmClient> = Arc::new(MockLlmClient);
            let _agent = create_agent(agent_type, client);
        }
    }
}
