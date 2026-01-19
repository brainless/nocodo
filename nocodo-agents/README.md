# Nocodo Agents

A collection of AI agents for various software development tasks.

## Available Agents

| Agent | Description | Required Input |
|-------|-------------|----------------|
| **codebase-analysis** | Analyzes codebase structure, identifies architectural patterns, and provides insights about code organization | Path to codebase directory |
| **sqlite** | Analyzes SQLite databases, explores schema, and runs SQL queries to answer questions about the data | Path to SQLite database file |
| **tesseract** | Extracts text from images using Tesseract OCR, with AI-powered cleaning and formatting of the extracted content | Path to image file |
| **structured-json** | Generates structured JSON that conforms to specified TypeScript types, useful for creating type-safe data from natural language | TypeScript type names and domain description |

All agents share a common execution pattern:
- Session tracking in SQLite (messages, tool calls, results)
- Iterative LLM calls with tool execution
- Typed tool requests/responses via `nocodo-tools`

## Quick Start

### 1. Install the agent runner

```bash
cargo build --release --bin agent-runner
```

### 2. Configure API Keys

Copy the example config file and add your API keys:

```bash
cp config.example.toml config.toml
```

Edit `config.toml` and add your Z.AI API key:

```toml
[api_keys]
zai_api_key = "your-zai-api-key-here"
# Optional: Enable coding plan mode (default: true)
zai_coding_plan = true
```

