# Manager Tools

A robust tool execution library for AI agents, providing type-safe execution of filesystem operations, shell commands, code patches, and user interactions.

## Overview

`manager-tools` provides a unified interface for executing various development tools through the `ToolExecutor` struct. All tools use typed request/response patterns defined in `manager-models`, ensuring compile-time safety and structured error handling.

## Features

- **Type-safe tool execution** - All tools use typed `ToolRequest` → `ToolResponse` pattern
- **File size limits** - Configurable maximum file size for read operations
- **Bash permission checking** - Trait-based permission system for command execution
- **Path validation** - Automatic path sanitization and validation
- **Structured errors** - Rich error types with context
- **Async execution** - All tools are async-compatible

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
manager-tools = { path = "../manager-tools" }
manager-models = { path = "../manager-models" }
```

## Quick Start

```rust
use manager_tools::ToolExecutor;
use manager_models::{ToolRequest, ToolResponse};
use manager_models::tools::filesystem::ReadFileRequest;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create tool executor
    let executor = ToolExecutor::new(PathBuf::from("/workspace"))
        .with_max_file_size(10 * 1024 * 1024); // 10MB limit

    // Create a typed request
    let request = ToolRequest::ReadFile(ReadFileRequest {
        file_path: "src/main.rs".to_string(),
        offset: None,
        limit: None,
    });

    // Execute and get typed response
    let response = executor.execute(request).await?;

    match response {
        ToolResponse::ReadFile(r) => {
            println!("File contents: {}", r.content);
        }
        _ => unreachable!(),
    }

    Ok(())
}
```

## ToolExecutor API

### Constructor

```rust
pub fn new(base_path: PathBuf) -> Self
```

Creates a new `ToolExecutor` with the specified base path. All relative file paths in tool requests will be resolved relative to this path.

**Default configuration:**
- `max_file_size`: 1MB
- `bash_executor`: None (uses default bash execution)

### Builder Methods

#### `with_max_file_size`

```rust
pub fn with_max_file_size(mut self, max_size: u64) -> Self
```

Sets the maximum file size (in bytes) for read operations. Files larger than this limit will return an error.

**Example:**
```rust
let executor = ToolExecutor::new(base_path)
    .with_max_file_size(10 * 1024 * 1024); // 10MB
```

#### `with_bash_executor`

```rust
pub fn with_bash_executor(
    mut self,
    bash_executor: Box<dyn BashExecutorTrait + Send + Sync>,
) -> Self
```

Sets a custom bash executor that implements permission checking and command validation.

**Example:**
```rust
use manager_tools::BashExecutorTrait;

struct MyBashExecutor;

impl BashExecutorTrait for MyBashExecutor {
    fn can_execute(&self, command: &str) -> bool {
        // Custom permission logic
        !command.contains("rm -rf")
    }
}

let executor = ToolExecutor::new(base_path)
    .with_bash_executor(Box::new(MyBashExecutor));
```

#### Builder Pattern

```rust
pub fn builder() -> ToolExecutorBuilder
```

Creates a builder for configuring a ToolExecutor with custom settings, including bash permissions.

**Example:**
```rust
use manager_tools::{ToolExecutor, bash::{BashExecutor, BashPermissions}};

let executor = ToolExecutor::builder()
    .base_path(PathBuf::from("."))
    .max_file_size(10 * 1024 * 1024)
    .bash_executor(Some(Box::new(
        BashExecutor::with_default_permissions()?
    )))
    .build();
```

### Core Methods

#### `execute`

```rust
pub async fn execute(&self, request: ToolRequest) -> Result<ToolResponse>
```

Executes a typed tool request and returns a typed response.

**Supported tools:**
- `ToolRequest::ListFiles` - List files matching a pattern
- `ToolRequest::ReadFile` - Read file contents
- `ToolRequest::WriteFile` - Write or create a file
- `ToolRequest::Grep` - Search for patterns in files
- `ToolRequest::ApplyPatch` - Apply patches to multiple files
- `ToolRequest::Bash` - Execute bash commands
- `ToolRequest::AskUser` - Ask user questions (interactive)
- `ToolRequest::Sqlite3Reader` - Execute read-only SQL queries on SQLite databases

**Example:**
```rust
use manager_models::tools::filesystem::ListFilesRequest;

let request = ToolRequest::ListFiles(ListFilesRequest {
    pattern: Some("**/*.rs".to_string()),
    path: Some("src".to_string()),
});

let response = executor.execute(request).await?;

if let ToolResponse::ListFiles(files) = response {
    for file in files.files {
        println!("Found: {}", file);
    }
}
```

#### `execute_from_json`

```rust
pub async fn execute_from_json(&self, json_request: Value) -> Result<Value>
```

Executes a tool from a JSON value. Useful for LLM integration where tools are called with JSON arguments.

**Example:**
```rust
use serde_json::json;

let json_request = json!({
    "ReadFile": {
        "file_path": "Cargo.toml",
        "offset": null,
        "limit": null
    }
});

