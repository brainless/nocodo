# Move Tool-Related Types from manager-models to manager-tools

**Status**: 📋 Not Started
**Priority**: Medium
**Created**: 2024-12-23

## Summary

Refactor the codebase to move all tool-related types from `manager-models` to `manager-tools`, establishing `manager-tools` as the single source of truth for tool definitions, requests, responses, and execution logic. Update `desktop-app` to import tool types directly from `manager-tools`.

## Problem Statement

Currently, tool-related types live in `manager-models`, but `manager-tools` is responsible for tool execution:

**Current Architecture:**
```
manager-models
  - ToolRequest, ToolResponse (types)
  - BashRequest, BashResponse
  - GrepRequest, GrepResponse
  - File operation types
  - User interaction types
        ↓
manager-tools (imports types from manager-models)
  - ToolExecutor (execution logic)
  - BashExecutor
  - Filesystem operations
  - Grep operations
        ↓
    ┌───────┴───────┐
    │               │
 manager      desktop-app
 (imports     (imports only
  both)       manager-models)
```

**Issues:**
- **Split responsibility**: Type definitions separated from their execution logic
- **Unclear ownership**: Tool types conceptually belong with tool execution
- **desktop-app doesn't use manager-tools**: Only imports types from manager-models
- **Architectural confusion**: Tool types mixed with domain models (Project, Work, User)

## Goals

1. **Establish manager-tools as tool owner**: All tool-related types live with execution logic
2. **Clean separation**: manager-models contains only domain models (Project, Work, User, Team, AiSession)
3. **Desktop-app uses manager-tools**: Direct import of tool types
4. **No circular dependencies**: Maintain clean dependency graph

## Architecture Changes

### Before
```
┌─────────────────┐
│ manager-models  │ (No dependencies)
│  - Project      │
│  - Work         │
│  - ToolRequest  │ ← Tool types mixed with domain
│  - BashRequest  │
└────────┬────────┘
         │
         ↓
┌─────────────────┐
│ manager-tools   │
│  - ToolExecutor │ ← Imports types from manager-models
│  - BashExecutor │
└────────┬────────┘
         │
         ↓
    ┌────────┴────────┐
    │                 │
┌───▼──────┐   ┌──────▼─────┐
│ manager  │   │ desktop-app │ ← Only imports manager-models
└──────────┘   └─────────────┘
```

### After
```
┌─────────────────┐
│ manager-models  │ (Pure domain models)
│  - Project      │
│  - Work         │
│  - User/Team    │
│  - AiSession    │
└─────────────────┘
         │
         ↓
┌─────────────────┐
│ manager-tools   │ (Tool types + execution)
│  - ToolRequest  │ ← All tool types here
│  - ToolResponse │
│  - ToolExecutor │
│  - BashExecutor │
└────────┬────────┘
         │
         ↓
    ┌────────┴────────┐
    │                 │
┌───▼──────┐   ┌──────▼─────┐
│ manager  │   │ desktop-app │ ← Imports both crates
└──────────┘   └─────────────┘
```

## Types to Move

### From `manager-models/src/lib.rs`:

**Core Tool Types (lines 347-544):**
- `ToolRequest` (enum)
- `ToolResponse` (enum)
- `GrepRequest`
- `GrepResponse`
- `GrepMatch`
- `BashRequest`
- `BashResponse`
- `ToolErrorResponse`

### From `manager-models/src/tools/filesystem.rs`:

**File Operation Types:**
- `ListFilesRequest`, `ListFilesResponse`
- `ReadFileRequest`, `ReadFileResponse`
- `WriteFileRequest`, `WriteFileResponse`
- `ApplyPatchRequest`, `ApplyPatchResponse`
- `ApplyPatchFileChange`
- `FileType`, `FileInfo`

### From `manager-models/src/tools/user_interaction.rs`:

**User Interaction Types:**
- `AskUserRequest`, `AskUserResponse`
- `UserQuestion`, `UserQuestionResponse`
- `QuestionType`, `QuestionValidation`

