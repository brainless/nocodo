use nocodo_agents::AgentStorage;
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
        AgentInfo {
            id: "imap".to_string(),
            name: "IMAP Email Agent".to_string(),
            description:
                "Agent for reading and analyzing emails from IMAP mailboxes with intelligent triage and information extraction"
                    .to_string(),
            enabled: true,
        },
        AgentInfo {
            id: "pdftotext".to_string(),
            name: "PDF to Text Agent".to_string(),
            description:
                "Agent for extracting text from PDF files using pdftotext with layout preservation and page selection capabilities"
                    .to_string(),
            enabled: true,
        },
    ]
}

/// Creates a SQLite analysis agent using the shared storage
///
/// # Arguments
///
/// * `llm_client` - The LLM client to use for the agent
/// * `storage` - Shared storage for session persistence
/// * `db_path` - Path to the SQLite database to analyze
///
/// # Returns
///
/// A SQLite analysis agent instance
pub async fn create_sqlite_agent<S: AgentStorage + 'static>(
    llm_client: &Arc<dyn LlmClient>,
    storage: &Arc<S>,
    db_path: &str,
) -> anyhow::Result<nocodo_agents::sqlite_reader::SqliteReaderAgent<S>> {
    let tool_executor = Arc::new(
        nocodo_tools::ToolExecutor::new(std::env::current_dir()?)
            .with_max_file_size(10 * 1024 * 1024),
    );

    let agent = nocodo_agents::sqlite_reader::SqliteReaderAgent::new(
        llm_client.clone(),
        storage.clone(),
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
/// * `llm_client` - The LLM client to use for agent
/// * `storage` - Shared storage for session persistence
/// * `image_path` - Path to the image file to process
///
/// # Returns
///
/// A Tesseract OCR agent instance
pub async fn create_tesseract_agent<S: AgentStorage + 'static>(
    llm_client: &Arc<dyn LlmClient>,
    storage: &Arc<S>,
    image_path: &str,
) -> anyhow::Result<nocodo_agents::tesseract::TesseractAgent<S>> {
    let agent = nocodo_agents::tesseract::TesseractAgent::new(
        llm_client.clone(),
        storage.clone(),
        std::path::PathBuf::from(image_path),
    )?;

    Ok(agent)
}

/// Creates a Structured JSON agent
///
/// # Arguments
///
/// * `llm_client` - The LLM client to use for agent
/// * `storage` - Shared storage for session persistence
/// * `type_names` - List of TypeScript type names to use for validation
/// * `domain_description` - Description of domain context
///
/// # Returns
///
/// A Structured JSON agent instance
pub fn create_structured_json_agent<S: AgentStorage + 'static>(
    llm_client: &Arc<dyn LlmClient>,
    storage: &Arc<S>,
    type_names: Vec<String>,
    domain_description: String,
) -> anyhow::Result<nocodo_agents::structured_json::StructuredJsonAgent<S>> {
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
        storage.clone(),
        tool_executor,
        config,
    )?;

    Ok(agent)
}

/// Creates a User Clarification agent
///
/// # Arguments
///
/// * `llm_client` - The LLM client to use for agent
/// * `storage` - Shared storage for session persistence
/// * `db_connection` - Shared database connection for requirements Q&A storage
///
/// # Returns
///
/// A User Clarification agent instance
pub fn create_user_clarification_agent(
    llm_client: &Arc<dyn LlmClient>,
    storage: &Arc<crate::storage::SqliteAgentStorage>,
    db_connection: &crate::DbConnection,
) -> anyhow::Result<
    nocodo_agents::requirements_gathering::UserClarificationAgent<
        crate::storage::SqliteAgentStorage,
        DirectRequirementsStorage,
    >,
> {
    use crate::storage::SqliteAgentStorage;

    let tool_executor = Arc::new(
        nocodo_tools::ToolExecutor::new(std::env::current_dir()?)
            .with_max_file_size(10 * 1024 * 1024),
    );

    // Create a requirements storage wrapper for direct database access
    let requirements_storage = Arc::new(DirectRequirementsStorage::new(db_connection.to_owned()));

    let agent = nocodo_agents::requirements_gathering::UserClarificationAgent::new(
        llm_client.clone(),
        storage.clone(),
        requirements_storage,
        tool_executor,
    );

    Ok(agent)
}

// Direct requirements storage that accesses database directly
pub struct DirectRequirementsStorage {
    db_connection: crate::DbConnection,
}

impl DirectRequirementsStorage {
    fn new(db_connection: crate::DbConnection) -> Self {
        Self { db_connection }
    }
}

