# Task: Implement Tool Execution Flow for Nocodo Agents

## Status
🔴 Not Started

## Overview
Implement a complete tool execution flow for the nocodo-agents crate, enabling agents to use tools (list_files, read_file, grep, bash, etc.) during execution. This includes database persistence, tool call handling, and integration with the LLM SDK.

## Background

### Current State
- Agents define `tools()` method returning `Vec<AgentTool>` enum
- `CompletionRequest` has no tools field
- Agent `execute()` never passes tools to LLM
- TODO comment in code: `// TODO: Implement tool execution flow`

### Reference Implementation
Manager crate has working tool execution (currently disabled pending SDK integration):
- Tool definitions in `manager-models`
- Tool executor in `manager-tools`
- Database schema for tool call tracking
- Request/Response enum pattern with JSON tagging

## Goals

1. ✅ Add database persistence for tool calls and conversation history
2. ✅ Extend LLM SDK types to support tools
3. ✅ Convert `AgentTool` enum to actual tool definitions with schemas
4. ✅ Implement tool call detection and execution loop
5. ✅ Handle tool results and send back to LLM
6. ✅ Create reusable migration system for other projects

## Architecture

### Layer 1: Database Schema
Create SQLite3 schema for:
- Agent sessions (tracks provider, model, status)
- Messages (user, assistant, system, tool roles)
- Tool calls (request, response, execution tracking)

### Layer 2: Tool Definitions
Convert `AgentTool` enum to provider-agnostic tool specifications:
- JSON schemas for parameters
- Descriptions for LLM understanding
- Validation rules

### Layer 3: Tool Executor
Implement actual tool execution:
- Path validation and sandboxing
- Error handling and timeouts
- Result formatting

### Layer 4: Agent Execution Loop
Modify agent `execute()` to:
1. Build initial request with tools
2. Call LLM
3. Detect tool calls in response
4. Execute tools
5. Send results back to LLM
6. Loop until completion (max 10 iterations)

## Detailed Implementation Plan

### Phase 1: Database Foundation

#### 1.1 Create Database Module
**File**: `nocodo-agents/src/database/mod.rs`

```rust
use rusqlite::{Connection, params};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub type DbConnection = Arc<Mutex<Connection>>;

pub struct Database {
    pub(crate) connection: DbConnection,
}

impl Database {
    pub fn new(db_path: &PathBuf) -> anyhow::Result<Self> {
        // Ensure directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(db_path)?;
        conn.execute("PRAGMA foreign_keys = ON", [])?;

        let database = Database {
            connection: Arc::new(Mutex::new(conn)),
        };

        database.run_migrations()?;
        Ok(database)
    }

    fn run_migrations(&self) -> anyhow::Result<()> {
        // See Phase 1.2
    }
}
```

#### 1.2 Define Schema
**Tables to create** in `run_migrations()`:

```sql
-- Agent sessions (analogous to llm_agent_sessions)
CREATE TABLE IF NOT EXISTS agent_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_name TEXT NOT NULL,
    provider TEXT NOT NULL,       -- zai, anthropic, openai, etc.
    model TEXT NOT NULL,           -- glm-4.6, claude-sonnet-4-5, etc.
    system_prompt TEXT,
    user_prompt TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'running' CHECK (status IN ('running', 'completed', 'failed')),
    started_at INTEGER NOT NULL,
    ended_at INTEGER,
    result TEXT,                   -- Final output
    error TEXT                     -- Error message if failed
);

-- Messages in conversation
CREATE TABLE IF NOT EXISTS agent_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system', 'tool')),
    content TEXT NOT NULL,         -- JSON for complex messages, text for simple
    created_at INTEGER NOT NULL,
    FOREIGN KEY (session_id) REFERENCES agent_sessions (id) ON DELETE CASCADE
);

-- Tool call tracking
CREATE TABLE IF NOT EXISTS agent_tool_calls (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL,
    message_id INTEGER,            -- Which assistant message requested this
    tool_call_id TEXT NOT NULL,    -- From LLM response (for matching results)
    tool_name TEXT NOT NULL,
    request TEXT NOT NULL,         -- JSON parameters
    response TEXT,                 -- JSON result
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'executing', 'completed', 'failed')),
    created_at INTEGER NOT NULL,
    completed_at INTEGER,
    execution_time_ms INTEGER,
    error_details TEXT,
    FOREIGN KEY (session_id) REFERENCES agent_sessions (id) ON DELETE CASCADE,
    FOREIGN KEY (message_id) REFERENCES agent_messages (id) ON DELETE SET NULL
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_agent_messages_session_created
    ON agent_messages(session_id, created_at);
CREATE INDEX IF NOT EXISTS idx_agent_tool_calls_session
    ON agent_tool_calls(session_id);
CREATE INDEX IF NOT EXISTS idx_agent_tool_calls_status
    ON agent_tool_calls(session_id, status);
```

