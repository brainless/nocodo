# Add SQLite Analysis Agent to nocodo-agents

**Status**: 📋 Not Started
**Priority**: Medium
**Created**: 2024-12-24

## Summary

Add a specialized AI agent (`SqliteReaderAgent`) for analyzing SQLite databases. This agent uses the sqlite3_reader tool to query databases, explore schemas, and answer user questions about database contents. The agent will be reusable across projects (manager, Indistocks, etc.).

## Problem Statement

Projects need a way to analyze SQLite databases using AI:
- Users want to ask natural language questions about their databases
- Schema exploration should be automatic (PRAGMA statements)
- Analysis should be safe (read-only via sqlite3_reader tool)
- Solution should be reusable across multiple projects

Currently, there's no agent specialized for database analysis in nocodo-agents.

## Goals

1. **Create SqliteReaderAgent**: Specialized agent for SQLite database analysis
2. **Database path configuration**: Agent configured with specific database at construction
3. **System prompt includes path**: Override Agent trait to return String containing db_path
4. **Path injection**: Automatically inject db_path into sqlite3_reader tool calls
5. **Security**: Use absolute paths and validate database paths
6. **Reusability**: Usable in manager project and external projects like Indistocks

## Architecture Overview

### Design Decisions

| Aspect | Decision | Rationale |
|--------|----------|-----------|
| **Database path** | Passed at agent construction | Agent analyzes specific database, not arbitrary paths |
| **Path injection** | Override tool execution to inject path | LLM doesn't manage paths, more secure |
| **System prompt** | Return String (not &str) to include path | LLM needs to know which database it's analyzing |
| **Path type** | Absolute paths only | Prevents ambiguity, easier validation |
| **Path validation** | Validate existence + allowlist support | Security layer for production use |
| **Tool access** | Only Sqlite3Reader tool | Agent is read-only database analyzer |

### Agent Structure

```rust
pub struct SqliteReaderAgent {
    client: Arc<dyn LlmClient>,
    database: Arc<Database>,           // Session tracking (separate from analyzed DB)
    tool_executor: Arc<ToolExecutor>,
    db_path: String,                   // Absolute path to database being analyzed
    system_prompt: String,             // Pre-computed prompt with db_path
}
```

### System Prompt Template

The system prompt will be generated at construction time to include the database path:

```rust
format!(
    "You are a database analysis expert specialized in SQLite databases. \
     You are analyzing the database at: {}

     Your role is to explore schemas, query data, and provide insights about \
     database structure and contents. You have access to the sqlite3_reader tool \
     which executes read-only SQL queries (SELECT and PRAGMA statements).

     IMPORTANT: Always use db_path='{}' in your sqlite3_reader tool calls.

     Schema Exploration Commands:
     - PRAGMA table_list - List all tables
     - PRAGMA table_info(table_name) - Get column details for a table
     - PRAGMA foreign_key_list(table_name) - Get foreign key relationships
     - SELECT sql FROM sqlite_master WHERE name='table_name' - Get CREATE statement

     Best Practices:
     1. Start by exploring the schema before querying data
     2. Use appropriate LIMIT clauses for large result sets
     3. Provide clear explanations of your findings
     4. When counting rows, use COUNT(*) queries
     5. Identify relationships between tables using foreign keys

     Always base your analysis on actual database contents, not assumptions.",
    db_path, db_path
)
```

## Implementation Plan

### Phase 1: Update Agent Infrastructure

#### 1.1 Modify Agent Trait to Support Dynamic System Prompts

**File**: `nocodo-agents/src/lib.rs`

Update the `Agent` trait to return `String` instead of `&str`:

```rust
#[async_trait]
pub trait Agent: Send + Sync {
    fn objective(&self) -> &str;

    fn system_prompt(&self) -> String;  // ← Changed from &str to String

    fn pre_conditions(&self) -> Option<Vec<String>> {
        None
    }

    fn tools(&self) -> Vec<AgentTool>;

    async fn execute(&self, _user_prompt: &str) -> anyhow::Result<String> {
        anyhow::bail!("Execute method not implemented for this agent")
    }
}
```