## Implementation Plan

### Phase 1: Prepare manager-tools Structure

#### 1.1 Create Type Modules in manager-tools

Create new module structure:
```
manager-tools/
  src/
    lib.rs
    types/           ← NEW
      mod.rs         ← Re-export all types
      core.rs        ← ToolRequest, ToolResponse
      bash.rs        ← BashRequest, BashResponse
      grep.rs        ← GrepRequest, GrepResponse, GrepMatch
      filesystem.rs  ← File operation types
      user_interaction.rs ← User interaction types
```

#### 1.2 Update manager-tools/Cargo.toml

Add dependency on manager-models:
```toml
[dependencies]
manager-models = { path = "../manager-models" }
serde = { version = "1.0", features = ["derive"] }
schemars = { version = "0.8", features = ["chrono"] }
# ... existing dependencies
```

### Phase 2: Move Types to manager-tools

#### 2.1 Copy Types from manager-models

**Create `manager-tools/src/types/core.rs`:**
```rust
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// Tool request enum containing all possible tool operations
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "tool", rename_all = "snake_case")]
pub enum ToolRequest {
    ListFiles(super::filesystem::ListFilesRequest),
    ReadFile(super::filesystem::ReadFileRequest),
    WriteFile(super::filesystem::WriteFileRequest),
    Grep(super::grep::GrepRequest),
    ApplyPatch(super::filesystem::ApplyPatchRequest),
    Bash(super::bash::BashRequest),
    AskUser(super::user_interaction::AskUserRequest),
}

/// Tool response enum containing all possible tool results
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "tool", rename_all = "snake_case")]
pub enum ToolResponse {
    ListFiles(super::filesystem::ListFilesResponse),
    ReadFile(super::filesystem::ReadFileResponse),
    WriteFile(super::filesystem::WriteFileResponse),
    Grep(super::grep::GrepResponse),
    ApplyPatch(super::filesystem::ApplyPatchResponse),
    Bash(super::bash::BashResponse),
    AskUser(super::user_interaction::AskUserResponse),
    Error(ToolErrorResponse),
}

/// Error response for tool execution failures
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolErrorResponse {
    pub error: String,
}
```

**Create `manager-tools/src/types/bash.rs`:**
```rust
// Copy BashRequest and BashResponse from manager-models/src/lib.rs (lines 442-536)
```

**Create `manager-tools/src/types/grep.rs`:**
```rust
// Copy GrepRequest, GrepResponse, GrepMatch from manager-models/src/lib.rs (lines 367-524)
```

**Create `manager-tools/src/types/filesystem.rs`:**
```rust
// Copy all filesystem types from manager-models/src/tools/filesystem.rs
```

**Create `manager-tools/src/types/user_interaction.rs`:**
```rust
// Copy all user interaction types from manager-models/src/tools/user_interaction.rs
```

**Create `manager-tools/src/types/mod.rs`:**
```rust
pub mod core;
pub mod bash;
pub mod grep;
pub mod filesystem;
pub mod user_interaction;

// Re-export commonly used types
pub use core::{ToolRequest, ToolResponse, ToolErrorResponse};
pub use bash::{BashRequest, BashResponse};
pub use grep::{GrepRequest, GrepResponse, GrepMatch};
pub use filesystem::{
    ListFilesRequest, ListFilesResponse,
    ReadFileRequest, ReadFileResponse,
    WriteFileRequest, WriteFileResponse,
    ApplyPatchRequest, ApplyPatchResponse,
    ApplyPatchFileChange, FileType, FileInfo,
};
pub use user_interaction::{
    AskUserRequest, AskUserResponse,
    UserQuestion, UserQuestionResponse,
    QuestionType, QuestionValidation,
};
```

#### 2.2 Update manager-tools/src/lib.rs

Add types module to public API:
```rust
pub mod types;
pub mod bash;
pub mod filesystem;
pub mod grep;
pub mod user_interaction;
pub mod tool_error;
pub mod tool_executor;

// Re-export for convenience
pub use types::*;
pub use bash::{BashExecutionResult, BashExecutorTrait};
pub use tool_error::ToolError;
pub use tool_executor::ToolExecutor;
```

