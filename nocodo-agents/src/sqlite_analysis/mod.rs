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
    pub async fn new(
        client: Arc<dyn LlmClient>,
        database: Arc<Database>,
        tool_executor: Arc<ToolExecutor>,
        db_path: String,
    ) -> anyhow::Result<Self> {
        validate_db_path(&db_path)?;

        let schema_info = discover_schema(&tool_executor, &db_path).await.unwrap_or_else(|e| {
            tracing::warn!("Schema discovery failed: {e}. Using fallback prompt.");
            SchemaInfo { tables: vec![] }
        });

        let system_prompt = generate_system_prompt_with_schema(&db_path, &schema_info);

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
        let system_prompt = generate_system_prompt_with_schema(&db_path, &SchemaInfo { tables: vec![] });

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

#[derive(Debug, Clone)]
struct SchemaInfo {
    tables: Vec<TableInfo>,
}

#[derive(Debug, Clone)]
struct TableInfo {
    name: String,
    create_sql: Option<String>,
}

async fn discover_schema(
    executor: &Arc<ToolExecutor>,
    db_path: &str,
) -> anyhow::Result<SchemaInfo> {
    let request = manager_tools::types::ToolRequest::Sqlite3Reader(
        manager_tools::types::Sqlite3ReaderRequest {
            db_path: db_path.to_string(),
            mode: manager_tools::types::SqliteMode::Reflect {
                target: "tables".to_string(),
                table_name: None,
            },
            limit: Some(1000),
        },
    );

    let response = executor.execute(request).await?;
    let schema_info = parse_schema_response(&response)?;

    Ok(schema_info)
}

fn parse_schema_response(
    response: &manager_tools::types::ToolResponse,
) -> anyhow::Result<SchemaInfo> {
    match response {
        manager_tools::types::ToolResponse::Sqlite3Reader(sqlite_response) => {
            let output = &sqlite_response.formatted_output;
            let tables = parse_tables_from_reflection(output)?;
            Ok(SchemaInfo { tables })
        }
        _ => anyhow::bail!("Unexpected response type from reflect mode"),
    }
}

fn parse_tables_from_reflection(output: &str) -> anyhow::Result<Vec<TableInfo>> {
    let mut tables = vec![];

    let lines: Vec<&str> = output.lines().collect();
    let mut in_table_section = false;
    let mut header_line_idx = None;

    for (idx, line) in lines.iter().enumerate() {
        if line.contains("Schema Reflection (tables):") {
            in_table_section = true;
            continue;
        }

        if in_table_section {
            if line.trim().is_empty() {
                continue;
            }

            if line.contains("name") && line.contains("sql") {
                header_line_idx = Some(idx);
                continue;
            }

            if header_line_idx.is_some() && (line.starts_with("─") || line.starts_with("-+-")) {
                continue;
            }

            if header_line_idx.is_some() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                let parts: Vec<&str> = line.split('|').map(|p| p.trim()).collect();
                if parts.len() >= 2 {
                    let table_name = parts[0].to_string();
                    let create_sql = if parts.len() >= 2 && !parts[1].is_empty() {
                        Some(parts[1].to_string())
                    } else {
                        None
                    };

                    if !table_name.is_empty() && !table_name.starts_with("name") {
                        tables.push(TableInfo {
                            name: table_name,
                            create_sql,
                        });
                    }
                }
            }
        }
    }

    Ok(tables)
}

fn extract_table_name_from_create_sql(sql: &str) -> Option<String> {
    let sql_upper = sql.to_uppercase();
    if let Some(start) = sql_upper.find("CREATE TABLE") {
        let start = start + "CREATE TABLE".len();
        let rest = &sql[start..];
        let rest = rest.trim_start();
        if rest.to_uppercase().starts_with("IF NOT EXISTS") {
            let rest = &rest["IF NOT EXISTS".len()..];
            let rest = rest.trim_start();
            rest.split_whitespace()
                .next()
                .map(|s| s.trim_matches(|c| matches!(c, '(' | '"' | '[' | ']')).to_string())
        } else {
            rest.split_whitespace()
                .next()
                .map(|s| s.trim_matches(|c| matches!(c, '(' | '"' | '[' | ']')).to_string())
        }
    } else {
        None
    }
}

