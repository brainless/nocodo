use crate::{
    storage::AgentStorage,
    types::{
        Message, MessageRole, Session, SessionStatus, ToolCall as StorageToolCall, ToolCallStatus,
    },
    Agent, AgentTool,
};
use anyhow::{self};
use async_trait::async_trait;
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::tools::{ToolCall as LlmToolCall, ToolChoice};
use nocodo_llm_sdk::types::{CompletionRequest, ContentBlock, Message as LlmMessage, Role};
use nocodo_tools::ToolExecutor;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

#[cfg(test)]
mod tests;

pub struct SqliteReaderAgent<S: AgentStorage> {
    client: Arc<dyn LlmClient>,
    storage: Arc<S>,
    tool_executor: Arc<ToolExecutor>,
    db_path: String,
    system_prompt: String,
}

impl<S: AgentStorage> SqliteReaderAgent<S> {
    pub async fn new(
        client: Arc<dyn LlmClient>,
        storage: Arc<S>,
        tool_executor: Arc<ToolExecutor>,
        db_path: String,
    ) -> anyhow::Result<Self> {
        validate_db_path(&db_path)?;

        let table_names = nocodo_tools::sqlite_reader::get_table_names(&db_path).await?;

        let db_name = std::path::Path::new(&db_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("database");

        let system_prompt = generate_system_prompt(db_name, &table_names);

        Ok(Self {
            client,
            storage,
            tool_executor,
            db_path,
            system_prompt,
        })
    }

    /// Create a new SqliteAnalysisAgent for testing (skips path validation)
    #[cfg(test)]
    pub fn new_for_testing(
        client: Arc<dyn LlmClient>,
        storage: Arc<S>,
        tool_executor: Arc<ToolExecutor>,
        db_path: String,
        table_names: Vec<String>,
    ) -> Self {
        let db_name = std::path::Path::new(&db_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("database");

        let system_prompt = generate_system_prompt(db_name, &table_names);

        Self {
            client,
            storage,
            tool_executor,
            db_path,
            system_prompt,
        }
    }

    fn get_tool_definitions(&self) -> Vec<nocodo_llm_sdk::tools::Tool> {
        self.tools()
            .into_iter()
            .map(|tool| tool.to_tool_definition())
            .collect()
    }

    async fn build_messages(&self, session_id: i64) -> anyhow::Result<Vec<LlmMessage>> {
        let db_messages = self.storage.get_messages(session_id).await?;

        db_messages
            .into_iter()
            .map(|msg| {
                let role = match msg.role {
                    MessageRole::User => Role::User,
                    MessageRole::Assistant => Role::Assistant,
                    MessageRole::System => Role::System,
                    MessageRole::Tool => Role::User,
                };

                Ok(LlmMessage {
                    role,
                    content: vec![ContentBlock::Text { text: msg.content }],
                })
            })
            .collect()
    }

    async fn get_session(&self, session_id: i64) -> anyhow::Result<Session> {
        self.storage
            .get_session(session_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))
    }

    async fn execute_tool_call(
        &self,
        session_id: i64,
        message_id: Option<i64>,
        tool_call: &LlmToolCall,
    ) -> anyhow::Result<()> {
        let mut tool_request =
            AgentTool::parse_tool_call(tool_call.name(), tool_call.arguments().clone())?;

        if let nocodo_tools::types::ToolRequest::Sqlite3Reader(ref mut req) = tool_request {
            req.db_path = self.db_path.clone();
            tracing::debug!(
                db_path = %self.db_path,
                "Injected database path into sqlite3_reader tool call"
            );
        }

        let mut tool_call_record = StorageToolCall {
            id: None,
            session_id,
            message_id,
            tool_call_id: tool_call.id().to_string(),
            tool_name: tool_call.name().to_string(),
            request: tool_call.arguments().clone(),
            response: None,
            status: ToolCallStatus::Pending,
            execution_time_ms: None,
            created_at: chrono::Utc::now().timestamp(),
            completed_at: None,
            error_details: None,
        };
        let call_id = self
            .storage
            .create_tool_call(tool_call_record.clone())
            .await?;

        let start = Instant::now();
        let result: anyhow::Result<nocodo_tools::types::ToolResponse> =
            self.tool_executor.execute(tool_request).await;
        let execution_time = start.elapsed().as_millis() as i64;

        match result {
            Ok(response) => {
                let response_json = serde_json::to_value(&response)?;
                tool_call_record.complete(response_json, execution_time);
                tool_call_record.id = Some(call_id);
                self.storage.update_tool_call(tool_call_record).await?;

                let result_text = crate::format_tool_response(&response);
                let message_to_llm = format!("Tool {} result:\n{}", tool_call.name(), result_text);

                tracing::debug!(
                    tool_name = tool_call.name(),
                    tool_id = tool_call.id(),
                    execution_time_ms = execution_time,
                    "Tool execution completed successfully"
                );

                let tool_message = Message {
                    id: None,
                    session_id,
                    role: MessageRole::Tool,
                    content: message_to_llm,
                    created_at: chrono::Utc::now().timestamp(),
                };
                self.storage.create_message(tool_message).await?;
            }
            Err(e) => {
                let error_msg = format!("{:?}", e);
                tool_call_record.fail(error_msg.clone());
                tool_call_record.id = Some(call_id);
                self.storage.update_tool_call(tool_call_record).await?;

                let error_message_to_llm =
                    format!("Tool {} failed: {}", tool_call.name(), error_msg);

                tracing::debug!(
                    tool_name = tool_call.name(),
                    tool_id = tool_call.id(),
                    error = %error_msg,
                    "Tool execution failed"
                );

                let tool_error_message = Message {
                    id: None,
                    session_id,
                    role: MessageRole::Tool,
                    content: error_message_to_llm,
                    created_at: chrono::Utc::now().timestamp(),
                };
                self.storage.create_message(tool_error_message).await?;
            }
        }

        Ok(())
    }
}

#[async_trait]
impl<S: AgentStorage> Agent for SqliteReaderAgent<S> {
    fn objective(&self) -> &str {
        "Analyze SQLite database structure and contents"
    }

