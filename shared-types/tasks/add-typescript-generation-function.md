# Add TypeScript Generation Library Function

## Overview

Add a public library function to `shared-types` that generates TypeScript definitions for requested types. This enables the `StructuredJsonAgent` to dynamically generate only the types it needs at runtime without any file system dependencies.

## Objectives

1. Add a public function `generate_typescript_definitions()` that:
   - Accepts a list of type names
   - Returns TypeScript definitions as a string
   - Provides clear error messages for unknown types
   - Cleans output (removes imports and generation comments)

2. Keep the existing `generate_api_types` binary for GUI usage (unchanged)

## Architecture

### Module Structure

```
shared-types/
├── src/
│   ├── lib.rs                        # Add public function here
│   ├── typescript_gen.rs             # NEW: Type generation module
│   └── bin/
│       └── generate_api_types.rs     # Existing (GUI-specific, unchanged)
└── Cargo.toml                         # No changes needed
```

### Key Differences from `generate_api_types` Binary

| Feature | `generate_api_types` (binary) | `generate_typescript_definitions` (function) |
|---------|-------------------------------|---------------------------------------------|
| **Type** | Binary executable | Library function |
| **Purpose** | GUI API types → file | Agent type definitions → string |
| **Output** | Single combined file | String (in-memory) |
| **Location** | `../gui/api-types/types.ts` | Returned from function |
| **Types** | API-specific subset | Requested types only |
| **Usage** | GUI TypeScript code (build-time) | Agent runtime type loading |
| **Dependencies** | File system | None (in-memory) |

## Implementation

### Step 1: Create TypeScript Generation Module

**File**: `shared-types/src/typescript_gen.rs`

```rust
use ts_rs::TS;

/// Generate TypeScript definitions for requested types
///
/// # Arguments
/// * `type_names` - Slice of type names to generate definitions for
///
/// # Returns
/// Combined TypeScript definitions as a string
///
/// # Errors
/// Returns an error if a requested type is unknown or export fails
///
/// # Example
/// ```
/// use shared_types::generate_typescript_definitions;
///
/// let types = vec!["PMProject", "Workflow", "WorkflowStep"];
/// let definitions = generate_typescript_definitions(&types)?;
/// // definitions contains TypeScript interface definitions
/// ```
pub fn generate_typescript_definitions(type_names: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    if type_names.is_empty() {
        return Err("No type names provided".into());
    }

    let mut definitions = Vec::new();

    for name in type_names {
        let type_def = export_type(name)?;
        let cleaned = clean_type(type_def);

        if !cleaned.trim().is_empty() {
            definitions.push(cleaned);
        }
    }

    Ok(definitions.join("\n\n"))
}

/// Export a single type by name
fn export_type(name: &str) -> Result<String, Box<dyn std::error::Error>> {
    use crate::*;

    let result = match name {
        // Agent types
        "AgentInfo" => AgentInfo::export_to_string()?,
        "AgentConfig" => AgentConfig::export_to_string()?,
        "SqliteAgentConfig" => SqliteAgentConfig::export_to_string()?,
        "CodebaseAnalysisAgentConfig" => CodebaseAnalysisAgentConfig::export_to_string()?,
        "TesseractAgentConfig" => TesseractAgentConfig::export_to_string()?,
        "AgentExecutionRequest" => AgentExecutionRequest::export_to_string()?,
        "AgentsResponse" => AgentsResponse::export_to_string()?,
        "AgentExecutionResponse" => AgentExecutionResponse::export_to_string()?,

        // Session types
        "SessionMessage" => SessionMessage::export_to_string()?,
        "SessionToolCall" => SessionToolCall::export_to_string()?,
        "SessionResponse" => SessionResponse::export_to_string()?,
        "SessionListItem" => SessionListItem::export_to_string()?,
        "SessionListResponse" => SessionListResponse::export_to_string()?,

        // Project Management types
        "PMProject" => PMProject::export_to_string()?,
        "Workflow" => Workflow::export_to_string()?,
        "WorkflowStep" => WorkflowStep::export_to_string()?,
        "WorkflowWithSteps" => WorkflowWithSteps::export_to_string()?,
        "SaveWorkflowRequest" => SaveWorkflowRequest::export_to_string()?,
        "WorkflowStepData" => WorkflowStepData::export_to_string()?,

        // Auth types
        "User" => User::export_to_string()?,
        "Team" => Team::export_to_string()?,
        "Permission" => Permission::export_to_string()?,
        "PermissionItem" => PermissionItem::export_to_string()?,
        "LoginRequest" => LoginRequest::export_to_string()?,
        "LoginResponse" => LoginResponse::export_to_string()?,
        "CreateUserRequest" => CreateUserRequest::export_to_string()?,
        "UpdateUserRequest" => UpdateUserRequest::export_to_string()?,
        "UpdateTeamRequest" => UpdateTeamRequest::export_to_string()?,
        "UserResponse" => UserResponse::export_to_string()?,
        "UserListResponse" => UserListResponse::export_to_string()?,
        "UserListItem" => UserListItem::export_to_string()?,
        "TeamListResponse" => TeamListResponse::export_to_string()?,
        "TeamListItem" => TeamListItem::export_to_string()?,
        "TeamItem" => TeamItem::export_to_string()?,
        "UserInfo" => UserInfo::export_to_string()?,
        "UserWithTeams" => UserWithTeams::export_to_string()?,
        "UserDetailResponse" => UserDetailResponse::export_to_string()?,
        "CurrentUserTeamsResponse" => CurrentUserTeamsResponse::export_to_string()?,
        "AddAuthorizedSshKeyRequest" => AddAuthorizedSshKeyRequest::export_to_string()?,
        "AddAuthorizedSshKeyResponse" => AddAuthorizedSshKeyResponse::export_to_string()?,
        "SearchQuery" => SearchQuery::export_to_string()?,

        // Settings types
        "ApiKeyConfig" => ApiKeyConfig::export_to_string()?,
        "SettingsResponse" => SettingsResponse::export_to_string()?,
        "UpdateApiKeysRequest" => UpdateApiKeysRequest::export_to_string()?,

        // Error types
        "ErrorResponse" => ErrorResponse::export_to_string()?,

        // Unknown type
        _ => {
            return Err(format!(
                "Unknown type: '{}'. Available types can be found in shared-types/src/",
                name
            ).into());
        }
    };

    Ok(result)
}