let json_response = executor.execute_from_json(json_request).await?;
```

## Available Tools

### Filesystem Tools

#### ListFiles
List files and directories matching a pattern.

```rust
use manager_models::tools::filesystem::ListFilesRequest;

let request = ToolRequest::ListFiles(ListFilesRequest {
    pattern: Some("*.toml".to_string()),
    path: None, // Current directory
});
```

#### ReadFile
Read the contents of a file.

```rust
use manager_models::tools::filesystem::ReadFileRequest;

let request = ToolRequest::ReadFile(ReadFileRequest {
    file_path: "README.md".to_string(),
    offset: Some(0),    // Start from line 0
    limit: Some(100),   // Read 100 lines
});
```

#### WriteFile
Write or create a file.

```rust
use manager_models::tools::filesystem::WriteFileRequest;

let request = ToolRequest::WriteFile(WriteFileRequest {
    file_path: "output.txt".to_string(),
    content: "Hello, world!".to_string(),
    create_directories: true,
});
```

### Search Tools

#### Grep
Search for patterns in files using ripgrep-style search.

```rust
use manager_models::GrepRequest;

let request = ToolRequest::Grep(GrepRequest {
    pattern: "fn main".to_string(),
    file_type: Some("rust".to_string()),
    glob: None,
    case_insensitive: Some(false),
    max_matches: Some(10),
});
```

### Code Modification Tools

#### ApplyPatch
Apply patches to create, modify, delete, or move multiple files.

```rust
use manager_models::tools::filesystem::{ApplyPatchRequest, FileOperation};

let request = ToolRequest::ApplyPatch(ApplyPatchRequest {
    operations: vec![
        FileOperation::Create {
            path: "new_file.rs".to_string(),
            content: "// New file\n".to_string(),
        },
        FileOperation::Modify {
            path: "existing.rs".to_string(),
            old_content: "old code".to_string(),
            new_content: "new code".to_string(),
        },
    ],
});
```

### Execution Tools

#### Bash
Execute bash commands with timeout and permission checking.

```rust
use manager_models::BashRequest;

let request = ToolRequest::Bash(BashRequest {
    command: "ls -la".to_string(),
    timeout_ms: Some(5000), // 5 second timeout
});
```

**Note:** Requires a bash executor to be configured for permission checking.

## Bash Tool Configuration

### Default Permissions

By default, the bash tool allows safe commands like `ls`, `cat`, `git`, `npm`, etc. and denies dangerous commands like `rm -rf /`, `sudo`, etc.

### Custom Permissions

You can create agents with restricted bash access using the builder pattern:

#### Example: Tesseract-Only Agent

```rust
use manager_tools::{ToolExecutor, bash::{BashExecutor, BashPermissions}};
use std::path::PathBuf;

// Create permissions that only allow tesseract command
let perms = BashPermissions::minimal(vec!["tesseract"]);

// Create bash executor with restricted permissions
let bash_executor = BashExecutor::new(perms, 120)?;

// Create tool executor with custom bash
let tool_executor = ToolExecutor::builder()
    .base_path(PathBuf::from("."))
    .bash_executor(Some(Box::new(bash_executor)))
    .build();

// Use with agent
let agent = TesseractAgent::new(/* ... */, tool_executor);
```

#### Example: Read-Only Bash

```rust
use manager_tools::bash::BashPermissions;

// Only allow read operations
let perms = BashPermissions::read_only();
let bash_executor = BashExecutor::new(perms, 120)?;
```

#### Example: Multiple Specific Commands

```rust
use manager_tools::bash::BashPermissions;

// Allow tesseract and convert commands (for image processing agent)
let perms = BashPermissions::only_allow(vec!["tesseract*", "convert*"]);
let bash_executor = BashExecutor::new(perms, 120)?;
```

#### Example: Disable Bash Tool

```rust
// Create tool executor without bash access
let tool_executor = ToolExecutor::builder()
    .base_path(PathBuf::from("."))
    .bash_executor(None)  // No bash tool
    .build();
```

### Permission Helper Methods

#### `BashPermissions::minimal(commands)`
Creates the most restrictive permissions - only allows the exact commands specified.

```rust
// Only allow tesseract command
let perms = BashPermissions::minimal(vec!["tesseract"]);
```

#### `BashPermissions::only_allow(commands)`
Allows specific commands but denies everything else.

```rust
// Allow multiple commands with patterns
let perms = BashPermissions::only_allow(vec!["tesseract*", "convert*"]);
```

#### `BashPermissions::read_only()`
Allows only read-safe commands (ls, cat, grep, etc.).

```rust
let perms = BashPermissions::read_only();
```

#### `BashPermissions::default()`
Provides a balanced set of safe commands for general development.

```rust
let perms = BashPermissions::default();
```

### Security Notes

- Command restrictions are evaluated using glob patterns
- First matching rule wins (allow or deny)
- Always add a catch-all deny rule at the end for restricted executors
- Sandboxing (via Codex) still applies regardless of permissions
- Working directory restrictions are also enforced

### User Interaction Tools

#### AskUser
Ask user questions and get responses.

```rust
use manager_models::tools::user_interaction::{AskUserRequest, Question};

