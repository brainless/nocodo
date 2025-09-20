# LIST_FILES_TOOL_UPDATE.md

## Overview

This document outlines the plan to update nocodo's file listing functionality to match opencode's approach, which uses a plain text tree structure representation instead of JSON.

## Current State Analysis

### Nocodo Current Implementation
- **File Type**: `FileInfo` struct with fields: `name`, `path`, `is_directory`, `size`, `modified_at`, `created_at`
- **Response Type**: `FileListResponse` struct with fields: `files: Vec<FileInfo>`, `current_path: String`
- **Output Format**: JSON array of FileInfo objects
- **Location**: `manager/src/models.rs:288-312`

### OpenCode Target Implementation
- **File Type**: `FileNode` struct with fields: `name`, `path`, `absolute`, `type`, `ignored`
- **Response Type**: Array of FileNode objects, but formatted as plain text tree structure
- **Output Format**: Plain text indented tree structure using 2 spaces per level
- **Key Features**:
  - Tree-like directory structure visualization
  - 100 file limit per call
  - Built-in ignore patterns for common directories
  - Metadata includes absolute paths and ignore status
  - Plain text format optimized for AI agent consumption

## Implementation Plan

### 1. Update Rust Types (`manager/src/models.rs`)

#### New FileInfo Structure
```rust
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct FileInfo {
    pub name: String,
    pub path: String,        // relative path
    pub absolute: String,    // absolute path
    pub file_type: FileType, // enum: File, Directory
    pub ignored: bool,       // whether file is ignored by .gitignore
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub enum FileType {
    File,
    Directory,
}
```

#### New FileListResponse Structure
```rust
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct FileListResponse {
    pub files: String,           // Plain text tree representation
    pub current_path: String,    // Current directory being listed
    pub total_files: u32,        // Total number of files found
    pub truncated: bool,         // Whether results were limited to 100
    pub limit: u32,              // Maximum files returned (100)
}
```

### 2. Update File Listing Logic (`manager/src/tools.rs`)

#### Key Changes:
- Implement tree structure generation algorithm
- Add 100-file limit with breadth-first traversal
- Include .gitignore parsing for `ignored` field
- Format output as plain text tree using 2-space indentation
- Add metadata fields (total_files, truncated, limit)

#### Tree Formatting Algorithm:
```rust
fn format_as_tree(files: Vec<FileInfo>, base_path: &str) -> String {
    // Build directory tree structure
    // Sort: directories first, then files, alphabetically
    // Format with 2-space indentation per level
    // Example output:
    // project_root/
    //   src/
    //     main.rs
    //     lib.rs
    //   tests/
    //     integration_test.rs
    //   README.md
}
```

### 3. Update Tests

#### Update `manager/src/llm_agent.rs` tests:
- Modify test expectations to check plain text tree format
- Update assertions to validate tree structure
- Test file limiting (100 file max)
- Test ignore functionality

#### Update `manager/tests/llm_e2e_real_test.rs` tests:
- Update end-to-end test expectations
- Verify tree format in realistic project structures
- Test integration with actual .gitignore files

### 4. Implementation Details

#### File Limit Strategy:
- Use breadth-first traversal to get representative file distribution
- Prioritize showing directory structure over deep nesting
- Include directories in count, but prefer showing files when possible

#### Ignore Logic:
- Parse .gitignore file in project root
- Use similar ignore patterns as opencode:
  - `node_modules/`, `.git/`, `dist/`, `build/`, `.next/`, etc.
- Mark files as ignored but still include in tree with indication

#### Tree Structure Format:
```
project_name/
  src/
    components/
      Button.tsx
      Modal.tsx
    utils/
      helpers.ts
    main.ts
  tests/
    unit/
      button.test.ts
  package.json
  README.md
```

## Migration Strategy

### Phase 1: Update Types
1. Update `FileInfo` struct to match opencode's `FileNode`
2. Update `FileListResponse` to include metadata and plain text format
3. Regenerate TypeScript bindings

### Phase 2: Update Implementation
1. Implement tree generation algorithm in `tools.rs`
2. Add .gitignore parsing logic
3. Implement 100-file limiting with breadth-first traversal

### Phase 3: Update Tests
1. Update unit tests in `llm_agent.rs`
2. Update e2e tests in `llm_e2e_real_test.rs`
3. Verify backward compatibility concerns

### Phase 4: Frontend Updates
1. Update web interface to handle new plain text format
2. Update API client to work with new response structure
3. Test UI components with new data format

## Benefits of This Approach

1. **AI-Optimized**: Plain text tree format is more readable for LLM agents
2. **Performance**: 100-file limit prevents overwhelming responses
3. **Context-Aware**: Includes ignore status and absolute paths
4. **Familiar Format**: Tree structure is universally understood
5. **Metadata**: Provides useful context about truncation and limits

## Backward Compatibility

- This is a breaking change to the API
- Frontend components will need updates
- TypeScript types will change
- Consider versioning the API endpoint if needed

## Testing Approach

1. **Unit Tests**: Verify tree generation algorithm
2. **Integration Tests**: Test with real project structures
3. **E2E Tests**: Verify full workflow with LLM agents
4. **Performance Tests**: Validate 100-file limit behavior
5. **Edge Cases**: Empty directories, deeply nested structures, large projects

## Success Criteria

- [ ] FileInfo matches opencode's FileNode structure
- [ ] FileListResponse contains plain text tree representation
- [ ] 100-file limit is enforced
- [ ] .gitignore parsing works correctly
- [ ] Tree format is consistent and readable
- [ ] All tests pass
- [ ] Frontend components work with new format
- [ ] Performance is acceptable for large projects