#### 1.3 Create Data Models
**File**: `nocodo-agents/src/models.rs`

```rust
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    pub id: i64,
    pub agent_name: String,
    pub provider: String,
    pub model: String,
    pub system_prompt: Option<String>,
    pub user_prompt: String,
    pub status: String,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub result: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub id: i64,
    pub session_id: i64,
    pub role: String,
    pub content: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolCall {
    pub id: i64,
    pub session_id: i64,
    pub message_id: Option<i64>,
    pub tool_call_id: String,
    pub tool_name: String,
    pub request: Value,
    pub response: Option<Value>,
    pub status: String,
    pub created_at: i64,
    pub completed_at: Option<i64>,
    pub execution_time_ms: Option<i64>,
    pub error_details: Option<String>,
}

impl AgentToolCall {
    pub fn complete(&mut self, response: Value, execution_time_ms: i64) {
        self.response = Some(response);
        self.status = "completed".to_string();
        self.completed_at = Some(chrono::Utc::now().timestamp());
        self.execution_time_ms = Some(execution_time_ms);
    }

    pub fn fail(&mut self, error: String) {
        self.status = "failed".to_string();
        self.error_details = Some(error);
        self.completed_at = Some(chrono::Utc::now().timestamp());
    }
}
```

#### 1.4 Database CRUD Operations
Add methods to `Database`:

```rust
// Session management
pub fn create_session(&self, agent_name: &str, provider: &str, model: &str,
                     system_prompt: Option<&str>, user_prompt: &str) -> anyhow::Result<i64>;
pub fn complete_session(&self, session_id: i64, result: &str) -> anyhow::Result<()>;
pub fn fail_session(&self, session_id: i64, error: &str) -> anyhow::Result<()>;

// Message management
pub fn create_message(&self, session_id: i64, role: &str, content: &str) -> anyhow::Result<i64>;
pub fn get_messages(&self, session_id: i64) -> anyhow::Result<Vec<AgentMessage>>;

// Tool call management
pub fn create_tool_call(&self, session_id: i64, message_id: Option<i64>,
                       tool_call_id: &str, tool_name: &str, request: Value) -> anyhow::Result<i64>;
pub fn complete_tool_call(&self, call_id: i64, response: Value, execution_time_ms: i64) -> anyhow::Result<()>;
pub fn fail_tool_call(&self, call_id: i64, error: &str) -> anyhow::Result<()>;
pub fn get_pending_tool_calls(&self, session_id: i64) -> anyhow::Result<Vec<AgentToolCall>>;
```

#### 1.5 Migration System for External Projects
**File**: `nocodo-agents/src/database/migrations.rs`

```rust
/// Run migrations on an external database connection
/// This allows other projects to use nocodo-agents migrations
pub fn run_agent_migrations(conn: &Connection) -> anyhow::Result<()> {
    conn.execute("PRAGMA foreign_keys = ON", [])?;

    // Create tables (same SQL as above)
    // ...

    Ok(())
}

/// Check if agent tables exist in database
pub fn has_agent_schema(conn: &Connection) -> anyhow::Result<bool> {
    let mut stmt = conn.prepare(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='agent_sessions'"
    )?;
    Ok(stmt.exists([])?)
}
```

**Usage in external projects**:
```rust
// In another project's main.rs or database setup
use nocodo_agents::database::migrations::run_agent_migrations;

let conn = Connection::open("my-app.db")?;
run_agent_migrations(&conn)?;
```