let request = ToolRequest::AskUser(AskUserRequest {
    questions: vec![
        Question {
            id: "confirm".to_string(),
            question: "Do you want to proceed?".to_string(),
            options: vec!["Yes".to_string(), "No".to_string()],
        },
    ],
});
```

### Database Tools

#### Sqlite3Reader
Execute read-only SQL queries on SQLite databases. Only SELECT queries and PRAGMA statements are allowed.

```rust
use manager_models::Sqlite3ReaderRequest;

let request = ToolRequest::Sqlite3Reader(Sqlite3ReaderRequest {
    db_path: "/path/to/database.db".to_string(),
    query: "SELECT * FROM users LIMIT 10".to_string(),
    limit: Some(10),
});
```

**Schema introspection with PRAGMA:**
```rust
// List all tables
let request = ToolRequest::Sqlite3Reader(Sqlite3ReaderRequest {
    db_path: "/path/to/database.db".to_string(),
    query: "SELECT name FROM sqlite_master WHERE type='table'".to_string(),
    limit: None,
});

// Get table schema
let request = ToolRequest::Sqlite3Reader(Sqlite3ReaderRequest {
    db_path: "/path/to/database.db".to_string(),
    query: "PRAGMA table_info(users)".to_string(),
    limit: None,
});
```

**Security:**
- Only SELECT queries and PRAGMA statements are allowed
- Blocks INSERT, UPDATE, DELETE, DROP, and other write operations
- SQL injection protection via AST parsing
- Automatic row limiting (default: 100, max: 1000)
- Path validation to ensure database file exists

## Error Handling

All tool operations return `anyhow::Result<ToolResponse>`. Errors include context about what went wrong:

```rust
match executor.execute(request).await {
    Ok(ToolResponse::ReadFile(response)) => {
        println!("Success: {}", response.content);
    }
    Ok(ToolResponse::Error(err)) => {
        eprintln!("Tool error: {}", err.message);
    }
    Err(e) => {
        eprintln!("Execution error: {}", e);
    }
    _ => unreachable!(),
}
```

## Integration with AI Agents

`manager-tools` is designed to work seamlessly with AI agents through the `manager-models` type system:

```rust
use manager_tools::ToolExecutor;
use manager_models::{ToolRequest, ToolResponse};

// Agent receives tool call from LLM
let tool_name = "read_file";
let arguments = json!({
    "file_path": "src/main.rs",
    "offset": null,
    "limit": null
});

// Parse into typed request
let request: ToolRequest = match tool_name {
    "read_file" => {
        let req: ReadFileRequest = serde_json::from_value(arguments)?;
        ToolRequest::ReadFile(req)
    }
    _ => anyhow::bail!("Unknown tool"),
};

// Execute with type safety
let response = executor.execute(request).await?;

// Format response for LLM
let result_text = match response {
    ToolResponse::ReadFile(r) => format!("File contents:\n{}", r.content),
    _ => "Unexpected response".to_string(),
};
```

## Best Practices

### 1. Configure File Size Limits
Always set appropriate file size limits to prevent memory issues:

```rust
let executor = ToolExecutor::new(base_path)
    .with_max_file_size(10 * 1024 * 1024); // 10MB for most cases
```

### 2. Use Path Validation
The executor automatically validates and sanitizes paths, but always use relative paths when possible:

```rust
// Good
ReadFileRequest { file_path: "src/main.rs".to_string(), .. }

// Avoid
ReadFileRequest { file_path: "/absolute/path/to/file".to_string(), .. }
```

### 3. Handle All Response Types
Always pattern match on the expected response type and handle errors:

```rust
match executor.execute(request).await? {
    ToolResponse::ReadFile(r) => { /* handle success */ }
    ToolResponse::Error(e) => { /* handle tool error */ }
    _ => anyhow::bail!("Unexpected response type"),
}
```

### 4. Implement Permission Checking for Bash
Always use a custom bash executor with permission checking in production:

```rust
struct SafeBashExecutor {
    allowed_commands: Vec<String>,
}

impl BashExecutorTrait for SafeBashExecutor {
    fn can_execute(&self, command: &str) -> bool {
        self.allowed_commands.iter().any(|allowed| command.starts_with(allowed))
    }
}
```

## Dependencies

- `manager-models` - Type definitions for tool requests and responses
- `anyhow` - Error handling
- `tokio` - Async runtime
- `serde_json` - JSON serialization
- `walkdir` - Directory traversal
- `regex` - Pattern matching
- `rusqlite` - SQLite database access
- `sqlparser` - SQL parsing and validation

## Related Crates

- **manager-models**: Type definitions for tools (`ToolRequest`, `ToolResponse`)
- **nocodo-agents**: AI agents that use manager-tools for execution
- **nocodo-llm-sdk**: LLM client integration

## License

See the workspace LICENSE file.