    fn system_prompt(&self) -> String {
        self.system_prompt.clone()
    }

    fn tools(&self) -> Vec<AgentTool> {
        vec![AgentTool::Sqlite3Reader]
    }

    fn settings_schema(&self) -> crate::AgentSettingsSchema {
        Self::static_settings_schema().unwrap_or_else(|| crate::AgentSettingsSchema {
            agent_name: "SQLite Analysis Agent".to_string(),
            section_name: "sqlite_reader".to_string(),
            settings: vec![],
        })
    }

    fn static_settings_schema() -> Option<crate::AgentSettingsSchema> {
        Some(crate::AgentSettingsSchema {
            agent_name: "SQLite Analysis Agent".to_string(),
            section_name: "sqlite_reader".to_string(),
            settings: vec![crate::SettingDefinition {
                name: "db_path".to_string(),
                label: "Database Path".to_string(),
                description: "Path to the SQLite database file to analyze".to_string(),
                setting_type: crate::SettingType::FilePath,
                required: true,
                default_value: None,
            }],
        })
    }

    async fn execute(&self, user_prompt: &str, session_id: i64) -> anyhow::Result<String> {
        let user_message = Message {
            id: None,
            session_id,
            role: MessageRole::User,
            content: user_prompt.to_string(),
            created_at: chrono::Utc::now().timestamp(),
        };
        self.storage.create_message(user_message).await?;

        let tools = self.get_tool_definitions();

        let mut iteration = 0;
        let max_iterations = 30;

        loop {
            iteration += 1;
            if iteration > max_iterations {
                let error = "Maximum iteration limit reached";
                let mut session = self.get_session(session_id).await?;
                session.status = SessionStatus::Failed;
                session.error = Some(error.to_string());
                session.ended_at = Some(chrono::Utc::now().timestamp());
                self.storage.update_session(session).await?;
                return Err(anyhow::anyhow!(error));
            }

            let messages = self.build_messages(session_id).await?;

            let request = CompletionRequest {
                messages,
                max_tokens: 4000,
                model: self.client.model_name().to_string(),
                system: Some(self.system_prompt()),
                temperature: Some(0.7),
                top_p: None,
                stop_sequences: None,
                tools: Some(tools.clone()),
                tool_choice: Some(ToolChoice::Auto),
                response_format: None,
            };

            let response = self.client.complete(request).await?;

            let text = extract_text_from_content(&response.content);

            let text_to_save = if text.is_empty() && response.tool_calls.is_some() {
                "[Using tools]".to_string()
            } else {
                text.clone()
            };

            let assistant_message = Message {
                id: None,
                session_id,
                role: MessageRole::Assistant,
                content: text_to_save,
                created_at: chrono::Utc::now().timestamp(),
            };
            let message_id = self.storage.create_message(assistant_message).await?;

            if let Some(tool_calls) = response.tool_calls {
                if tool_calls.is_empty() {
                    let mut session = self.get_session(session_id).await?;
                    session.status = SessionStatus::Completed;
                    session.result = Some(text.clone());
                    session.ended_at = Some(chrono::Utc::now().timestamp());
                    self.storage.update_session(session).await?;
                    return Ok(text);
                }

                for tool_call in tool_calls {
                    self.execute_tool_call(session_id, Some(message_id), &tool_call)
                        .await?;
                }
            } else {
                let mut session = self.get_session(session_id).await?;
                session.status = SessionStatus::Completed;
                session.result = Some(text.clone());
                session.ended_at = Some(chrono::Utc::now().timestamp());
                self.storage.update_session(session).await?;
                return Ok(text);
            }
        }
    }
}

fn validate_db_path(db_path: &str) -> anyhow::Result<()> {
    if db_path.is_empty() {
        anyhow::bail!("Database path cannot be empty");
    }

    let path = Path::new(db_path);

    if !path.is_absolute() {
        anyhow::bail!(
            "Database path must be absolute: {}. Use std::fs::canonicalize() if needed.",
            db_path
        );
    }

    if !path.exists() {
        anyhow::bail!("Database file not found: {}", db_path);
    }

    if !path.is_file() {
        anyhow::bail!("Path is not a file: {}", db_path);
    }

    Ok(())
}

fn generate_system_prompt(db_name: &str, table_names: &[String]) -> String {
    let tables_list = if table_names.is_empty() {
        "No tables found".to_string()
    } else {
        table_names.join(", ")
    };

    format!(
        "You are a database analysis expert specialized in SQLite databases.
Your role is to query data and provide insights about database contents.
You have access to the sqlite3_reader tool to execute SQL queries.

Use SELECT queries to retrieve data and PRAGMA statements to inspect schema and database structure.

ALLOWED QUERIES:
- SELECT queries to retrieve and analyze data
- PRAGMA queries to inspect database schema and structure

Useful PRAGMA commands for schema discovery:
- PRAGMA table_info(table_name) - Get column information for a specific table
- PRAGMA index_list(table_name) - Get indexes for a table
- PRAGMA foreign_key_list(table_name) - Get foreign keys for a table

You can ONLY use SELECT and PRAGMA statements. Do NOT use CREATE, \
INSERT, UPDATE, DELETE, ALTER, DROP, or any other data modification statements.

You should not summarize the data unless explicitly asked. Just list the results.

IMPORTANT: The database path is already configured.
You are analyzing the database named: {}
Tables in the database: {}
",
        db_name, tables_list
    )
}

fn extract_text_from_content(content: &[ContentBlock]) -> String {
    content
        .iter()
        .filter_map(|block| match block {
            ContentBlock::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}
