use crate::codebase_analysis::CodebaseAnalysisAgent;
use crate::requirements_gathering::UserClarificationAgent;
use crate::settings_management::SettingsManagementAgent;
use crate::sqlite_reader::SqliteReaderAgent;
use crate::storage::{AgentStorage, InMemoryStorage};
use crate::structured_json::StructuredJsonAgent;
use crate::tesseract::TesseractAgent;
use crate::Agent;
use nocodo_llm_sdk::client::LlmClient;
use nocodo_tools::ToolExecutor;
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
    /// Agent for collecting and managing settings required for workflow automation
    SettingsManagement,
}

/// Factory for creating AI agents with shared dependencies
pub struct AgentFactory<S: AgentStorage> {
    llm_client: Arc<dyn LlmClient>,
    storage: Arc<S>,
    tool_executor: Arc<ToolExecutor>,
}

impl<S: AgentStorage + 'static> AgentFactory<S> {
    /// Create a new AgentFactory with the given dependencies
    pub fn new(
        llm_client: Arc<dyn LlmClient>,
        storage: Arc<S>,
        tool_executor: Arc<ToolExecutor>,
    ) -> Self {
        Self {
            llm_client,
            storage,
            tool_executor,
        }
    }

    /// Create a CodebaseAnalysisAgent
    pub fn create_codebase_analysis_agent(&self) -> CodebaseAnalysisAgent<S> {
        CodebaseAnalysisAgent::new(
            self.llm_client.clone(),
            self.storage.clone(),
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
    ) -> anyhow::Result<TesseractAgent<S>> {
        TesseractAgent::new(self.llm_client.clone(), self.storage.clone(), base_path)
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
    ) -> anyhow::Result<StructuredJsonAgent<S>> {
        StructuredJsonAgent::new(
            self.llm_client.clone(),
            self.storage.clone(),
            self.tool_executor.clone(),
            config,
        )
    }

    /// Create a UserClarificationAgent for analyzing user requests
    ///
    /// This agent determines if a user's request needs clarification
    /// before proceeding with task.
    pub fn create_user_clarification_agent(&self) -> UserClarificationAgent<S, S> {
        UserClarificationAgent::new(
            self.llm_client.clone(),
            self.storage.clone(),
            self.storage.clone(),
            self.tool_executor.clone(),
        )
    }

    /// Create a SettingsManagementAgent for collecting workflow settings
    ///
    /// This agent collects API keys, file paths, URLs, and other settings
    /// needed for workflow automation.
    ///
    /// # Arguments
    /// * `settings_file_path` - Path where collected settings will be saved
    /// * `agent_schemas` - List of agent schemas defining what settings are needed
    pub fn create_settings_management_agent(
        &self,
        settings_file_path: std::path::PathBuf,
        agent_schemas: Vec<crate::AgentSettingsSchema>,
    ) -> SettingsManagementAgent<S> {
        SettingsManagementAgent::new(
            self.llm_client.clone(),
            self.storage.clone(),
            self.tool_executor.clone(),
            settings_file_path,
            agent_schemas,
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
    let storage = Arc::new(InMemoryStorage::new());
    let tool_executor = Arc::new(
        ToolExecutor::new(std::env::current_dir().unwrap()).with_max_file_size(10 * 1024 * 1024), // 10MB
    );

    create_agent_with_tools(agent_type, client, storage, tool_executor)
}

/// Factory function to create an agent with storage and tool executor support
///
/// # Arguments
///
/// * `agent_type` - The type of agent to create
/// * `client` - The LLM client to use for agent
/// * `storage` - Storage for session persistence
/// * `tool_executor` - Tool executor for running tools
///
/// # Returns
///
/// A boxed trait object implementing the Agent trait
pub fn create_agent_with_tools(
    agent_type: AgentType,
    client: Arc<dyn LlmClient>,
    storage: Arc<InMemoryStorage>,
    tool_executor: Arc<ToolExecutor>,
) -> Box<dyn Agent> {
    match agent_type {
        AgentType::CodebaseAnalysis => {
            Box::new(CodebaseAnalysisAgent::new(client, storage, tool_executor))
        }
        AgentType::Tesseract => {
            // For Tesseract, we need a specific base path. Use current directory as default
            let base_path = std::env::current_dir().unwrap_or_default();
            Box::new(TesseractAgent::new(client, storage, base_path).unwrap())
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
            Box::new(StructuredJsonAgent::new(client, storage, tool_executor, config).unwrap())
        }
        AgentType::UserClarification => {
            Box::new(UserClarificationAgent::new(
                client,
                storage.clone(),
                storage,
                tool_executor,
            ))
        }
        AgentType::SettingsManagement => {
            panic!(
                "SettingsManagement agent cannot be created via create_by_type. \
                 Use AgentFactory::create_settings_management_agent() or \
                 create_settings_management_agent() function instead, which require \
                 settings_file_path and agent_schemas parameters."
            )
        }
    }
}

/// Create a CodebaseAnalysisAgent with tool executor support
///
/// Uses in-memory storage by default for session persistence
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
) -> CodebaseAnalysisAgent<InMemoryStorage> {
    let storage = Arc::new(InMemoryStorage::new());
    CodebaseAnalysisAgent::new(client, storage, tool_executor)
}

/// Create a SqliteReaderAgent with tool executor support
///
/// Uses in-memory storage by default for session persistence
///
/// # Arguments
///
/// * `client` - The LLM client to use for the agent
/// * `tool_executor` - Tool executor for running tools
/// * `db_path` - Path to the SQLite database to analyze
///
/// # Returns
///
/// A SqliteReaderAgent instance
pub async fn create_sqlite_reader_agent(
    client: Arc<dyn LlmClient>,
    tool_executor: Arc<ToolExecutor>,
    db_path: String,
) -> anyhow::Result<SqliteReaderAgent<InMemoryStorage>> {
    let storage = Arc::new(InMemoryStorage::new());
    let agent = SqliteReaderAgent::new(client, storage, tool_executor, db_path).await?;
    Ok(agent)
}

/// Create a TesseractAgent with tool executor support
///
/// Uses in-memory storage by default for session persistence
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
) -> anyhow::Result<TesseractAgent<InMemoryStorage>> {
    let storage = Arc::new(InMemoryStorage::new());
    let agent = TesseractAgent::new(client, storage, base_path)?;
    Ok(agent)
}

