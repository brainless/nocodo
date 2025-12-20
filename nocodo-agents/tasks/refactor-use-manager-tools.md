# Task: Refactor nocodo-agents to Use manager-tools and manager-models

## Status
🔴 Not Started

## Priority
🔥 High - Blocks proper tool execution flow

## Overview
Remove duplicate tool implementations from nocodo-agents and refactor to use the existing, production-ready implementations from manager-tools and manager-models crates.

## Problem Statement

### Current State (WRONG)
- **nocodo-agents** has created duplicate tool implementations:
  - `src/tools/executor.rs` (287 lines) - duplicate ToolExecutor
  - `src/tools/schemas.rs` - duplicate parameter structs (ListFilesParams, ReadFileParams, etc.)
  - No dependency on `manager-tools` or `manager-models`

### What Should Be Used (CORRECT)
- **manager-models** (`/manager-models/src/`):
  - `ToolRequest` enum - Typed tool requests (ListFiles, ReadFile, Grep, Bash, etc.)
  - `ToolResponse` enum - Typed tool responses with success/error variants
  - Request/Response structs in `tools/filesystem.rs` and `tools/user_interaction.rs`

- **manager-tools** (`/manager-tools/src/`):
  - `ToolExecutor` struct (145 lines, production-ready)
  - Complete implementations for all tools
  - Bash permission checking via `BashExecutorTrait`
  - File size limits and path validation
  - Structured error handling with `ToolError` enum

### Why This Matters

| Current (nocodo-agents) | Correct (manager-tools) |
|------------------------|-------------------------|
| ❌ Untyped (String + JSON) | ✅ Typed enums (ToolRequest/Response) |
| ❌ No file size limits | ✅ max_file_size protection |
| ❌ Direct bash execution | ✅ Permission checking via traits |
| ❌ Basic error handling | ✅ Structured ToolError enum |
| ❌ Simple text replacement | ✅ Codex libraries for patches |
| ❌ 287 lines of duplicate code | ✅ Reuse existing 145 lines |

## Goals

1. ✅ Add manager-models and manager-tools as dependencies
2. ✅ Remove duplicate ToolExecutor implementation
3. ✅ Remove duplicate parameter structs (schemas.rs)
4. ✅ Update AgentTool enum to convert to ToolRequest
5. ✅ Refactor CodebaseAnalysisAgent to use typed tool execution
6. ✅ Update agent factory to pass manager-tools ToolExecutor
7. ✅ Update tests to use manager-models types
8. ✅ Verify all tools work with typed execution

## Implementation Plan

### Phase 1: Add Dependencies

#### 1.1 Update Cargo.toml
**File**: `nocodo-agents/Cargo.toml`

**Add these dependencies after line 10:**
```toml
[dependencies]
nocodo-llm-sdk = { path = "../nocodo-llm-sdk" }
manager-models = { path = "../manager-models" }  # NEW
manager-tools = { path = "../manager-tools" }    # NEW
anyhow = { workspace = true }
# ... rest of dependencies
```

**Remove these dependencies (no longer needed):**
```toml
# walkdir = "2.4"  # manager-tools handles this
# regex = "1.10"   # manager-tools handles this
```

#### 1.2 Build to Verify
```bash
cd nocodo-agents
cargo check
```

Expected: Should compile successfully with new dependencies.

---

### Phase 2: Remove Duplicate Code

#### 2.1 Delete Duplicate ToolExecutor
**Action**: Delete file `nocodo-agents/src/tools/executor.rs`

This 287-line file is completely replaced by `manager-tools::ToolExecutor`.

#### 2.2 Delete Duplicate Parameter Structs
**Action**: Delete file `nocodo-agents/src/tools/schemas.rs`

Parameter types are now provided by `manager-models::ToolRequest` variants.

#### 2.3 Update tools/mod.rs
**File**: `nocodo-agents/src/tools/mod.rs`

**Remove:**
```rust
pub mod executor;
pub mod schemas;
```

**Keep only:**
```rust
// Empty or remove this file entirely
```

#### 2.4 Build to See What Breaks
```bash
cargo check 2>&1 | grep error
```

Expected errors:
- `executor` module not found
- `schemas` module not found
- References to `ToolExecutor` broken
- References to parameter structs broken

---

### Phase 3: Update AgentTool Enum

#### 3.1 Add Conversion to ToolRequest
**File**: `nocodo-agents/src/lib.rs`

**Current code (lines 10-41):**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentTool {
    ListFiles,
    ReadFile,
    WriteFile,
    Grep,
    ApplyPatch,
    Bash,
    AskUser,
}