### Phase 3: Update manager-tools Internal Imports

#### 3.1 Update tool_executor.rs

**Before:**
```rust
use manager_models::{ToolRequest, ToolResponse};
```

**After:**
```rust
use crate::types::{ToolRequest, ToolResponse};
```

**File**: `manager-tools/src/tool_executor.rs` (line 2)

### Phase 4: Update desktop-app

#### 4.1 Add manager-tools Dependency

**File**: `desktop-app/Cargo.toml`

Add:
```toml
[dependencies]
manager-tools = { path = "../manager-tools" }
# ... existing dependencies
```

#### 4.2 Update Imports in desktop-app

**File**: `desktop-app/src/pages/board.rs` (line 6)

**Before:**
```rust
use manager_models::{BashRequest, ReadFileRequest, ToolRequest, ToolResponse};
```

**After:**
```rust
use manager_tools::types::{BashRequest, ReadFileRequest, ToolRequest, ToolResponse};
```

**File**: `desktop-app/src/api_client.rs` (lines 1-7)

Update any tool-related imports to use `manager_tools::types` instead of `manager_models`.

### Phase 5: Update manager Backend

#### 5.1 Update Re-exports

**File**: `manager/src/models.rs`

**Before:**
```rust
pub use manager_models::{
    ToolRequest, ToolResponse, BashRequest, BashResponse,
    // ... other tool types
};
```

**After:**
```rust
pub use manager_tools::types::{
    ToolRequest, ToolResponse, BashRequest, BashResponse,
    // ... other tool types
};
```

#### 5.2 Update llm_agent.rs Imports

**File**: `manager/src/llm_agent.rs`

Update imports to reference `manager_tools::types` for tool-related types while keeping domain model imports from `manager_models`.

### Phase 6: Remove Types from manager-models

#### 6.1 Delete Tool Types

**From `manager-models/src/lib.rs`:**
- Remove lines 347-544 (ToolRequest, ToolResponse, and related types)

**Delete files:**
- `manager-models/src/tools/filesystem.rs`
- `manager-models/src/tools/user_interaction.rs`
- `manager-models/src/tools/mod.rs` (if now empty)

#### 6.2 Update manager-models/src/lib.rs

Remove the `pub mod tools;` declaration and all tool-related re-exports.

### Phase 7: Update nocodo-agents

**File**: `nocodo-agents/src/main.rs` (and other files as needed)

nocodo-agents already imports from both `manager-models` and `manager-tools`, so update any tool type imports to use `manager-tools::types`.

## Testing & Validation

### Test Each Crate Separately

#### manager-models
```bash
cd manager-models
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build
```

#### manager-tools
```bash
cd manager-tools
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build
```

#### manager
```bash
cd manager
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build
```

#### desktop-app
```bash
cd desktop-app
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build
```

#### nocodo-agents
```bash
cd nocodo-agents
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build
```

### Integration Testing

1. **Test LLM Agent with Tools**
   - Start manager backend
   - Create a work session
   - Execute tool calls (bash, grep, file operations)
   - Verify all tools still work correctly

2. **Test desktop-app UI**
   - Launch desktop-app
   - Verify project board displays tool calls
   - Check that tool requests/responses display correctly

3. **Test nocodo-agents**
   - Run agent with tool execution
   - Verify tool definitions are correct
   - Check schema generation still works

### Verification Checklist

- [ ] No compilation errors in any crate
- [ ] All tests pass in all crates
- [ ] No clippy warnings
- [ ] Code properly formatted (cargo fmt)
- [ ] Tool execution works in manager
- [ ] desktop-app displays tool calls correctly
- [ ] nocodo-agents can execute tools
- [ ] Schema generation produces correct JSON schemas
- [ ] API serialization/deserialization works correctly

## Files Changed