#### 1.6 Update Cargo.toml
Add dependencies:
```toml
[dependencies]
rusqlite = { version = "0.32", features = ["bundled"] }
chrono = { version = "0.4", features = ["serde"] }
# ... existing deps
```

### Phase 2: Extend LLM SDK Types

#### 2.1 Add Tools to CompletionRequest
**File**: `nocodo-llm-sdk/src/types.rs`

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    pub messages: Vec<Message>,
    pub max_tokens: u32,
    pub model: String,
    pub system: Option<String>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub stop_sequences: Option<Vec<String>>,
    pub tools: Option<Vec<crate::tools::Tool>>,      // NEW
    pub tool_choice: Option<crate::tools::ToolChoice>, // NEW
}
```

#### 2.2 Add Tool Calls to CompletionResponse
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub content: Vec<ContentBlock>,
    pub role: Role,
    pub usage: Usage,
    pub stop_reason: Option<String>,
    pub tool_calls: Option<Vec<crate::tools::ToolCall>>, // NEW
}
```

#### 2.3 Update ZaiGlmClient Implementation
**File**: `nocodo-llm-sdk/src/glm/zai/client.rs`

In `impl LlmClient for ZaiGlmClient`:
```rust
async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
    // ... existing message conversion ...

    // Convert tools to ZAI format
    let tools = request.tools.map(|tools| {
        tools.into_iter().map(|tool| {
            // Convert from generic Tool to GlmTool
            GlmToolFormat::to_provider_tool(&tool)
        }).collect()
    });

    let zai_request = ZaiChatCompletionRequest {
        // ... existing fields ...
        tools,
        tool_choice: request.tool_choice.map(|choice|
            GlmToolFormat::to_provider_tool_choice(&choice)
        ),
        // ...
    };

    let zai_response = self.create_chat_completion(zai_request).await?;

    // Extract tool calls from response
    let tool_calls = zai_response.choices[0]
        .message
        .tool_calls
        .as_ref()
        .map(|calls| {
            calls.iter().map(|call| {
                crate::tools::ToolCall::new(
                    call.id.clone(),
                    call.function.name.clone(),
                    serde_json::from_str(&call.function.arguments).unwrap_or_default(),
                )
            }).collect()
        });

    Ok(CompletionResponse {
        // ... existing fields ...
        tool_calls,
    })
}
```

### Phase 3: Tool Definitions and Schemas

#### 3.1 Create Tool Schema Generator
**File**: `nocodo-agents/src/tools/schemas.rs`

```rust
use nocodo_llm_sdk::tools::{Tool, ToolBuilder};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// List files tool parameters
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListFilesParams {
    /// Glob pattern to match files (e.g., "**/*.rs")
    pub pattern: String,
    /// Directory to search in (relative to base path)
    pub path: Option<String>,
}

/// Read file tool parameters
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ReadFileParams {
    /// Path to the file to read
    pub file_path: String,
    /// Optional line offset to start reading from
    pub offset: Option<usize>,
    /// Optional number of lines to read
    pub limit: Option<usize>,
}

/// Grep tool parameters
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GrepParams {
    /// Regular expression pattern to search for
    pub pattern: String,
    /// File type filter (e.g., "rs", "toml")
    #[serde(rename = "type")]
    pub file_type: Option<String>,
    /// Glob pattern to filter files
    pub glob: Option<String>,
    /// Case insensitive search
    #[serde(rename = "-i")]
    pub case_insensitive: Option<bool>,
}

// ... similar for Bash, ApplyPatch, AskUser ...

pub fn create_tool_definitions() -> Vec<Tool> {
    vec![
        ToolBuilder::<ListFilesParams>::new()
            .name("list_files")
            .description("List files matching a glob pattern in the codebase")
            .build(),

        ToolBuilder::<ReadFileParams>::new()
            .name("read_file")
            .description("Read the contents of a file")
            .build(),

        ToolBuilder::<GrepParams>::new()
            .name("grep")
            .description("Search for a pattern across files using ripgrep")
            .build(),

        // ... add all tools
    ]
}
```

#### 3.2 Convert AgentTool to Tool Definitions
**File**: `nocodo-agents/src/lib.rs`