impl AgentTool {
    pub fn to_tool_definition(&self) -> nocodo_llm_sdk::tools::Tool {
        // Current implementation using schemas
    }
}
```

**Replace with:**
```rust
use manager_models::{ToolRequest, tools::*};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentTool {
    ListFiles,
    ReadFile,
    WriteFile,
    Grep,
    ApplyPatch,
    Bash,
    AskUser,
}

impl AgentTool {
    /// Convert AgentTool to nocodo-llm-sdk Tool definition for LLM
    pub fn to_tool_definition(&self) -> nocodo_llm_sdk::tools::Tool {
        // Get the tool name
        let name = match self {
            AgentTool::ListFiles => "list_files",
            AgentTool::ReadFile => "read_file",
            AgentTool::WriteFile => "write_file",
            AgentTool::Grep => "grep",
            AgentTool::ApplyPatch => "apply_patch",
            AgentTool::Bash => "bash",
            AgentTool::AskUser => "ask_user",
        };

        // Create tool definition using manager-models schemas
        // This requires adding a helper to convert manager-models structs to JSON schemas
        ToolRequest::to_llm_tool_definition(name)
    }

    /// Parse LLM tool call into typed ToolRequest
    pub fn parse_tool_call(
        name: &str,
        arguments: serde_json::Value,
    ) -> anyhow::Result<ToolRequest> {
        let request = match name {
            "list_files" => {
                let req: ListFilesRequest = serde_json::from_value(arguments)?;
                ToolRequest::ListFiles(req)
            }
            "read_file" => {
                let req: ReadFileRequest = serde_json::from_value(arguments)?;
                ToolRequest::ReadFile(req)
            }
            "write_file" => {
                let req: WriteFileRequest = serde_json::from_value(arguments)?;
                ToolRequest::WriteFile(req)
            }
            "grep" => {
                let req: GrepRequest = serde_json::from_value(arguments)?;
                ToolRequest::Grep(req)
            }
            "apply_patch" => {
                let req: ApplyPatchRequest = serde_json::from_value(arguments)?;
                ToolRequest::ApplyPatch(req)
            }
            "bash" => {
                let req: BashRequest = serde_json::from_value(arguments)?;
                ToolRequest::Bash(req)
            }
            "ask_user" => {
                let req: AskUserRequest = serde_json::from_value(arguments)?;
                ToolRequest::AskUser(req)
            }
            _ => anyhow::bail!("Unknown tool: {}", name),
        };

        Ok(request)
    }
}
```

#### 3.2 Add Helper to manager-models (Optional Enhancement)
**File**: `manager-models/src/lib.rs`

Add a method to convert ToolRequest to JSON Schema for LLM tool definitions:
```rust
impl ToolRequest {
    pub fn to_llm_tool_definition(tool_name: &str) -> nocodo_llm_sdk::tools::Tool {
        // Use schemars to generate JSON schema from the request struct
        // This is a helper to avoid duplication
        todo!("Implement JSON schema generation from manager-models structs")
    }
}
```

**Alternative**: Keep schema generation in nocodo-agents using `schemars` on manager-models types.

---

### Phase 4: Refactor CodebaseAnalysisAgent

#### 4.1 Update Agent Struct
**File**: `nocodo-agents/src/codebase_analysis/mod.rs`

**Current:**
```rust
use crate::tools::executor::ToolExecutor;

pub struct CodebaseAnalysisAgent {
    client: Arc<dyn LlmClient>,
    database: Arc<Database>,
    tool_executor: Arc<ToolExecutor>,  // nocodo-agents version
}
```

**Replace with:**
```rust
use manager_tools::ToolExecutor;

pub struct CodebaseAnalysisAgent {
    client: Arc<dyn LlmClient>,
    database: Arc<Database>,
    tool_executor: Arc<ToolExecutor>,  // manager-tools version
}
```

#### 4.2 Update execute_tool_call Method
**File**: `nocodo-agents/src/codebase_analysis/mod.rs`

**Current (lines ~650-699):**
```rust
async fn execute_tool_call(
    &self,
    session_id: i64,
    message_id: Option<i64>,
    tool_call: &nocodo_llm_sdk::tools::ToolCall,
) -> anyhow::Result<()> {
    // ... database recording ...

    // Execute tool with untyped arguments
    let result = self.tool_executor
        .execute(tool_call.name(), tool_call.arguments().clone())  // ❌ Untyped
        .await;

    // ... handle result ...
}
```

**Replace with:**
```rust
use crate::lib::AgentTool;
use manager_models::{ToolRequest, ToolResponse};

