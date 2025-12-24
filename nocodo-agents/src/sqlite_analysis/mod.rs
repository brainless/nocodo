use crate::{database::Database, Agent, AgentTool};
use anyhow::{self};
use async_trait::async_trait;
use manager_tools::ToolExecutor;
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::tools::{ToolCall, ToolChoice};
use nocodo_llm_sdk::types::{CompletionRequest, ContentBlock, Message, Role};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

#[cfg(test)]
mod tests;

pub struct SqliteAnalysisAgent {
    client: Arc<dyn LlmClient>,
    database: Arc<Database>,
    tool_executor: Arc<ToolExecutor>,
    db_path: String,
    system_prompt: String,
}

impl SqliteAnalysisAgent {
    pub fn new(
        client: Arc<dyn LlmClient>,
        database: Arc<Database>,
        tool_executor: Arc<ToolExecutor>,
        db_path: String,
    ) -> anyhow::Result<Self> {
        validate_db_path(&db_path)?;
        let system_prompt = generate_system_prompt(&db_path);

        Ok(Self {
            client,
            database,
            tool_executor,
            db_path,
            system_prompt,
        })
    }

    /// Create a new SqliteAnalysisAgent for testing (skips path validation)
    #[cfg(test)]
    pub fn new_for_testing(
        client: Arc<dyn LlmClient>,
        database: Arc<Database>,
        tool_executor: Arc<ToolExecutor>,
        db_path: String,
    ) -> Self {
        let system_prompt = generate_system_prompt(&db_path);

        Self {
            client,
            database,
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

    fn build_messages(&self, session_id: i64) -> anyhow::Result<Vec<Message>> {
        let db_messages = self.database.get_messages(session_id)?;

        db_messages
            .into_iter()
            .map(|msg| {
                let role = match msg.role.as_str() {
                    "user" => Role::User,
                    "assistant" => Role::Assistant,
                    "system" => Role::System,
                    "tool" => Role::User,
                    _ => Role::User,
                };

                Ok(Message {
                    role,
                    content: vec![ContentBlock::Text { text: msg.content }],
                })
            })
            .collect()
    }

    async fn execute_tool_call(
        &self,
        session_id: i64,
        message_id: Option<i64>,
        tool_call: &ToolCall,
    ) -> anyhow::Result<()> {
        let mut tool_request =
            AgentTool::parse_tool_call(tool_call.name(), tool_call.arguments().clone())?;

        if let manager_tools::types::ToolRequest::Sqlite3Reader(ref mut req) = tool_request {
            req.db_path = self.db_path.clone();
            tracing::debug!(
                db_path = %self.db_path,
                "Injected database path into sqlite3_reader tool call"
            );
        }

        let call_id = self.database.create_tool_call(
            session_id,
            message_id,
            tool_call.id(),
            tool_call.name(),
            tool_call.arguments().clone(),
        )?;

        let start = Instant::now();
        let result: anyhow::Result<manager_tools::types::ToolResponse> =
            self.tool_executor.execute(tool_request).await;
        let execution_time = start.elapsed().as_millis() as i64;

        match result {
            Ok(response) => {
                let response_json = serde_json::to_value(&response)?;
                self.database
                    .complete_tool_call(call_id, response_json.clone(), execution_time)?;

                let result_text = crate::format_tool_response(&response);
                let message_to_llm = format!("Tool {} result:\n{}", tool_call.name(), result_text);

                tracing::debug!(
                    tool_name = tool_call.name(),
                    tool_id = tool_call.id(),
                    execution_time_ms = execution_time,
                    "Tool execution completed successfully"
                );

                self.database
                    .create_message(session_id, "tool", &message_to_llm)?;
            }
            Err(e) => {
                let error_msg = format!("{:?}", e);
                self.database.fail_tool_call(call_id, &error_msg)?;

                let error_message_to_llm =
                    format!("Tool {} failed: {}", tool_call.name(), error_msg);

                tracing::debug!(
                    tool_name = tool_call.name(),
                    tool_id = tool_call.id(),
                    error = %error_msg,
                    "Tool execution failed"
                );

                self.database
                    .create_message(session_id, "tool", &error_message_to_llm)?;
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Agent for SqliteAnalysisAgent {
    fn objective(&self) -> &str {
        "Analyze SQLite database structure and contents"
    }

    fn system_prompt(&self) -> String {
        self.system_prompt.clone()
    }

    fn tools(&self) -> Vec<AgentTool> {
        vec![AgentTool::Sqlite3Reader]
    }

    async fn execute(&self, user_prompt: &str) -> anyhow::Result<String> {
        let session_id = self.database.create_session(
            "sqlite-analysis",
            self.client.provider_name(),
            self.client.model_name(),
            Some(&self.system_prompt),
            user_prompt,
        )?;

        self.database
            .create_message(session_id, "user", user_prompt)?;

        let tools = self.get_tool_definitions();

        let mut iteration = 0;
        let max_iterations = 30;

        loop {
            iteration += 1;
            if iteration > max_iterations {
                let error = "Maximum iteration limit reached";
                self.database.fail_session(session_id, error)?;
                return Err(anyhow::anyhow!(error));
            }

            let messages = self.build_messages(session_id)?;

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
            };

            let response = self.client.complete(request).await?;

            let text = extract_text_from_content(&response.content);

            // If there's no text but there are tool calls, use a placeholder for storage
            let text_to_save = if text.is_empty() && response.tool_calls.is_some() {
                "[Using tools]"
            } else {
                &text
            };

            let message_id = self
                .database
                .create_message(session_id, "assistant", text_to_save)?;

            if let Some(tool_calls) = response.tool_calls {
                if tool_calls.is_empty() {
                    self.database.complete_session(session_id, &text)?;
                    return Ok(text);
                }

                for tool_call in tool_calls {
                    self.execute_tool_call(session_id, Some(message_id), &tool_call)
                        .await?;
                }
            } else {
                self.database.complete_session(session_id, &text)?;
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

fn generate_system_prompt(db_path: &str) -> String {
    format!(
        "You are a database analysis expert specialized in SQLite databases. \
         You are analyzing the database at: {}

Your role is to query data and provide insights about database contents. \
You have access to the sqlite3_reader tool which executes read-only SQL queries.

IMPORTANT: The database path is already configured. You do NOT need to specify \
db_path in your tool calls - just provide the SQL query.

ALLOWED QUERIES:
- SELECT queries to retrieve data
- PRAGMA queries to inspect schema (PRAGMA table_list, PRAGMA table_info(name))

You can ONLY use SELECT and PRAGMA statements. Do NOT use CREATE, INSERT, UPDATE, \
DELETE, ALTER, DROP, or any other statements.

Best Practices:
1. Keep queries simple and direct
2. Use LIMIT clauses for large result sets
3. For latest/newest records: use ORDER BY column DESC LIMIT 1
4. For counting: use SELECT COUNT(*) FROM table
5. Answer user questions concisely based on query results

Focus on answering the user's question directly.",
        db_path
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