fn extract_columns_from_ddl(create_sql: &str) -> String {
    let sql_upper = create_sql.to_uppercase();
    if let Some(start) = sql_upper.find("CREATE TABLE") {
        let start = start + "CREATE TABLE".len();
        let rest = &create_sql[start..];
        if let Some(paren_start) = rest.find('(') {
            let columns_section = &rest[paren_start + 1..];
            if let Some(paren_end) = columns_section.rfind(')') {
                let columns = &columns_section[..paren_end];
                let column_names: Vec<String> = columns
                    .split(',')
                    .take(10)
                    .filter_map(|col| {
                        let col = col.trim();
                        col.split_whitespace()
                            .next()
                            .filter(|c| {
                                !c.to_uppercase().starts_with("CONSTRAINT")
                                    && !c.to_uppercase().starts_with("PRIMARY")
                            })
                            .map(|s| s.to_string())
                    })
                    .collect();

                if column_names.len() >= 10 {
                    format!(
                        "{}, ... and {} more",
                        column_names.join(", "),
                        columns.split(',').count() - 10
                    )
                } else {
                    column_names.join(", ")
                }
            } else {
                "columns available".to_string()
            }
        } else {
            "columns available".to_string()
        }
    } else {
        "columns available".to_string()
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
    generate_system_prompt_with_schema(db_path, &SchemaInfo { tables: vec![] })
}

fn generate_system_prompt_with_schema(db_path: &str, schema_info: &SchemaInfo) -> String {
    let tables_section = if schema_info.tables.is_empty() {
        "No tables found in the database.".to_string()
    } else {
        let table_list = schema_info
            .tables
            .iter()
            .map(|table| {
                if let Some(sql) = &table.create_sql {
                    format!("- {} ({})", table.name, extract_columns_from_ddl(sql))
                } else {
                    format!("- {}", table.name)
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        format!("Available Tables:\n{}", table_list)
    };

    format!(
        "You are a database analysis expert specialized in SQLite databases. \
         You are analyzing the database at: {}

DATABASE SCHEMA (discovered at initialization):
{}

Your role is to query data and provide insights about database contents. \
You have access to the sqlite3_reader tool with TWO modes:

1. QUERY MODE - Execute SQL queries
   - Use for: SELECT statements, PRAGMA queries
   - Example: {{\"mode\": \"query\", \"query\": \"SELECT * FROM users LIMIT 5\"}}

2. REFLECT MODE - Introspect database schema at runtime
   - Use for: Discovering tables, getting column info, viewing indexes
   - Targets: \"tables\", \"schema\", \"table_info\", \"indexes\", \"views\", \"foreign_keys\", \"stats\"
   - Example: {{\"mode\": \"reflect\", \"target\": \"tables\"}}
   - Example: {{\"mode\": \"reflect\", \"target\": \"table_info\", \"table_name\": \"users\"}}

IMPORTANT: The database path is already configured. You do NOT need to specify \
db_path in your tool calls.

ALLOWED QUERIES (query mode):
- SELECT queries to retrieve data
- PRAGMA queries to inspect schema

You can ONLY use SELECT and PRAGMA statements in query mode. Do NOT use CREATE, \
INSERT, UPDATE, DELETE, ALTER, DROP, or any other statements.

Best Practices:
1. Use the schema information above to construct accurate queries
2. If you need detailed column information, use reflect mode with \"table_info\"
3. Keep queries simple and direct
4. Use LIMIT clauses for large result sets
5. For latest/newest records: use ORDER BY column DESC LIMIT 1
6. For counting: use SELECT COUNT(*) FROM table
7. Answer user questions concisely based on query results

Focus on answering the user's question directly using the schema provided.",
        db_path,
        tables_section
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