**Impact**: This is a breaking change for existing agents (CodebaseAnalysisAgent).

#### 1.2 Update CodebaseAnalysisAgent to Return String

**File**: `nocodo-agents/src/codebase_analysis/mod.rs`

Change the system_prompt method:

```rust
fn system_prompt(&self) -> String {
    "You are a codebase analysis expert...".to_string()
}
```

Update all usages of `self.system_prompt()` to handle String instead of &str:

```rust
// Line 68-69
let session_id = self.database.create_session(
    "codebase-analysis",
    self.client.provider_name(),
    self.client.model_name(),
    Some(&self.system_prompt()),  // ← Add & to borrow String
    user_prompt,
)?;

// Line 98
system: Some(self.system_prompt()),  // ← Already works (moved, not borrowed)
```

#### 1.3 Add Sqlite3Reader to AgentTool Enum

**File**: `nocodo-agents/src/lib.rs`

Add new variant to `AgentTool` enum:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum AgentTool {
    ListFiles,
    ReadFile,
    WriteFile,
    Grep,
    ApplyPatch,
    Bash,
    AskUser,
    Sqlite3Reader,  // ← ADD THIS
}
```

Update the `name()` method:

```rust
pub fn name(&self) -> &'static str {
    match self {
        AgentTool::ListFiles => "list_files",
        AgentTool::ReadFile => "read_file",
        AgentTool::WriteFile => "write_file",
        AgentTool::Grep => "grep",
        AgentTool::ApplyPatch => "apply_patch",
        AgentTool::Bash => "bash",
        AgentTool::AskUser => "ask_user",
        AgentTool::Sqlite3Reader => "sqlite3_reader",  // ← ADD THIS
    }
}
```

Update the `parse_tool_call()` method:

```rust
pub fn parse_tool_call(
    name: &str,
    arguments: serde_json::Value,
) -> anyhow::Result<ToolRequest> {
    let request = match name {
        "list_files" => {
            let req: ListFilesRequest = serde_json::from_value(arguments)?;
            ToolRequest::ListFiles(req)
        }
        // ... existing cases
        "sqlite3_reader" => {  // ← ADD THIS
            let req: manager_tools::types::Sqlite3ReaderRequest =
                serde_json::from_value(arguments)?;
            ToolRequest::Sqlite3Reader(req)
        }
        _ => anyhow::bail!("Unknown tool: {}", name),
    };

    Ok(request)
}
```

**Note**: `format_tool_response()` already handles `Sqlite3Reader` at line 99, no changes needed! ✅

#### 1.4 Register Sqlite3Reader Tool Schema

**File**: `nocodo-agents/src/tools/llm_schemas.rs`

Add import:

```rust
use manager_tools::types::Sqlite3ReaderRequest;
```

Add tool definition to `create_tool_definitions()`:

```rust
pub fn create_tool_definitions() -> Vec<Tool> {
    vec![
        Tool::from_type::<ListFilesRequest>()
            .name("list_files")
            .description("List files and directories in a given path")
            .build(),
        // ... existing tools
        Tool::from_type::<Sqlite3ReaderRequest>()
            .name("sqlite3_reader")
            .description("Execute read-only SQL queries (SELECT and PRAGMA) on SQLite databases")
            .build(),
    ]
}
```

### Phase 2: Create SqliteReaderAgent Module

#### 2.1 Create Module Structure

Create new directory and files:
```
nocodo-agents/src/
  sqlite_reader/
    mod.rs          # Main agent implementation
```

#### 2.2 Implement SqliteReaderAgent

**File**: `nocodo-agents/src/sqlite_reader/mod.rs`

```rust
use crate::{database::Database, Agent, AgentTool};
use anyhow::{self, Context};
use async_trait::async_trait;
use manager_tools::ToolExecutor;
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::tools::{ToolCall, ToolChoice};
use nocodo_llm_sdk::types::{CompletionRequest, ContentBlock, Message, Role};
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