### New Files
- `manager-tools/src/types/mod.rs`
- `manager-tools/src/types/core.rs`
- `manager-tools/src/types/bash.rs`
- `manager-tools/src/types/grep.rs`
- `manager-tools/src/types/filesystem.rs`
- `manager-tools/src/types/user_interaction.rs`

### Modified Files
- `manager-tools/Cargo.toml` - Add manager-models dependency
- `manager-tools/src/lib.rs` - Add types module, update exports
- `manager-tools/src/tool_executor.rs` - Update imports
- `desktop-app/Cargo.toml` - Add manager-tools dependency
- `desktop-app/src/pages/board.rs` - Update imports
- `desktop-app/src/api_client.rs` - Update imports (if needed)
- `manager/src/models.rs` - Update re-exports
- `manager/src/llm_agent.rs` - Update imports
- `manager-models/src/lib.rs` - Remove tool types

### Deleted Files
- `manager-models/src/tools/filesystem.rs`
- `manager-models/src/tools/user_interaction.rs`
- `manager-models/src/tools/mod.rs` (if applicable)

## Expected Benefits

### Architectural Clarity
- **Clear ownership**: Tool types live with tool execution logic
- **Better separation**: Domain models vs tool infrastructure
- **Logical grouping**: Request/Response types with their executors

### Maintainability
- **Single source of truth**: All tool-related code in one place
- **Easier updates**: Add new tools in manager-tools only
- **Clear boundaries**: desktop-app explicitly depends on tools

### No Breaking Changes
- **API compatibility**: Types remain identical (same serde output)
- **Clean dependencies**: No circular dependencies introduced
- **Incremental migration**: Can be done step-by-step

## Migration Checklist

### Preparation
- [ ] Review current tool type usage across all crates
- [ ] Verify no circular dependency risks
- [ ] Backup current state

### Implementation
- [ ] Phase 1: Create type modules in manager-tools
- [ ] Phase 2: Move types from manager-models to manager-tools
- [ ] Phase 3: Update manager-tools internal imports
- [ ] Phase 4: Update desktop-app dependency and imports
- [ ] Phase 5: Update manager backend imports
- [ ] Phase 6: Remove types from manager-models
- [ ] Phase 7: Update nocodo-agents imports

### Testing
- [ ] Test manager-models builds and passes tests
- [ ] Test manager-tools builds and passes tests
- [ ] Test manager builds and passes tests
- [ ] Test desktop-app builds and passes tests
- [ ] Test nocodo-agents builds and passes tests
- [ ] Run integration tests
- [ ] Manual testing of tool execution
- [ ] Verify desktop-app UI works correctly

### Validation
- [ ] All crates compile without errors
- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Code properly formatted
- [ ] API responses unchanged
- [ ] Tool execution works end-to-end
- [ ] Documentation updated (if needed)

## Rollout Plan

1. **Preparation** (30 mins): Review and backup
2. **Phase 1-2** (1-2 hours): Create structure and move types
3. **Phase 3-5** (1 hour): Update imports across crates
4. **Phase 6** (30 mins): Clean up manager-models
5. **Phase 7** (30 mins): Update nocodo-agents
6. **Testing** (1-2 hours): Comprehensive testing
7. **Validation** (30 mins): Final checks

**Total estimated time**: 5-7 hours

## Success Criteria

- [ ] manager-tools contains all tool-related types
- [ ] manager-models contains only domain models
- [ ] desktop-app imports from manager-tools
- [ ] All crates build successfully
- [ ] All tests pass
- [ ] No circular dependencies
- [ ] Tool execution works correctly
- [ ] API compatibility maintained
- [ ] Code quality checks pass (fmt, clippy)

## Notes

- Tool types are completely independent (no dependencies on Project, Work, etc.)
- This is a pure refactoring - no functional changes
- Serde serialization format remains identical
- desktop-app will have slightly longer compile times (now compiles manager-tools)

## Related Tasks

- `extract-tool-descriptions-from-schema.md` - Tool description management
- `make-llm-sdk-trait-based.md` - LLM client architecture