async fn execute_tool_call(
    &self,
    session_id: i64,
    message_id: Option<i64>,
    tool_call: &nocodo_llm_sdk::tools::ToolCall,
) -> anyhow::Result<()> {
    // 1. Parse LLM tool call into typed ToolRequest
    let tool_request = AgentTool::parse_tool_call(
        tool_call.name(),
        tool_call.arguments().clone(),
    )?;

    // 2. Record tool call in database
    let call_id = self.database.create_tool_call(
        session_id,
        message_id,
        tool_call.id(),
        tool_call.name(),
        tool_call.arguments().clone(),
    )?;

    // 3. Execute tool with typed request ✅
    let start = std::time::Instant::now();
    let result: Result<ToolResponse, manager_tools::ToolError> = self.tool_executor
        .execute(tool_request)  // ✅ Typed execution
        .await;
    let execution_time = start.elapsed().as_millis() as i64;

    // 4. Update database with typed result
    match result {
        Ok(response) => {
            // Convert ToolResponse to JSON for storage
            let response_json = serde_json::to_value(&response)?;
            self.database.complete_tool_call(call_id, response_json.clone(), execution_time)?;

            // Add tool result as a message for next LLM call
            let result_text = format_tool_response(&response);
            self.database.create_message(
                session_id,
                "tool",
                &format!("Tool {} result:\n{}", tool_call.name(), result_text),
            )?;
        }
        Err(e) => {
            let error_msg = format!("{:?}", e);
            self.database.fail_tool_call(call_id, &error_msg)?;

            // Send error back to LLM
            self.database.create_message(
                session_id,
                "tool",
                &format!("Tool {} failed: {}", tool_call.name(), error_msg),
            )?;
        }
    }

    Ok(())
}

/// Format ToolResponse for display to LLM
fn format_tool_response(response: &ToolResponse) -> String {
    match response {
        ToolResponse::ListFiles(r) => format!("Found {} files:\n{}", r.files.len(), r.files.join("\n")),
        ToolResponse::ReadFile(r) => format!("File contents ({} bytes):\n{}", r.content.len(), r.content),
        ToolResponse::WriteFile(r) => format!("Wrote {} bytes to {}", r.bytes_written, r.path),
        ToolResponse::Grep(r) => format!("Found {} matches:\n{:#?}", r.matches.len(), r.matches),
        ToolResponse::ApplyPatch(r) => format!("Applied patch: {:?}", r),
        ToolResponse::Bash(r) => format!("Exit code: {}\nStdout:\n{}\nStderr:\n{}", r.exit_code, r.stdout, r.stderr),
        ToolResponse::AskUser(r) => format!("User response: {}", r.response),
        ToolResponse::Error(e) => format!("Error: {}", e.message),
    }
}
```

---

### Phase 5: Update Factory

#### 5.1 Update Agent Factory
**File**: `nocodo-agents/src/factory.rs`

**Current:**
```rust
use crate::tools::executor::ToolExecutor;

pub fn create_agent_with_tools(
    agent_type: AgentType,
    client: Arc<dyn LlmClient>,
    database: Arc<Database>,
    tool_executor: Arc<ToolExecutor>,
) -> Box<dyn Agent> {
    // ...
}
```

**Replace with:**
```rust
use manager_tools::ToolExecutor;

pub fn create_agent_with_tools(
    agent_type: AgentType,
    client: Arc<dyn LlmClient>,
    database: Arc<Database>,
    tool_executor: Arc<ToolExecutor>,  // Now from manager-tools
) -> Box<dyn Agent> {
    match agent_type {
        AgentType::CodebaseAnalysis => {
            Box::new(CodebaseAnalysisAgent::new(client, database, tool_executor))
        }
    }
}
```

#### 5.2 Update Binary Runner
**File**: `nocodo-agents/bin/runner.rs`

**Current:**
```rust
use nocodo_agents::tools::executor::ToolExecutor;

// ...
let tool_executor = Arc::new(ToolExecutor::new(base_path));
```

**Replace with:**
```rust
use manager_tools::ToolExecutor;

