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
use std::sync::Arc;
use std::time::Instant;

pub struct PostgresReaderAgent<S: AgentStorage> {
    client: Arc<dyn LlmClient>,
    storage: Arc<S>,
    tool_executor: Arc<ToolExecutor>,
    connection_string: String,
    system_prompt: String,
}

impl<S: AgentStorage> PostgresReaderAgent<S> {
    pub async fn new(
        client: Arc<dyn LlmClient>,
        storage: Arc<S>,
        tool_executor: Arc<ToolExecutor>,
        connection_string: String,
    ) -> anyhow::Result<Self> {
        validate_connection_string(&connection_string)?;

        let table_names =
            nocodo_tools::postgres_reader::get_table_names(&connection_string, Some("public"))
                .await?;

        let db_name = extract_db_name(&connection_string)?;

        let system_prompt = generate_system_prompt(&db_name, &table_names);

        Ok(Self {
            client,
            storage,
            tool_executor,
            connection_string,
            system_prompt,
        })
    }

    /// Create a new PostgresReaderAgent for testing (skips connection validation)
    #[cfg(test)]
    pub fn new_for_testing(
        client: Arc<dyn LlmClient>,
        storage: Arc<S>,
        tool_executor: Arc<ToolExecutor>,
        connection_string: String,
        table_names: Vec<String>,
    ) -> Self {
        let db_name = extract_db_name(&connection_string).unwrap_or_else(|_| "database".to_string());

        let system_prompt = generate_system_prompt(&db_name, &table_names);

        Self {
            client,
            storage,
            tool_executor,
            connection_string,
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

        // Inject connection string into postgres_reader tool calls
        if let nocodo_tools::types::ToolRequest::PostgresReader(ref mut req) = tool_request {
            req.connection_string = self.connection_string.clone();
            tracing::debug!(
                connection_string_len = self.connection_string.len(),
                "Injected connection string into postgres_reader tool call"
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
impl<S: AgentStorage> Agent for PostgresReaderAgent<S> {
    fn objective(&self) -> &str {
        "Analyze PostgreSQL database structure and contents"
    }

    fn system_prompt(&self) -> String {
        self.system_prompt.clone()
    }

    fn tools(&self) -> Vec<AgentTool> {
        vec![AgentTool::PostgresReader]
    }

    fn settings_schema(&self) -> crate::AgentSettingsSchema {
        Self::static_settings_schema().unwrap_or_else(|| crate::AgentSettingsSchema {
            agent_name: "PostgreSQL Analysis Agent".to_string(),
            section_name: "postgres_reader".to_string(),
            settings: vec![],
        })
    }

    fn static_settings_schema() -> Option<crate::AgentSettingsSchema> {
        Some(crate::AgentSettingsSchema {
            agent_name: "PostgreSQL Analysis Agent".to_string(),
            section_name: "postgres_reader".to_string(),
            settings: vec![crate::SettingDefinition {
                name: "connection_string".to_string(),
                label: "Connection String".to_string(),
                description: "PostgreSQL connection string (postgresql://user:password@host:port/database)".to_string(),
                setting_type: crate::SettingType::Text,
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

fn validate_connection_string(connection_string: &str) -> anyhow::Result<()> {
    nocodo_tools::postgres_reader::validate_connection_string(connection_string)
        .map_err(|e| anyhow::anyhow!("Invalid connection string: {}", e))
}

fn extract_db_name(connection_string: &str) -> anyhow::Result<String> {
    let url = url::Url::parse(connection_string)
        .map_err(|e| anyhow::anyhow!("Failed to parse connection string: {}", e))?;

    let path = url.path().trim_start_matches('/');
    if path.is_empty() {
        Ok("postgres".to_string())
    } else {
        Ok(path.to_string())
    }
}

fn generate_system_prompt(db_name: &str, table_names: &[String]) -> String {
    let tables_list = if table_names.is_empty() {
        "No tables found in 'public' schema".to_string()
    } else {
        table_names.join(", ")
    };

    format!(
        "You are a database analysis expert specialized in PostgreSQL databases.
Your role is to query data and provide insights about database contents.
You have access to the postgres_reader tool to execute SQL queries.

Use SELECT queries to retrieve data and reflection mode to inspect schema and database structure.

ALLOWED QUERIES:
- SELECT queries to retrieve and analyze data
- Reflection queries to inspect database schema and structure

Reflection targets (use mode: 'reflect'):
- schema: List all schemas in the database
- tables: List all tables in a schema (default: 'public')
- table_info: Get column information for a specific table (requires table_name)
- indexes: List indexes for a table or schema
- views: List views in a schema
- foreign_keys: Get foreign key relationships for a table (requires table_name)
- constraints: Get all constraints for a table (requires table_name)
- stats: Get table statistics (row counts, dead tuples, etc.)

You can ONLY use SELECT statements and reflection mode. Do NOT use CREATE, \
INSERT, UPDATE, DELETE, ALTER, DROP, or any other data modification statements.

You should not summarize the data unless explicitly asked. Just list the results.

IMPORTANT: The connection string is already configured.
You are analyzing the database named: {}
Tables in the 'public' schema: {}
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