```rust
impl AgentTool {
    pub fn to_tool_definition(&self) -> Tool {
        let all_tools = tools::schemas::create_tool_definitions();
        all_tools.into_iter()
            .find(|tool| tool.name() == self.name())
            .expect("Tool definition must exist")
    }
}

impl Agent {
    fn get_tool_definitions(&self) -> Vec<Tool> {
        self.tools()
            .into_iter()
            .map(|tool| tool.to_tool_definition())
            .collect()
    }
}
```

### Phase 4: Tool Executor

#### 4.1 Create Tool Executor Module
**File**: `nocodo-agents/src/tools/executor.rs`

```rust
use anyhow::Result;
use serde_json::Value;
use std::path::PathBuf;
use std::time::Instant;

pub struct ToolExecutor {
    base_path: PathBuf,
}

impl ToolExecutor {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    pub async fn execute(&self, tool_name: &str, arguments: Value) -> Result<Value> {
        let start = Instant::now();

        let result = match tool_name {
            "list_files" => self.execute_list_files(arguments).await,
            "read_file" => self.execute_read_file(arguments).await,
            "grep" => self.execute_grep(arguments).await,
            // ... other tools
            _ => Err(anyhow::anyhow!("Unknown tool: {}", tool_name)),
        };

        let execution_time = start.elapsed().as_millis() as i64;
        tracing::info!("Tool {} executed in {}ms", tool_name, execution_time);

        result
    }

    async fn execute_list_files(&self, args: Value) -> Result<Value> {
        // Use nocodo_llm_sdk::Glob or implement simplified version
        // Return JSON with file list
        todo!()
    }

    async fn execute_read_file(&self, args: Value) -> Result<Value> {
        // Use nocodo_llm_sdk::Read or implement simplified version
        // Return JSON with file contents
        todo!()
    }

    async fn execute_grep(&self, args: Value) -> Result<Value> {
        // Use nocodo_llm_sdk::Grep or implement simplified version
        // Return JSON with matches
        todo!()
    }
}
```

**Note**: Reuse logic from Claude Code's tool implementations if available, or create simplified versions.

### Phase 5: Agent Execution Loop

#### 5.1 Modify CodebaseAnalysisAgent::execute()
**File**: `nocodo-agents/src/codebase_analysis/mod.rs`