// ...
// Create tool executor with manager-tools (supports more configuration)
let tool_executor = Arc::new(
    ToolExecutor::new(
        base_path.clone(),
        10 * 1024 * 1024,  // max_file_size: 10MB
        None,              // bash_executor: None (uses default)
    )
);
```

---

### Phase 6: Update Tests

#### 6.1 Update Test Imports
**File**: `nocodo-agents/src/codebase_analysis/tests.rs`

**Add imports:**
```rust
use manager_models::{ToolRequest, ToolResponse};
use manager_tools::ToolExecutor;
```

#### 6.2 Update Mock Implementations
If you have mock ToolExecutor for tests, replace with real manager-tools ToolExecutor or create mock using manager-tools traits.

```rust
#[tokio::test]
async fn test_agent_with_typed_tools() {
    let db = Database::new(&PathBuf::from(":memory:")).unwrap();
    let client = Arc::new(MockLlmClient::new());

    // Use real manager-tools ToolExecutor
    let tool_executor = Arc::new(ToolExecutor::new(
        PathBuf::from("."),
        10 * 1024 * 1024,
        None,
    ));

    let agent = CodebaseAnalysisAgent::new(client, Arc::new(db), tool_executor);

    let result = agent.execute("List all Rust files").await.unwrap();
    assert!(!result.is_empty());
}
```

---

### Phase 7: Handle Tool Definition Schemas

#### 7.1 Generate Tool Definitions from manager-models
Since we removed `schemas.rs`, we need to generate nocodo-llm-sdk Tool definitions from manager-models types.

**Option A: Add to manager-models**
Add a feature to manager-models to export JSON schemas:
```rust
// In manager-models/src/lib.rs
#[cfg(feature = "llm-schemas")]
pub mod llm_schemas {
    use schemars::{schema_for, JsonSchema};

    pub fn list_files_schema() -> serde_json::Value {
        let schema = schema_for!(crate::tools::ListFilesRequest);
        serde_json::to_value(schema).unwrap()
    }

    // ... similar for other tools
}
```

Then in manager-models/Cargo.toml:
```toml
[features]
llm-schemas = ["schemars"]

[dependencies]
schemars = { version = "0.8", optional = true }
```

**Option B: Keep schema generation in nocodo-agents**
Create `nocodo-agents/src/tools/llm_schemas.rs`:
```rust
use manager_models::tools::*;
use nocodo_llm_sdk::tools::{Tool, ToolBuilder};
use schemars::JsonSchema;

// Derive JsonSchema on manager-models types (requires adding schemars derives)
// OR manually create tool definitions

pub fn create_tool_definitions() -> Vec<Tool> {
    vec![
        ToolBuilder::new()
            .name("list_files")
            .description("List files matching a glob pattern")
            .parameters_schema(serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Glob pattern to match files"
                    },
                    "path": {
                        "type": "string",
                        "description": "Optional directory to search in"
                    }
                },
                "required": ["pattern"]
            }))
            .build(),
        // ... other tools
    ]
}
```

**Recommendation**: Use Option B initially (keep schemas in nocodo-agents) to avoid modifying manager-models. Can refactor to Option A later if schemas are needed elsewhere.

---

### Phase 8: Final Integration

#### 8.1 Update Agent trait tools() method
**File**: `nocodo-agents/src/lib.rs`

```rust
impl Agent for CodebaseAnalysisAgent {
    // ... other methods ...

    fn tools(&self) -> Vec<AgentTool> {
        vec![
            AgentTool::ListFiles,
            AgentTool::ReadFile,
            AgentTool::WriteFile,
            AgentTool::Grep,
            AgentTool::Bash,
        ]
    }
}

impl CodebaseAnalysisAgent {
    fn get_tool_definitions(&self) -> Vec<nocodo_llm_sdk::tools::Tool> {
        // Use the schema generation from Phase 7
        crate::tools::llm_schemas::create_tool_definitions()
    }
}
```

#### 8.2 Build and Test
```bash
cd nocodo-agents
cargo build --release
cargo test

# Run the agent
./target/release/agent-runner \
    --agent codebase-analysis \
    --prompt "List all Rust files in src/" \
    --config config.example.toml