/// Agent specialized in analyzing SQLite databases
pub struct SqliteReaderAgent {
    client: Arc<dyn LlmClient>,
    database: Arc<Database>,
    tool_executor: Arc<ToolExecutor>,
    db_path: String,
    system_prompt: String,
}

impl SqliteReaderAgent {
    /// Create a new SqliteReaderAgent
    ///
    /// # Arguments
    /// * `client` - LLM client for AI inference
    /// * `database` - Database for session/message tracking (not the analyzed database)
    /// * `tool_executor` - Tool executor for running tools
    /// * `db_path` - Absolute path to the SQLite database to analyze
    ///
    /// # Security
    /// The db_path is validated to ensure:
    /// - It is an absolute path
    /// - The file exists and is readable
    /// - The path is not a directory
    pub fn new(
        client: Arc<dyn LlmClient>,
        database: Arc<Database>,
        tool_executor: Arc<ToolExecutor>,
        db_path: String,
    ) -> anyhow::Result<Self> {
        // Validate database path
        validate_db_path(&db_path)?;

        // Generate system prompt with database path
        let system_prompt = generate_system_prompt(&db_path);

        Ok(Self {
            client,
            database,
            tool_executor,
            db_path,
            system_prompt,
        })
    }

    /// Get tool definitions for this agent
    fn get_tool_definitions(&self) -> Vec<nocodo_llm_sdk::tools::Tool> {
        self.tools()
            .into_iter()
            .map(|tool| tool.to_tool_definition())
            .collect()
    }

    /// Build messages from session history
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