/// Create a UserClarificationAgent with tool executor support
///
/// Uses in-memory storage by default for session persistence
///
/// # Arguments
///
/// * `client` - The LLM client to use for the agent
///
/// # Returns
///
/// A UserClarificationAgent instance
pub fn create_user_clarification_agent(
    client: Arc<dyn LlmClient>,
) -> UserClarificationAgent<InMemoryStorage, InMemoryStorage> {
    let storage = Arc::new(InMemoryStorage::new());
    let tool_executor = Arc::new(ToolExecutor::new(std::path::PathBuf::from(".")));
    UserClarificationAgent::new(client, storage.clone(), storage, tool_executor)
}

/// Create a SettingsManagementAgent with tool executor support
///
/// Uses in-memory storage by default for session persistence
///
/// # Arguments
///
/// * `client` - The LLM client to use for the agent
/// * `settings_file_path` - Path where collected settings will be saved
/// * `agent_schemas` - List of agent schemas defining what settings are needed
///
/// # Returns
///
/// A SettingsManagementAgent instance
pub fn create_settings_management_agent(
    client: Arc<dyn LlmClient>,
    settings_file_path: std::path::PathBuf,
    agent_schemas: Vec<crate::AgentSettingsSchema>,
) -> SettingsManagementAgent<InMemoryStorage> {
    let storage = Arc::new(InMemoryStorage::new());
    let tool_executor = Arc::new(ToolExecutor::new(std::path::PathBuf::from(".")));
    SettingsManagementAgent::new(
        client,
        storage,
        tool_executor,
        settings_file_path,
        agent_schemas,
    )
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