```

---

## File Changes Summary

### Files to Modify
1. ✅ `Cargo.toml` - Add manager-models and manager-tools dependencies
2. ✅ `src/lib.rs` - Add AgentTool::parse_tool_call() method
3. ✅ `src/codebase_analysis/mod.rs` - Update execute_tool_call() to use typed ToolRequest
4. ✅ `src/factory.rs` - Update imports to use manager-tools::ToolExecutor
5. ✅ `bin/runner.rs` - Update ToolExecutor initialization
6. ✅ `src/codebase_analysis/tests.rs` - Update test imports and mocks

### Files to Delete
1. ❌ `src/tools/executor.rs` - 287 lines, replaced by manager-tools
2. ❌ `src/tools/schemas.rs` - Replaced by manager-models types

### Files to Create (Optional)
1. ➕ `src/tools/llm_schemas.rs` - Tool definition schemas for LLM (if not added to manager-models)

---

## Success Criteria

- [ ] nocodo-agents depends on manager-models and manager-tools
- [ ] No duplicate ToolExecutor implementation in nocodo-agents
- [ ] No duplicate parameter structs (schemas.rs removed)
- [ ] AgentTool::parse_tool_call() converts LLM calls to typed ToolRequest
- [ ] CodebaseAnalysisAgent::execute_tool_call() uses manager-tools::ToolExecutor
- [ ] Tool execution is type-safe (ToolRequest → ToolResponse)
- [ ] All tests pass with typed tool execution
- [ ] Binary runner works with manager-tools ToolExecutor
- [ ] Tool responses are properly formatted for LLM
- [ ] No compiler warnings about unused code

---

## Testing Plan

### Unit Tests
```bash
# Test agent tool parsing
cargo test test_agent_tool_parse_tool_call

# Test typed tool execution
cargo test test_typed_tool_execution

# Test tool response formatting
cargo test test_format_tool_response
```

### Integration Tests
```bash
# Test codebase analysis with real tools
cargo test test_codebase_analysis_integration

# Test tool execution loop
cargo test test_agent_execution_loop
```

### Manual Testing
```bash
# Test list_files tool
./target/release/agent-runner \
    --agent codebase-analysis \
    --prompt "List all Rust files" \
    --config config.example.toml

# Test read_file tool
./target/release/agent-runner \
    --agent codebase-analysis \
    --prompt "Read the contents of src/lib.rs" \
    --config config.example.toml

# Test grep tool
./target/release/agent-runner \
    --agent codebase-analysis \
    --prompt "Search for 'ToolExecutor' in the codebase" \
    --config config.example.toml
```

---

## Migration Notes

### Breaking Changes
1. **ToolExecutor API Change**:
   - Old: `execute(tool_name: &str, arguments: Value)`
   - New: `execute(request: ToolRequest) -> Result<ToolResponse, ToolError>`

2. **Constructor Change**:
   - Old: `ToolExecutor::new(base_path: PathBuf)`
   - New: `ToolExecutor::new(base_path: PathBuf, max_file_size: u64, bash_executor: Option<...>)`

3. **Error Types**:
   - Old: `anyhow::Result<Value>`
   - New: `Result<ToolResponse, manager_tools::ToolError>`

### Benefits Gained
1. ✅ Type safety - Compile-time verification of tool arguments
2. ✅ Production features - File size limits, bash permissions, proper error handling
3. ✅ Code reuse - No duplication, single source of truth
4. ✅ Consistency - Same tool behavior across manager and agents
5. ✅ Maintainability - Changes to tools only need to happen in one place

---

## Dependencies

### Crate Dependency Graph (After Refactor)
```
nocodo-agents
├── nocodo-llm-sdk
├── manager-models  ← NEW
└── manager-tools   ← NEW
    ├── manager-models
    └── other deps (codex, etc.)
```

### Version Compatibility
Ensure all crates use compatible versions:
- rusqlite: 0.37 (nocodo-agents) should match manager-tools if it uses rusqlite
- serde: Workspace version
- tokio: Compatible versions (1.x)

---

## References

- **manager-tools source**: `/Users/brainless/GitWorktrees/nocodo/agent-builder-mvp-manager/manager-tools/src/`
- **manager-models source**: `/Users/brainless/GitWorktrees/nocodo/agent-builder-mvp-manager/manager-models/src/`
- **Current nocodo-agents tools**: `/Users/brainless/GitWorktrees/nocodo/agent-builder-mvp-manager/nocodo-agents/src/tools/`
- **Task plan that led to duplication**: `nocodo-agents/tasks/implement-tool-execution-flow.md`

---

## Future Enhancements

After this refactor is complete:

1. **Shared Tool Definitions**: Move JSON schema generation to manager-models so all projects can use the same tool definitions
2. **Tool Permissions**: Leverage manager-tools bash permission system
3. **Tool Metrics**: Use manager-tools metrics tracking for agent sessions
4. **Advanced Features**: Apply patch tool using codex libraries, not simple text replacement
5. **Streaming**: Support streaming tool execution if manager-tools adds it

---

## Notes

- This refactor removes ~400 lines of duplicate code from nocodo-agents
- Type safety prevents runtime errors from malformed tool arguments
- manager-tools has been battle-tested in the manager crate
- This aligns nocodo-agents with the original goal: "use existing tools from manager-tools"