```rust
use crate::database::Database;
use crate::tools::executor::ToolExecutor;
use std::sync::Arc;

pub struct CodebaseAnalysisAgent {
    client: Arc<dyn LlmClient>,
    database: Arc<Database>,
    tool_executor: Arc<ToolExecutor>,
}

impl CodebaseAnalysisAgent {
    pub fn new(
        client: Arc<dyn LlmClient>,
        database: Arc<Database>,
        tool_executor: Arc<ToolExecutor>,
    ) -> Self {
        Self { client, database, tool_executor }
    }
}

#[async_trait]
impl Agent for CodebaseAnalysisAgent {
    // ... existing methods ...

    async fn execute(&self, user_prompt: &str) -> anyhow::Result<String> {
        // 1. Create session in database
        let session_id = self.database.create_session(
            "codebase-analysis",
            self.client.provider_name(),
            self.client.model_name(),
            Some(self.system_prompt()),
            user_prompt,
        )?;

        // 2. Create initial user message
        self.database.create_message(session_id, "user", user_prompt)?;

        // 3. Get tool definitions
        let tools = self.get_tool_definitions();

        // 4. Execution loop (max 10 iterations)
        let mut iteration = 0;
        let max_iterations = 10;

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
                system: Some(self.system_prompt().to_string()),
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
            let message_id = self.database.create_message(
                session_id,
                "assistant",
                &text,
            )?;

            // 8. Check for tool calls
            if let Some(tool_calls) = response.tool_calls {
                if tool_calls.is_empty() {
                    // No more tool calls, we're done
                    self.database.complete_session(session_id, &text)?;
                    return Ok(text);
                }

                // 9. Execute tools
                for tool_call in tool_calls {
                    self.execute_tool_call(
                        session_id,
                        Some(message_id),
                        &tool_call,
                    ).await?;
                }

                // Continue loop to send results back to LLM
            } else {
                // No tool calls in response, we're done
                self.database.complete_session(session_id, &text)?;
                return Ok(text);
            }
        }
    }
}

impl CodebaseAnalysisAgent {
    fn build_messages(&self, session_id: i64) -> anyhow::Result<Vec<Message>> {
        let db_messages = self.database.get_messages(session_id)?;

        db_messages.into_iter().map(|msg| {
            let role = match msg.role.as_str() {
                "user" => Role::User,
                "assistant" => Role::Assistant,
                "system" => Role::System,
                "tool" => Role::User, // Tool results sent as user messages
                _ => Role::User,
            };

            Ok(Message {
                role,
                content: vec![ContentBlock::Text { text: msg.content }],
            })
        }).collect()
    }

    async fn execute_tool_call(
        &self,
        session_id: i64,
        message_id: Option<i64>,
        tool_call: &nocodo_llm_sdk::tools::ToolCall,
    ) -> anyhow::Result<()> {
        // 1. Record tool call in database
        let call_id = self.database.create_tool_call(
            session_id,
            message_id,
            tool_call.id(),
            tool_call.name(),
            tool_call.arguments().clone(),
        )?;

        // 2. Execute tool
        let start = Instant::now();
        let result = self.tool_executor
            .execute(tool_call.name(), tool_call.arguments().clone())
            .await;
        let execution_time = start.elapsed().as_millis() as i64;

        // 3. Update database with result
        match result {
            Ok(response) => {
                self.database.complete_tool_call(call_id, response.clone(), execution_time)?;

                // 4. Add tool result as a message for next LLM call
                let result_text = serde_json::to_string_pretty(&response)?;
                self.database.create_message(
                    session_id,
                    "tool",
                    &format!("Tool {} result:\n{}", tool_call.name(), result_text),
                )?;
            }
            Err(e) => {
                self.database.fail_tool_call(call_id, &e.to_string())?;

                // Still send error back to LLM so it can handle it
                self.database.create_message(
                    session_id,
                    "tool",
                    &format!("Tool {} failed: {}", tool_call.name(), e),
                )?;
            }
        }

        Ok(())
    }
}
```

### Phase 6: Update Binary Runner

#### 6.1 Modify agent-runner Binary
**File**: `nocodo-agents/bin/runner.rs`

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Load config
    let config = load_config(&args.config)?;

    // Get API key and coding plan setting
    let zai_api_key = get_api_key(&config, "zai_api_key")
        .or_else(|_| get_api_key(&config, "ZAI_API_KEY"))?;
    let coding_plan = get_bool_option(&config, "zai_coding_plan", true);

    // Create ZAI GLM client
    let client = ZaiGlmClient::with_coding_plan(zai_api_key, coding_plan)?;
    let client: Arc<dyn nocodo_llm_sdk::client::LlmClient> = Arc::new(client);

    // Initialize database (use ~/.local/share/nocodo/agents.db or config option)
    let db_path = get_database_path(&config)?;
    let database = Arc::new(Database::new(&db_path)?);

    // Initialize tool executor (use current directory as base path or config option)
    let base_path = get_base_path(&config)?;
    let tool_executor = Arc::new(ToolExecutor::new(base_path));

    // Parse agent type
    let agent_type = match args.agent.to_lowercase().as_str() {
        "codebase-analysis" | "codebase_analysis" => AgentType::CodebaseAnalysis,
        _ => anyhow::bail!("Unknown agent type: {}", args.agent),
    };

    // Create agent with database and tool executor
    let agent = create_agent_with_tools(agent_type, client, database, tool_executor);

    println!("Running agent: {}", agent.objective());
    println!("User prompt: {}\n", args.prompt);

    // Execute agent
    let result = agent.execute(&args.prompt).await?;

    println!("\n--- Agent Result ---\n{}", result);

    Ok(())
}

fn get_database_path(config: &Config) -> anyhow::Result<PathBuf> {
    // Check config for database_path option
    // Otherwise use ~/.local/share/nocodo/agents.db
    Ok(PathBuf::from(std::env::var("HOME")?)
        .join(".local")
        .join("share")
        .join("nocodo")
        .join("agents.db"))
}