    /// Execute a tool call and inject database path
    async fn execute_tool_call(
        &self,
        session_id: i64,
        message_id: Option<i64>,
        tool_call: &ToolCall,
    ) -> anyhow::Result<()> {
        // 1. Parse LLM tool call into typed ToolRequest
        let mut tool_request =
            AgentTool::parse_tool_call(tool_call.name(), tool_call.arguments().clone())?;

        // 2. Inject database path for Sqlite3Reader tool calls
        if let manager_tools::types::ToolRequest::Sqlite3Reader(ref mut req) = tool_request {
            req.db_path = self.db_path.clone();
            tracing::debug!(
                db_path = %self.db_path,
                "Injected database path into sqlite3_reader tool call"
            );
        }

        // 3. Record tool call in database
        let call_id = self.database.create_tool_call(
            session_id,
            message_id,
            tool_call.id(),
            tool_call.name(),
            tool_call.arguments().clone(),
        )?;

        // 4. Execute tool
        let start = Instant::now();
        let result: anyhow::Result<manager_tools::types::ToolResponse> =
            self.tool_executor.execute(tool_request).await;
        let execution_time = start.elapsed().as_millis() as i64;

        // 5. Update database with result
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
impl Agent for SqliteReaderAgent {
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
        // 1. Create session
        let session_id = self.database.create_session(
            "sqlite-analysis",
            self.client.provider_name(),
            self.client.model_name(),
            Some(&self.system_prompt),
            user_prompt,
        )?;

        // 2. Create initial user message
        self.database
            .create_message(session_id, "user", user_prompt)?;

        // 3. Get tool definitions
        let tools = self.get_tool_definitions();

        // 4. Execution loop (max 30 iterations)
        let mut iteration = 0;
        let max_iterations = 30;

        loop {
            iteration += 1;
            if iteration > max_iterations {
                let error = "Maximum iteration limit reached";
                self.database.fail_session(session_id, error)?;
                return Err(anyhow::anyhow!(error));
            }

            // 5. Build request with conversation history
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

            // 6. Call LLM
            let response = self.client.complete(request).await?;

            // 7. Extract text and save assistant message
            let text = extract_text_from_content(&response.content);
            let message_id = self
                .database
                .create_message(session_id, "assistant", &text)?;

            // 8. Check for tool calls
            if let Some(tool_calls) = response.tool_calls {
                if tool_calls.is_empty() {
                    self.database.complete_session(session_id, &text)?;
                    return Ok(text);
                }

                // 9. Execute tools
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

/// Validate database path for security
fn validate_db_path(db_path: &str) -> anyhow::Result<()> {
    if db_path.is_empty() {
        anyhow::bail!("Database path cannot be empty");
    }

    let path = Path::new(db_path);

    // Require absolute paths
    if !path.is_absolute() {
        anyhow::bail!(
            "Database path must be absolute: {}. Use std::fs::canonicalize() if needed.",
            db_path
        );
    }

    // Check file exists
    if !path.exists() {
        anyhow::bail!("Database file not found: {}", db_path);
    }

    // Ensure it's a file, not a directory
    if !path.is_file() {
        anyhow::bail!("Path is not a file: {}", db_path);
    }

    // TODO: Production enhancement - add allowlist/denylist validation
    // For example:
    // - Check against configured allowed directories
    // - Deny access to system directories
    // - Validate file extension (.db, .sqlite, .sqlite3)

    Ok(())
}

/// Generate system prompt with database path
fn generate_system_prompt(db_path: &str) -> String {
    format!(
        "You are a database analysis expert specialized in SQLite databases. \
         You are analyzing the database at: {}

Your role is to explore schemas, query data, and provide insights about \
database structure and contents. You have access to the sqlite3_reader tool \
which executes read-only SQL queries (SELECT and PRAGMA statements).

IMPORTANT: Always use db_path='{}' in your sqlite3_reader tool calls.

Schema Exploration Commands:
- PRAGMA table_list - List all tables
- PRAGMA table_info(table_name) - Get column details for a table
- PRAGMA foreign_key_list(table_name) - Get foreign key relationships
- SELECT sql FROM sqlite_master WHERE name='table_name' - Get CREATE statement

Best Practices:
1. Start by exploring the schema before querying data
2. Use appropriate LIMIT clauses for large result sets
3. Provide clear explanations of your findings
4. When counting rows, use COUNT(*) queries
5. Identify relationships between tables using foreign keys

Always base your analysis on actual database contents, not assumptions.",
        db_path, db_path
    )
}

/// Helper function to extract text from content blocks
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
```

#### 2.3 Export Module

**File**: `nocodo-agents/src/lib.rs`

Add module declaration:

```rust
pub mod codebase_analysis;
pub mod database;
pub mod factory;
pub mod sqlite_reader;  // ← ADD THIS
pub mod tools;
```

### Phase 3: Update Agent Factory

#### 3.1 Add SqliteReaderAgent to Factory

**File**: `nocodo-agents/src/factory.rs`

Add factory method for creating SqliteReaderAgent:

```rust
use crate::sqlite_reader::SqliteReaderAgent;

impl AgentFactory {
    // ... existing methods

    /// Create a SqliteReaderAgent for analyzing a specific database
    pub fn create_sqlite_reader_agent(
        &self,
        db_path: String,
    ) -> anyhow::Result<SqliteReaderAgent> {
        SqliteReaderAgent::new(
            self.llm_client.clone(),
            self.database.clone(),
            self.tool_executor.clone(),
            db_path,
        )
    }
}
```

## Files Changed

### New Files
- `nocodo-agents/src/sqlite_reader/mod.rs` - SqliteReaderAgent implementation
- `nocodo-agents/tasks/add-sqlite-analysis-agent.md` - This task document

### Modified Files
- `nocodo-agents/src/lib.rs` - Add Sqlite3Reader to AgentTool, export sqlite_reader module, change Agent trait
- `nocodo-agents/src/tools/llm_schemas.rs` - Add sqlite3_reader tool schema
- `nocodo-agents/src/factory.rs` - Add factory method for SqliteReaderAgent
- `nocodo-agents/src/codebase_analysis/mod.rs` - Update system_prompt to return String

## Build & Quality Checks

### Compilation
```bash
cd nocodo-agents
cargo build
```

### Code Quality
```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

### Type Check
```bash
cargo check
```

## Usage Examples

### Basic Usage

```rust
use nocodo_agents::sqlite_reader::SqliteReaderAgent;
use nocodo_agents::{Agent, database::Database};
use nocodo_llm_sdk::client::LlmClient;
use manager_tools::ToolExecutor;
use std::sync::Arc;
use std::path::PathBuf;

// Setup components
let llm_client: Arc<dyn LlmClient> = /* ... */;
let database = Arc::new(Database::new(&PathBuf::from("agent-session.db"))?);
let tool_executor = Arc::new(ToolExecutor::new(PathBuf::from(".")));

// Create agent for specific database
let agent = SqliteReaderAgent::new(
    llm_client,
    database,
    tool_executor,
    "/absolute/path/to/data.db".to_string(),
)?;

// Ask questions
let answer = agent.execute("How many users are in the database?").await?;
println!("Answer: {}", answer);

let schema_info = agent.execute("What is the schema of the users table?").await?;
println!("Schema: {}", schema_info);
```

### Usage in Indistocks Project

```rust
// In Indistocks project
use nocodo_agents::factory::AgentFactory;
use nocodo_agents::Agent;

let factory = AgentFactory::new(/* config */)?;

let agent = factory.create_sqlite_reader_agent(
    "/Users/user/Projects/Indistocks/data/stocks.db".to_string()
)?;

let analysis = agent.execute(
    "Show me the top 10 stocks by trading volume in the last week"
).await?;
```

### Usage in Manager Project

```rust
// In manager project
use nocodo_agents::sqlite_reader::SqliteReaderAgent;

let agent = SqliteReaderAgent::new(
    llm_client,
    session_db,
    tool_executor,
    std::fs::canonicalize("./project-data.db")?.to_str().unwrap().to_string(),
)?;

let result = agent.execute("Summarize the data in this database").await?;
```

## Security Considerations

### Path Validation

The agent implements multiple security layers:

1. **Absolute paths only**: Prevents directory traversal attacks
2. **File existence check**: Ensures path points to valid file
3. **File type check**: Ensures path is a file, not a directory
4. **Path injection**: LLM cannot modify the database path during execution

### Production Enhancements

For production deployment, consider adding:

```rust
fn validate_db_path(db_path: &str) -> anyhow::Result<()> {
    // ... existing validation

    // Allowlist validation
    let allowed_dirs = vec!["/data/databases", "/home/user/projects"];
    let allowed = allowed_dirs.iter().any(|dir| db_path.starts_with(dir));
    if !allowed {
        anyhow::bail!("Database path not in allowed directories: {}", db_path);
    }

    // Deny system directories
    let denied_dirs = vec!["/etc", "/sys", "/proc", "/var"];
    let denied = denied_dirs.iter().any(|dir| db_path.starts_with(dir));
    if denied {
        anyhow::bail!("Cannot access system directories: {}", db_path);
    }

    // Validate file extension
    let valid_extensions = vec!["db", "sqlite", "sqlite3"];
    let extension = Path::new(db_path)
        .extension()
        .and_then(|e| e.to_str());

    if let Some(ext) = extension {
        if !valid_extensions.contains(&ext) {
            anyhow::bail!("Invalid database file extension: {}", ext);
        }
    }

    Ok(())
}
```

## Success Criteria

- [ ] Agent trait updated to return String for system_prompt
- [ ] CodebaseAnalysisAgent updated to work with new trait signature
- [ ] Sqlite3Reader added to AgentTool enum
- [ ] Sqlite3Reader tool schema registered in llm_schemas.rs
- [ ] SqliteReaderAgent module created with full implementation
- [ ] Database path validation implemented (absolute paths, existence, file type)
- [ ] Path injection working in execute_tool_call method
- [ ] System prompt includes database path
- [ ] Factory method added for creating SqliteReaderAgent
- [ ] Code compiles without errors
- [ ] No clippy warnings
- [ ] Code properly formatted
- [ ] Module exported in lib.rs

## Notes

- Testing deferred due to complexity of mocking multi-turn tool calling
- Agent follows same pattern as CodebaseAnalysisAgent for consistency
- Database path is injected at tool execution time, LLM cannot override it
- System prompt is pre-computed at construction time with database path
- The agent uses a separate tracking database (for sessions) vs the analyzed database
- Production deployments should implement path allowlist/denylist validation