Get your API key from [Z.AI](https://z.ai/).

### 3. Run an agent

```bash
./target/release/agent-runner \
  --agent codebase-analysis \
  --prompt "Analyze the structure of this Rust project" \
  --config config.toml
```

## Usage

The agent runner takes three required arguments:

- `--agent` or `-a`: The name of the agent to run (e.g., `codebase-analysis`)
- `--prompt` or `-p`: The user prompt for the agent
- `--config` or `-c`: Path to the configuration file containing API keys

Optional arguments:

- `--database-path`: Path to SQLite database for storing agent sessions (default: `~/.local/share/nocodo/agents.db`)
- `--base-path`: Base directory for tool execution (default: current directory)

Example:

```bash
cargo run --bin agent-runner -- \
  -a codebase-analysis \
  -p "What are the main components of this codebase?" \
  -c config.toml
```

### Advanced Usage

Customize the database and base path:

```bash
./target/release/agent-runner \
  --agent codebase-analysis \
  --prompt "Analyze the authentication system" \
  --config config.toml \
  --database-path ./my-agents.db \
  --base-path /path/to/project
```

## Default Configuration

The agent runner uses the following defaults:

- **LLM Provider**: Z.AI
- **Model**: GLM 4.6
- **Coding Plan Mode**: Enabled by default (uses `https://api.z.ai/api/coding/paas/v4`)
  - Set `zai_coding_plan = false` in config to use regular mode (`https://api.z.ai/api/paas/v4`)

Future versions will allow configuring the LLM provider and model through command-line arguments or the config file.

## Architecture

### Tool Execution

Agents use the `nocodo-tools` crate for executing development tools. All tool execution is:

- **Type-safe**: Uses typed `ToolRequest` → `ToolResponse` pattern from `manager-models`
- **Sandboxed**: All file operations are relative to the configured base path
- **Protected**: File size limits (default: 10MB) prevent memory issues
- **Persistent**: Tool calls and results are stored in SQLite for debugging

**Tool Executor Configuration:**

```rust
use nocodo_tools::ToolExecutor;

let executor = ToolExecutor::new(base_path)
    .with_max_file_size(10 * 1024 * 1024);  // 10MB limit
```

The tool executor supports these tools:
- **list_files**: List files matching patterns (glob support)
- **read_file**: Read file contents with line offset/limit
- **write_file**: Create or modify files
- **grep**: Search for patterns in files (ripgrep-style)
- **apply_patch**: Apply multi-file patches
- **bash**: Execute shell commands (with permission checking)
- **ask_user**: Ask questions to gather information

### Database and Sessions

All agent executions are tracked in a SQLite database:

**Tables:**
- `agent_sessions` - Track agent runs, status, results
- `agent_messages` - Conversation history (user, assistant, tool messages)
- `agent_tool_calls` - Tool execution tracking with timing and results

**Default location**: `~/.local/share/nocodo/agents.db`

**Session lifecycle:**
1. Session created with `running` status
2. Messages and tool calls recorded during execution
3. Session marked as `completed` or `failed` with result/error

View session data:
```bash
sqlite3 ~/.local/share/nocodo/agents.db "SELECT * FROM agent_sessions ORDER BY started_at DESC LIMIT 5;"
```

### Agent Execution Flow

    1. **Initialize** - Create session in database
    2. **Loop** (max 10 iterations):
       - Build LLM request with conversation history and tool definitions
       - Call LLM and save assistant response
       - If tool calls present:
         - Parse LLM tool call → typed `ToolRequest`
         - Execute tool using `nocodo-tools::ToolExecutor`
         - Save typed `ToolResponse` to database
         - Add tool result to conversation history
         - Continue loop
       - If no tool calls:
         - Mark session as complete
         - Return result

## Development

### Adding New Agents

1. Create a new agent module in `src/`
2. Implement the `Agent` trait for your agent
3. Add the agent type to `AgentType` enum in `src/factory.rs`
4. Update the factory function to create your agent

See `src/codebase_analysis/mod.rs` for an example implementation.

**Example Agent Implementation:**

```rust
use crate::{Agent, AgentTool, database::Database};
use nocodo_tools::ToolExecutor;
use async_trait::async_trait;
use nocodo_llm_sdk::client::LlmClient;
use std::sync::Arc;

pub struct MyAgent {
    client: Arc<dyn LlmClient>,
    database: Arc<Database>,
    tool_executor: Arc<ToolExecutor>,
}

impl MyAgent {
    pub fn new(
        client: Arc<dyn LlmClient>,
        database: Arc<Database>,
        tool_executor: Arc<ToolExecutor>,
    ) -> Self {
        Self { client, database, tool_executor }
    }
}

#[async_trait]
impl Agent for MyAgent {
    fn objective(&self) -> &str {
        "Your agent's objective"
    }

    fn system_prompt(&self) -> &str {
        "System prompt with instructions for the LLM"
    }

    fn tools(&self) -> Vec<AgentTool> {
        vec![
            AgentTool::ListFiles,
            AgentTool::ReadFile,
            AgentTool::Grep,
        ]
    }

    async fn execute(&self, user_prompt: &str) -> anyhow::Result<String> {
        // Implement execution logic (see CodebaseAnalysisAgent for reference)
        todo!()
    }
}
```

### Tool Integration

Agents integrate with `nocodo-tools` through typed requests:

```rust
use manager_models::{ToolRequest, ToolResponse};
use manager_models::tools::filesystem::ReadFileRequest;

// Parse LLM tool call into typed request
let tool_request = AgentTool::parse_tool_call(
    tool_call.name(),
    tool_call.arguments().clone(),
)?;

// Execute with type safety
let tool_response = self.tool_executor
    .execute(tool_request)
    .await?;

// Format response for LLM
let result_text = match tool_response {
    ToolResponse::ReadFile(r) => format!("File contents:\n{}", r.content),
    ToolResponse::ListFiles(r) => format!("Found {} files:\n{}", r.files.len(), r.files),
    _ => "Unexpected response".to_string(),
};
```

### Testing

Run tests:
```bash
cargo test
```

Run a specific agent test:
```bash
cargo test --lib codebase_analysis
```

### Dependencies

Key dependencies:
- **nocodo-llm-sdk**: LLM client abstraction (ZAI, Claude, OpenAI)
- **manager-models**: Tool request/response types
- **manager-tools**: Tool execution engine
- **rusqlite**: SQLite database for session tracking
- **tokio**: Async runtime

## License

See the workspace LICENSE file.