fn get_base_path(config: &Config) -> anyhow::Result<PathBuf> {
    // Check config for base_path option
    // Otherwise use current directory
    Ok(std::env::current_dir()?)
}
```

#### 6.2 Update Factory
**File**: `nocodo-agents/src/factory.rs`

```rust
use crate::database::Database;
use crate::tools::executor::ToolExecutor;

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
    }
}
```

### Phase 7: Testing

#### 7.1 Unit Tests
Create tests for:
- Database CRUD operations
- Tool schema generation
- Tool execution
- Message building from history

#### 7.2 Integration Tests
**File**: `nocodo-agents/tests/integration/tool_execution.rs`

```rust
#[tokio::test]
async fn test_agent_with_tools() {
    // Create in-memory database
    let db = Database::new(&PathBuf::from(":memory:")).unwrap();

    // Create mock client
    let client = Arc::new(MockLlmClient::with_tool_calls());

    // Create tool executor
    let tool_executor = Arc::new(ToolExecutor::new(PathBuf::from(".")));

    // Create agent
    let agent = CodebaseAnalysisAgent::new(
        client,
        Arc::new(db),
        tool_executor,
    );

    // Execute
    let result = agent.execute("List all Rust files").await.unwrap();

    // Verify tool calls were made and stored
    // Verify result contains expected information
}
```

## Dependencies to Add

```toml
[dependencies]
# Existing
nocodo-llm-sdk = { path = "../nocodo-llm-sdk" }
anyhow = { workspace = true }
async-trait = "0.1"
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"
clap = { version = "4.5", features = ["derive"] }

# New for tool execution
rusqlite = { version = "0.32", features = ["bundled"] }
chrono = { version = "0.4", features = ["serde"] }
tracing = "0.1"
schemars = { version = "0.8", features = ["preserve_order"] }
```

## Configuration Updates

Update `config.example.toml`:
```toml
[api_keys]
zai_api_key = "your-zai-api-key-here"
zai_coding_plan = true

[agent]
# Path to SQLite database for storing agent sessions
# Default: ~/.local/share/nocodo/agents.db
database_path = "/path/to/agent.db"

# Base path for tool execution (file operations are relative to this)
# Default: current directory
base_path = "/path/to/workspace"
```

## Success Criteria

- [ ] Database schema created and migrations work
- [ ] External projects can use migration system
- [ ] AgentTool enum converts to Tool definitions with schemas
- [ ] Tools field added to CompletionRequest/Response
- [ ] ZaiGlmClient passes tools to API and extracts tool calls
- [ ] Tool executor can execute all tool types
- [ ] Agent execution loop handles tool calls correctly
- [ ] Tool calls and results stored in database
- [ ] Integration tests pass
- [ ] Binary runner works with real ZAI API

## Future Enhancements

1. **Tool Permissions**: Add whitelist/blacklist for bash commands
2. **Streaming**: Support streaming tool execution progress
3. **Parallel Tool Execution**: Execute independent tools concurrently
4. **Tool Validation**: Validate tool arguments against schemas before execution
5. **Metrics**: Track tool usage statistics (most used, failure rates, avg execution time)
6. **Web UI**: Dashboard to view agent sessions and tool calls
7. **Export**: Export sessions to JSON for debugging/sharing

## References

- Manager tool execution: `/Users/brainless/GitWorktrees/nocodo/agent-builder-mvp-manager/manager/src/llm_agent.rs`
- Manager database schema: `/Users/brainless/GitWorktrees/nocodo/agent-builder-mvp-manager/manager/src/database/common.rs`
- Manager tool executor: `/Users/brainless/GitWorktrees/nocodo/agent-builder-mvp-manager/manager-tools/src/tool_executor.rs`
- LLM SDK tools: `/Users/brainless/GitWorktrees/nocodo/agent-builder-mvp-manager/nocodo-llm-sdk/src/tools/mod.rs`

## Notes

- Use web research for best practices on SQLite migrations in Rust
- Check rusqlite documentation for bundled feature and migration patterns
- Consider using refinery or diesel for more robust migrations if needed
- Keep database schema simple and focused on agent execution tracking
- Tool execution should be sandboxed and safe (path validation, timeouts, etc.)
