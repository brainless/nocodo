# nocodo-github-actions

A library for parsing GitHub Actions workflows and extracting executable commands.

## Overview

This crate provides functionality to:
- Parse GitHub Actions workflow YAML files
- Extract `run` commands with their execution context
- Execute commands in isolated environments
- Integrate with nocodo manager for workflow management

## Features

- **Core Library**: GitHub Actions YAML parsing and command extraction
- **CLI Tool**: Standalone command-line interface for parsing and execution
- **Nocodo Integration**: Database storage and API integration with nocodo manager

## Usage

### Basic Parsing

```rust
use nocodo_github_actions::{WorkflowParser, WorkflowCommand};

let (info, commands) = WorkflowParser::parse_workflow_file(
    "path/to/workflow.yml",
    "path/to/project/root"
).await?;

println!("Found {} commands in workflow '{}'", commands.len(), info.name);
```

### Command Execution

```rust
use nocodo_github_actions::CommandExecutor;

let execution = CommandExecutor::execute_command(&command, Some(300)).await?;
println!("Command exited with code: {:?}", execution.exit_code);
```

### Nocodo Integration

```rust
use nocodo_github_actions::WorkflowService;

let service = WorkflowService::new(database_pool);
let response = service.scan_workflows("project-id", &project_path).await?;
```

## API Endpoints (when integrated with nocodo)

- `POST /api/projects/{id}/workflows/scan` - Scan workflows for a project
- `GET /api/projects/{id}/workflows/commands` - Get extracted commands
- `POST /api/projects/{project_id}/workflows/commands/{command_id}/execute` - Execute a command
- `GET /api/projects/{project_id}/workflows/commands/{command_id}/executions` - Get execution history

## CLI Usage

```bash
# Parse a workflow file
nocodo-github-actions parse --path .github/workflows/ci.yml

# Execute a command from a workflow
nocodo-github-actions execute --workflow .github/workflows/ci.yml --job test --step 0
```

## Building

```bash
# Build the library
cargo build

# Build with CLI
cargo build --features cli

# Build with nocodo integration
cargo build --features nocodo-integration

# Run tests
cargo test --features nocodo-integration
```

## License

MIT OR Apache-2.0