/// Clean type definition by removing imports and generation comments
fn clean_type(mut type_def: String) -> String {
    // Remove Windows line endings
    type_def.retain(|c| c != '\r');

    let lines: Vec<&str> = type_def.lines().collect();

    // Filter out import statements and generation comments
    let filtered: Vec<&str> = lines
        .iter()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.starts_with("import type")
                && !trimmed.starts_with("// This file was generated")
        })
        .cloned()
        .collect();

    filtered.join("\n").trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_single_type() {
        let result = generate_typescript_definitions(&["PMProject"]).unwrap();
        assert!(result.contains("interface PMProject"));
        assert!(result.contains("id: number"));
    }

    #[test]
    fn test_generate_multiple_types() {
        let result = generate_typescript_definitions(&["PMProject", "Workflow"]).unwrap();
        assert!(result.contains("interface PMProject"));
        assert!(result.contains("interface Workflow"));
    }

    #[test]
    fn test_unknown_type_error() {
        let result = generate_typescript_definitions(&["NonExistentType"]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown type"));
    }

    #[test]
    fn test_empty_type_names() {
        let result = generate_typescript_definitions(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_cleaned_output() {
        let result = generate_typescript_definitions(&["PMProject"]).unwrap();
        // Should not contain import statements or generation comments
        assert!(!result.contains("import type"));
        assert!(!result.contains("This file was generated"));
    }
}
```

### Step 2: Export from Library

**File**: `shared-types/src/lib.rs`

Add the module and re-export the function:

```rust
// ... existing code ...

pub mod typescript_gen;

// Re-export the main function for convenience
pub use typescript_gen::generate_typescript_definitions;

// ... rest of existing code ...
```

## Usage

### From Agent Code (nocodo-agents)

```rust
use shared_types::generate_typescript_definitions;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate definitions for specific types
    let type_names = vec!["PMProject", "Workflow", "WorkflowStep"];
    let typescript_defs = generate_typescript_definitions(&type_names)?;

    println!("Generated TypeScript definitions:");
    println!("{}", typescript_defs);

    Ok(())
}
```

### Example Output

```typescript
export interface PMProject {
    id: number;
    name: string;
    description: string;
    created_at: number;
}

export interface Workflow {
    id: number;
    project_id: number;
    name: string;
    parent_workflow_id: number | null;
    branch_condition: string | null;
    created_at: number;
}

export interface WorkflowStep {
    id: number;
    workflow_id: number;
    step_number: number;
    description: string;
    created_at: number;
}
```

## Type Coverage

The function supports ALL types that have `#[derive(TS)]` in the `export_type()` match statement:

### Core Categories

1. **Agent Types** (8 types)
   - AgentInfo, AgentConfig, SqliteAgentConfig, CodebaseAnalysisAgentConfig, TesseractAgentConfig
   - AgentExecutionRequest, AgentExecutionResponse, AgentsResponse

2. **Session Types** (5 types)
   - SessionMessage, SessionToolCall, SessionResponse
   - SessionListItem, SessionListResponse

3. **Project Management Types** (6 types)
   - PMProject, Workflow, WorkflowStep, WorkflowWithSteps
   - SaveWorkflowRequest, WorkflowStepData

4. **Authentication Types** (19 types)
   - User, Team, Permission, PermissionItem, UserInfo, UserWithTeams
   - LoginRequest, LoginResponse
   - CreateUserRequest, UpdateUserRequest, UpdateTeamRequest
   - UserResponse, UserListResponse, UserListItem, TeamListResponse, TeamListItem, TeamItem
   - UserDetailResponse, CurrentUserTeamsResponse
   - AddAuthorizedSshKeyRequest, AddAuthorizedSshKeyResponse, SearchQuery

5. **Settings Types** (3 types)
   - ApiKeyConfig, SettingsResponse, UpdateApiKeysRequest

6. **Error Types** (1 type)
   - ErrorResponse

**Total: ~42 types**

### Adding New Types

When adding a new type to `shared-types`:

1. Add `#[derive(TS)]` and `#[ts(export)]` to the type
2. Add a match arm in `export_type()` function in `typescript_gen.rs`:
   ```rust
   "MyNewType" => MyNewType::export_to_string()?,
   ```
3. The type is now available via `generate_typescript_definitions()`

## Testing

### Unit Tests

Tests are included in `typescript_gen.rs`:

```bash
cd shared-types
cargo test typescript_gen
```

Tests cover:
- Single type generation
- Multiple types generation
- Unknown type error handling
- Empty input error handling
- Clean output (no imports/comments)

### Integration Test

Create a simple test program:

```rust
use shared_types::generate_typescript_definitions;

fn main() {
    // Test valid types
    let result = generate_typescript_definitions(&["PMProject", "Workflow"]).unwrap();
    println!("Generated types:\n{}", result);

    // Test invalid type
    match generate_typescript_definitions(&["InvalidType"]) {
        Ok(_) => panic!("Should have failed"),
        Err(e) => println!("Expected error: {}", e),
    }
}
```

## Error Handling

The function handles these scenarios:

1. **Empty type list**: Returns error "No type names provided"
2. **Unknown type**: Returns error "Unknown type: 'TypeName'. Available types can be found in shared-types/src/"
3. **Export failures**: Propagates ts-rs export errors
4. **Empty exports**: Skipped automatically (after cleaning)

## Documentation

Add to `shared-types/README.md`:

```markdown
## TypeScript Type Generation

### For GUI (Build-time)

Generate API types for the GUI:

\`\`\`bash
cargo run --bin generate_api_types
\`\`\`

Output: `../gui/api-types/types.ts`

### For Agents (Runtime)

Use the library function to generate types on-demand:

\`\`\`rust
use shared_types::generate_typescript_definitions;

let types = vec!["PMProject", "Workflow", "WorkflowStep"];
let typescript_defs = generate_typescript_definitions(&types)?;
\`\`\`

This generates TypeScript definitions in-memory without file I/O.
```

## Acceptance Criteria

- [ ] Module `typescript_gen.rs` is created in `shared-types/src/`
- [ ] Function `generate_typescript_definitions()` is implemented
- [ ] Function is exported from `lib.rs`
- [ ] Function accepts `&[&str]` for type names
- [ ] Function returns `Result<String, Box<dyn std::error::Error>>`
- [ ] All ~42 types with `#[derive(TS)]` are supported
- [ ] Import statements are removed from output
- [ ] Generation comments are removed from output
- [ ] Empty type definitions are skipped automatically
- [ ] Unknown type errors include helpful message
- [ ] Unit tests cover all edge cases
- [ ] Tests pass: `cargo test typescript_gen`
- [ ] Documentation is added to README
- [ ] Function has comprehensive doc comments

## Benefits Over Binary Approach

1. **No file system**: Purely in-memory, no I/O overhead
2. **Always current**: No stale files, always matches code
3. **Type safe**: Compile-time verification
4. **Simpler**: Just call a function
5. **Faster**: No file reading/writing
6. **Testable**: Easy unit testing
7. **Flexible**: Generate only needed types

## Future Enhancements

1. **Auto-discovery**: Use macros or `inventory` crate to auto-discover types
2. **Validation**: Validate TypeScript syntax in tests
3. **Caching**: Cache generated definitions (if performance needed)
4. **Type graph**: Return dependency information
5. **Multiple formats**: Support different TypeScript dialects
6. **JSON Schema**: Generate JSON schemas in addition to TypeScript