#[async_trait::async_trait]
impl nocodo_agents::requirements_gathering::storage::RequirementsStorage
    for DirectRequirementsStorage
{
    async fn store_questions(
        &self,
        _session_id: i64,
        _tool_call_id: Option<i64>,
        _questions: &[shared_types::user_interaction::UserQuestion],
    ) -> Result<(), nocodo_agents::StorageError> {
        Ok(())
    }

    async fn get_pending_questions(
        &self,
        session_id: i64,
    ) -> Result<Vec<shared_types::user_interaction::UserQuestion>, nocodo_agents::StorageError>
    {
        let conn = self.db_connection.clone();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| {
                nocodo_agents::StorageError::OperationFailed(format!("Lock error: {}", e))
            })?;

            let mut stmt = conn
                .prepare(
                    "SELECT question_id, question, description, response_type
                     FROM project_requirements_qna
                     WHERE session_id = ?1 AND answer IS NULL
                     ORDER BY created_at ASC",
                )
                .map_err(|e| nocodo_agents::StorageError::OperationFailed(e.to_string()))?;

            let questions = stmt
                .query_map([session_id], |row| {
                    let response_type_str: String = row.get(3)?;
                    let response_type = match response_type_str.as_str() {
                        "text" => shared_types::user_interaction::QuestionType::Text,
                        "password" => shared_types::user_interaction::QuestionType::Password,
                        "file_path" => shared_types::user_interaction::QuestionType::FilePath,
                        "email" => shared_types::user_interaction::QuestionType::Email,
                        "url" => shared_types::user_interaction::QuestionType::Url,
                        _ => shared_types::user_interaction::QuestionType::Text,
                    };

                    Ok(shared_types::user_interaction::UserQuestion {
                        id: row.get(0)?,
                        question: row.get(1)?,
                        description: row.get(2)?,
                        response_type,
                        default: None,
                        options: None,
                    })
                })
                .map_err(|e| nocodo_agents::StorageError::OperationFailed(e.to_string()))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| nocodo_agents::StorageError::OperationFailed(e.to_string()))?;

            Ok(questions)
        })
        .await
        .map_err(|e| {
            nocodo_agents::StorageError::OperationFailed(format!("Task join error: {}", e))
        })?
    }

    async fn store_answers(
        &self,
        session_id: i64,
        answers: &std::collections::HashMap<String, String>,
    ) -> Result<(), nocodo_agents::StorageError> {
        let conn = self.db_connection.clone();
        let answers = answers.clone();

        tokio::task::spawn_blocking(move || {
            let conn = conn.lock().map_err(|e| {
                nocodo_agents::StorageError::OperationFailed(format!("Lock error: {}", e))
            })?;

            let now = chrono::Utc::now().timestamp();
            for (question_id, answer) in answers {
                conn.execute(
                    "UPDATE project_requirements_qna SET answer = ?1, answered_at = ?2 WHERE session_id = ?3 AND question_id = ?4",
                    rusqlite::params![answer, now, session_id, question_id],
                )
                .map_err(|e| nocodo_agents::StorageError::OperationFailed(e.to_string()))?;
            }

            Ok(())
        })
        .await
        .map_err(|e| nocodo_agents::StorageError::OperationFailed(format!("Task join error: {}", e)))?
    }
}

/// Creates a Settings Management agent
///
/// # Arguments
///
/// * `llm_client` - The LLM client to use for agent
/// * `storage` - Shared storage for session persistence
/// * `settings_file_path` - Path to the TOML settings file
/// * `agent_schemas` - List of agent settings schemas
///
/// # Returns
///
/// A Settings Management agent instance
pub fn create_settings_management_agent<S: AgentStorage + 'static>(
    llm_client: &Arc<dyn nocodo_llm_sdk::client::LlmClient>,
    storage: &Arc<S>,
    settings_file_path: &str,
    agent_schemas: Vec<nocodo_agents::AgentSettingsSchema>,
) -> anyhow::Result<nocodo_agents::settings_management::SettingsManagementAgent<S>> {
    let tool_executor = Arc::new(
        nocodo_tools::ToolExecutor::new(std::env::current_dir()?)
            .with_max_file_size(10 * 1024 * 1024),
    );

    let agent = nocodo_agents::settings_management::SettingsManagementAgent::new(
        llm_client.clone(),
        storage.clone(),
        tool_executor,
        std::path::PathBuf::from(settings_file_path),
        agent_schemas,
    );

    Ok(agent)
}

/// Creates an IMAP Email agent
///
/// # Arguments
///
/// * `llm_client` - The LLM client to use for agent
/// * `storage` - Shared storage for session persistence
/// * `host` - IMAP server hostname
/// * `port` - IMAP server port
/// * `username` - IMAP username
/// * `password` - IMAP password
///
/// # Returns
///
/// An IMAP Email agent instance
pub fn create_imap_agent<S: AgentStorage + 'static>(
    llm_client: &Arc<dyn nocodo_llm_sdk::client::LlmClient>,
    storage: &Arc<S>,
    host: &str,
    port: u16,
    username: &str,
    password: &str,
) -> anyhow::Result<nocodo_agents::imap_email::ImapEmailAgent<S>> {
    let tool_executor = Arc::new(
        nocodo_tools::ToolExecutor::new(std::env::current_dir()?)
            .with_max_file_size(10 * 1024 * 1024),
    );

    let agent = nocodo_agents::imap_email::ImapEmailAgent::new(
        llm_client.clone(),
        storage.clone(),
        tool_executor,
        host.to_string(),
        port,
        username.to_string(),
        password.to_string(),
    );

    Ok(agent)
}

/// Creates a PDF to Text agent
///
/// # Arguments
///
/// * `llm_client` - The LLM client to use for agent
/// * `storage` - Shared storage for session persistence
/// * `pdf_path` - Path to the PDF file to process
///
/// # Returns
///
/// A PDF to Text agent instance
pub async fn create_pdftotext_agent<S: AgentStorage + 'static>(
    llm_client: &Arc<dyn LlmClient>,
    storage: &Arc<S>,
    pdf_path: &str,
) -> anyhow::Result<nocodo_agents::pdftotext::PdfToTextAgent<S>> {
    let agent = nocodo_agents::pdftotext::PdfToTextAgent::new(
        llm_client.clone(),
        storage.clone(),
        std::path::PathBuf::from(pdf_path),
    )?;

    Ok(agent)
}
