# Extract Tool Descriptions from Schema Metadata

## Objective

Refactor tool definition creation to extract descriptions from struct-level doc comments via `schemars` schema metadata, eliminating hardcoded descriptions and establishing a single source of truth in type definitions.

## Benefits

- **Single source of truth**: Tool descriptions live with type definitions
- **No duplication**: Any code using these tools gets consistent descriptions
- **Better maintainability**: Update doc comment once, reflected everywhere
- **Prevents drift**: Can't have mismatched descriptions across usages

## Current State

Tool descriptions are hardcoded in `manager/src/llm_agent.rs:1483-1514`:

```rust
make_tool(
    "list_files",
    "List files and directories in a given path",  // ← Hardcoded
    schema_for!(ListFilesRequest),
)
```

Struct-level doc comments already exist but are unused:

```rust
/// List files tool request  // ← Not being used!
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListFilesRequest { ... }
```

## Implementation Plan

### 1. Update `make_tool` helper in `manager/src/llm_agent.rs` (lines 1437-1447)

**Replace:**
```rust
let make_tool = |name: &str, description: &str, schema: schemars::schema::RootSchema| {
    let customized_schema = schema_provider.customize_schema(schema.schema.into());
    crate::llm_client::ToolDefinition {
        r#type: "function".to_string(),
        function: crate::llm_client::FunctionDefinition {
            name: name.to_string(),
            description: description.to_string(),
            parameters: serde_json::to_value(customized_schema).unwrap_or_default(),
        },
    }
};
```

**With:**
```rust
let make_tool = |name: &str, schema: schemars::schema::RootSchema| {
    // Extract description from schema metadata (struct doc comment)
    let description = schema.schema.metadata
        .as_ref()
        .and_then(|m| m.description.clone())
        .unwrap_or_else(|| format!("Tool: {}", name));

    let customized_schema = schema_provider.customize_schema(schema.schema.into());
    crate::llm_client::ToolDefinition {
        r#type: "function".to_string(),
        function: crate::llm_client::FunctionDefinition {
            name: name.to_string(),
            description,  // ← From schema metadata
            parameters: serde_json::to_value(customized_schema).unwrap_or_default(),
        },
    }
};
```

### 2. Simplify tool creation calls (lines 1483-1514)

**Before:**
```rust
vec![
    make_tool("list_files", "List files and directories...", schema_for!(ListFilesRequest)),
    make_tool("read_file", "Read the contents of a file", schema_for!(ReadFileRequest)),
    // etc.
]
```

**After:**
```rust
vec![
    make_tool("list_files", schema_for!(ListFilesRequest)),
    make_tool("read_file", schema_for!(ReadFileRequest)),
    make_tool("write_file", schema_for!(WriteFileRequest)),
    make_tool("grep", schema_for!(GrepRequest)),
    make_tool("apply_patch", schema_for!(ApplyPatchRequest)),
    make_tool("bash", schema_for!(BashRequest)),
]
```

### 3. Enhance struct-level doc comments in `manager-models/src/lib.rs`

Update existing minimal doc comments to be more descriptive:

**Lines 416-417:**
```rust
/// List files and directories in a given path
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListFilesRequest { ... }
```

**Lines 459-460:**
```rust
/// Read the contents of a file
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ReadFileRequest { ... }
```

**Lines 488-489:**
```rust
/// Write or modify a file. Supports two modes: 1) Full write with 'content' parameter, 2) Search & replace with 'search' and 'replace' parameters
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WriteFileRequest { ... }
```

**Lines 573-574:**
```rust
/// Search for patterns in files using grep
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GrepRequest { ... }
```

**Lines 689-690:** (Already good, no change needed)
```rust
/// Apply a patch to create, modify, delete, or move multiple files
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ApplyPatchRequest { ... }
```

**Lines 648-649:**
```rust
/// Execute bash commands with timeout and permission checking
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BashRequest { ... }
```

### 4. Check imports

Ensure `schemars::schema::Metadata` types are accessible (should already be available via existing imports).

## Testing & Validation

### Run in each affected crate:

**manager crate:**
```bash
cd manager
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
cargo build
```

**manager-models crate:**
```bash
cd manager-models
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
cargo build
```

### Verification:
1. Check that tool definitions sent to LLM still have descriptions
2. Verify descriptions match the struct doc comments
3. Test with actual LLM agent session to ensure tools work correctly

## Files Changed

- `manager/src/llm_agent.rs` - Refactor `make_tool` helper (~10 lines)
- `manager-models/src/lib.rs` - Enhance struct doc comments (6 structs)

## Code Conventions

- Follow existing doc comment style (single line for brief descriptions)
- Use `///` for doc comments (not `//`)
- Preserve existing indentation and formatting
- Keep fallback behavior (`format!("Tool: {}", name)`) for robustness
