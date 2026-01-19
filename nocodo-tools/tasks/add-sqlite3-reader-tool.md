# Add SQLite3 Reader Tool to manager-tools

**Status**: 📋 Not Started
**Priority**: Medium
**Created**: 2024-12-24

## Summary

Add a read-only SQLite3 database query tool (`sqlite3_reader`) to manager-tools by porting and adapting the proven implementation from the Indistocks project. This tool will enable AI agents to safely query SQLite databases for analysis purposes.

## Problem Statement

Multiple projects need SQLite database access for AI agents:
- The manager project needs database query capabilities for agent workflows
- The Indistocks project (`~/Projects/Indistocks`) has its own SQLite agent implementation
- Each project currently implements its own database access layer

This leads to:
- **Code duplication**: Same logic implemented multiple times
- **Maintenance burden**: Bug fixes and improvements need to be applied in multiple places
- **Inconsistent security**: Different validation approaches across projects
- **No reusability**: Cannot share improvements between projects

## Goals

1. **Create reusable sqlite3_reader tool**: Single implementation in manager-tools
2. **Read-only safety**: Strictly enforce SELECT and PRAGMA queries only
3. **Path-based access**: Agents provide database file paths (no connection pooling)
4. **Schema introspection**: Support PRAGMA statements for schema discovery
5. **Security first**: Comprehensive SQL injection protection and validation
6. **Proven implementation**: Port battle-tested code from Indistocks

## Architecture Overview

### Design Decisions

| Aspect | Decision | Rationale |
|--------|----------|-----------|
| **Database paths** | Agent-provided paths | Maximum flexibility, no pre-configuration needed |
| **Write access** | Read-only (SELECT + PRAGMA) | Tool is for analysis only, not data modification |
| **Statement batching** | Single statement per call | Prevents SQL injection, keeps calls atomic |
| **Connection lifecycle** | One connection per request | Simpler implementation, stateless |
| **Schema discovery** | SQL/PRAGMA queries | Agents can explore dynamically, no manual config |
| **Library choice** | `rusqlite` + `sqlparser` | Pure Rust, proven in Indistocks, excellent safety |

### Tool Interface

```rust
// Request
pub struct Sqlite3ReaderRequest {
    /// Absolute path to the SQLite database file
    pub db_path: String,
    /// SQL query to execute (SELECT or PRAGMA only)
    pub query: String,
    /// Optional limit on rows returned (default: 100, max: 1000)
    pub limit: Option<usize>,
}

// Response
pub struct Sqlite3ReaderResponse {
    /// Column names
    pub columns: Vec<String>,
    /// Result rows (as JSON values)
    pub rows: Vec<Vec<serde_json::Value>>,
    /// Number of rows returned
    pub row_count: usize,
    /// Whether results were truncated due to limit
    pub truncated: bool,
    /// Query execution time in milliseconds
    pub execution_time_ms: u64,
    /// Formatted output (table format for LLMs)
    pub formatted_output: String,
}
```

## Implementation Plan

### Phase 1: Port Core Components from Indistocks

#### 1.1 Create sqlite Module Structure

Create new module in manager-tools:
```
manager-tools/
  src/
    sqlite/
      mod.rs              # Public interface and executor wrapper
      executor.rs         # SqlExecutor (ported from Indistocks)
      validator.rs        # Query validation logic
      formatter.rs        # Result formatting for LLMs
    types/
      sqlite.rs           # Sqlite3ReaderRequest/Response types
```

#### 1.2 Port SqlExecutor from Indistocks

**Source**: `~/Projects/Indistocks/indistocks-ai/src/executor.rs`

Copy and adapt:
- `QueryResult` struct (lines 10-17)
- `SqlExecutor` struct (lines 19-269)
- Core methods:
  - `new()` - Database connection creation
  - `execute()` - Query execution with validation
  - `validate_query()` - SQL injection protection
  - `validate_query_body()` - AST-level validation
  - `apply_limit()` - Automatic LIMIT clause injection

**Key modifications:**
1. **Add PRAGMA support** - Extend validator to allow `Statement::Pragma`
2. **Update error types** - Use manager-tools error types instead of `IndistocksAiError`
3. **Remove Indistocks-specific context** - Make code generic

#### 1.3 Port Query Validation Logic

**Source**: `~/Projects/Indistocks/indistocks-ai/src/executor.rs` (lines 112-281)

Key features to preserve:
- ✅ Block multiple statements (SQL injection prevention)
- ✅ Parse SQL using `sqlparser` crate
- ✅ Only allow SELECT queries
- ✅ **NEW**: Allow PRAGMA statements for schema introspection
- ✅ Block dangerous keywords (INSERT, UPDATE, DELETE, DROP, etc.)
- ✅ Validate subqueries recursively
- ✅ Safe handling of UNION (allowed in SELECT context)

**Updated validation for PRAGMA support:**
```rust
match &statements[0] {
    Statement::Query(query) => {
        self.validate_query_body(&query.body)?;
    }
    Statement::Pragma { .. } => {
        // PRAGMA statements are read-only metadata queries - safe to allow
        // Examples: PRAGMA table_info(table_name), PRAGMA table_list
    }
    _ => {
        return Err(ToolError::InvalidInput(
            "Only SELECT queries and PRAGMA statements are allowed".to_string(),
        ));
    }
}
```

#### 1.4 Port Result Formatting

**Source**: `~/Projects/Indistocks/indistocks-ai/src/tools.rs` (lines 74-171)

Copy formatting logic:
- `format_query_result()` - Table-formatted output for LLMs
- `format_cell_value()` - Cell value formatting with truncation
- Summary information (row count, execution time, truncation notice)
- Pretty-printed table with column alignment

### Phase 2: Integrate with manager-tools Type System

#### 2.1 Create Type Definitions

**File**: `manager-tools/src/types/sqlite.rs`

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Execute read-only SQL queries on SQLite databases. Only SELECT queries and
/// PRAGMA statements are allowed. This tool does NOT support INSERT, UPDATE,
/// DELETE, or other write operations.
///
/// To explore database schema:
/// - List tables: SELECT name FROM sqlite_master WHERE type='table'
/// - Get columns: PRAGMA table_info(table_name)
/// - See full schema: SELECT sql FROM sqlite_master WHERE name='table_name'
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Sqlite3ReaderRequest {
    /// Absolute path to the SQLite database file
    #[serde(default)]
    #[schemars(description = "Absolute path to the SQLite database file")]
    pub db_path: String,

    /// SQL query to execute (SELECT or PRAGMA only)
    #[serde(default)]
    #[schemars(description = "SQL query to execute. Only SELECT queries and PRAGMA statements are allowed.")]
    pub query: String,

    /// Maximum number of rows to return (default: 100, max: 1000)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Maximum number of rows to return. Defaults to 100, maximum 1000.")]
    pub limit: Option<usize>,
}

/// Response from sqlite3_reader tool
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Sqlite3ReaderResponse {
    /// Column names from the query result
    pub columns: Vec<String>,

    /// Result rows (each row is an array of values)
    pub rows: Vec<Vec<serde_json::Value>>,

    /// Number of rows returned
    pub row_count: usize,

    /// Whether results were truncated due to limit
    pub truncated: bool,

    /// Query execution time in milliseconds
    pub execution_time_ms: u64,

    /// Human-readable formatted output (table format)
    pub formatted_output: String,
}
```

#### 2.2 Update ToolRequest and ToolResponse Enums

**File**: `manager-tools/src/types/core.rs`

Add new variant to `ToolRequest`:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type")]
pub enum ToolRequest {
    // ... existing variants
    #[serde(rename = "sqlite3_reader")]
    Sqlite3Reader(super::sqlite::Sqlite3ReaderRequest),
}
```

Add new variant to `ToolResponse`:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolResponse {
    // ... existing variants
    #[serde(rename = "sqlite3_reader")]
    Sqlite3Reader(super::sqlite::Sqlite3ReaderResponse),
}
```

#### 2.3 Update Type Module Exports

**File**: `manager-tools/src/types/mod.rs`

Add:
```rust
pub mod sqlite;
// ... existing modules

pub use sqlite::{Sqlite3ReaderRequest, Sqlite3ReaderResponse};
```

### Phase 3: Implement Tool Executor Integration

#### 3.1 Create SQLite Tool Executor

**File**: `manager-tools/src/sqlite/mod.rs`

```rust
use crate::types::{Sqlite3ReaderRequest, Sqlite3ReaderResponse, ToolResponse};
use crate::tool_error::ToolError;
use anyhow::Result;
use std::path::Path;

mod executor;
mod validator;
mod formatter;

use executor::SqlExecutor;

/// Execute a sqlite3_reader tool request
pub async fn execute_sqlite3_reader(
    base_path: &Path,
    request: Sqlite3ReaderRequest,
) -> Result<ToolResponse> {
    // Validate database path (security check)
    validate_db_path(&request.db_path, base_path)?;

    // Default limits
    const DEFAULT_LIMIT: usize = 100;
    const MAX_LIMIT: usize = 1000;
    const TIMEOUT_MS: u64 = 5000;

    let limit = request.limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT);

    // Create executor and execute query
    let executor = SqlExecutor::new(&request.db_path, MAX_LIMIT, TIMEOUT_MS)?;
    let result = executor.execute(&request.query, Some(limit))?;

    // Format output for LLM
    let formatted_output = formatter::format_query_result(&result);

    let response = Sqlite3ReaderResponse {
        columns: result.columns,
        rows: result.rows,
        row_count: result.row_count,
        truncated: result.truncated,
        execution_time_ms: result.execution_time_ms,
        formatted_output,
    };

    Ok(ToolResponse::Sqlite3Reader(response))
}

/// Validate that the database path is safe to access
fn validate_db_path(db_path: &str, _base_path: &Path) -> Result<(), ToolError> {
    // Basic validation
    if db_path.is_empty() {
        return Err(ToolError::InvalidInput("Database path cannot be empty".to_string()));
    }

    // Check file exists
    let path = Path::new(db_path);
    if !path.exists() {
        return Err(ToolError::InvalidInput(format!("Database file not found: {}", db_path)));
    }

    if !path.is_file() {
        return Err(ToolError::InvalidInput(format!("Path is not a file: {}", db_path)));
    }

    // TODO: Consider adding path allowlist/denylist validation if needed

    Ok(())
}
```

#### 3.2 Update ToolExecutor

**File**: `manager-tools/src/tool_executor.rs`

Add sqlite import:
```rust
use crate::sqlite;
```

Add match arm in `execute()` method:
```rust
pub async fn execute(&self, request: ToolRequest) -> Result<ToolResponse> {
    match request {
        // ... existing match arms
        ToolRequest::Sqlite3Reader(req) => {
            sqlite::execute_sqlite3_reader(&self.base_path, req).await
        }
    }
}
```

Add match arm in `execute_from_json()` method:
```rust
let response_value = match tool_response {
    // ... existing match arms
    ToolResponse::Sqlite3Reader(response) => serde_json::to_value(response)?,
};
```

### Phase 4: Add Dependencies

#### 4.1 Update Cargo.toml

**File**: `manager-tools/Cargo.toml`

Add dependencies:
```toml
[dependencies]
# ... existing dependencies
rusqlite = "0.32"
sqlparser = "0.51"
```

### Phase 5: Testing

#### 5.1 Unit Tests

**File**: `manager-tools/src/sqlite/executor.rs`

Port and adapt tests from Indistocks:
- `test_sql_validation()` - Test query validation logic
- `test_sql_execution()` - Test query execution
- `test_pragma_support()` - **NEW**: Test PRAGMA statement support
- `test_multiple_statement_blocking()` - Test SQL injection prevention
- `test_dangerous_keyword_blocking()` - Test dangerous operation blocking

**File**: `manager-tools/src/sqlite/formatter.rs`

Port and adapt tests:
- `test_format_query_result()` - Test table formatting
- `test_format_cell_value()` - Test cell value formatting
- `test_format_empty_query_result()` - Test empty results

#### 5.2 Integration Tests

**File**: `manager-tools/src/sqlite/mod.rs` or separate test file

Create integration tests:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use rusqlite::Connection;

    #[tokio::test]
    async fn test_sqlite3_reader_tool() -> Result<()> {
        // Create test database
        let temp_file = NamedTempFile::new()?;
        let conn = Connection::open(temp_file.path())?;

        conn.execute_batch(r#"
            CREATE TABLE test_table (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                value REAL
            );
            INSERT INTO test_table (name, value) VALUES ('test1', 100.5);
            INSERT INTO test_table (name, value) VALUES ('test2', 200.3);
        "#)?;
        drop(conn);

        // Test SELECT query
        let request = Sqlite3ReaderRequest {
            db_path: temp_file.path().to_str().unwrap().to_string(),
            query: "SELECT name, value FROM test_table ORDER BY value DESC".to_string(),
            limit: None,
        };

        let base_path = Path::new("/tmp");
        let response = execute_sqlite3_reader(base_path, request).await?;

        match response {
            ToolResponse::Sqlite3Reader(result) => {
                assert_eq!(result.columns, vec!["name", "value"]);
                assert_eq!(result.row_count, 2);
                assert!(!result.truncated);
                assert!(result.formatted_output.contains("test2"));
            }
            _ => panic!("Expected Sqlite3Reader response"),
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_pragma_support() -> Result<()> {
        // Create test database
        let temp_file = NamedTempFile::new()?;
        let conn = Connection::open(temp_file.path())?;

        conn.execute_batch(r#"
            CREATE TABLE users (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL
            );
        "#)?;
        drop(conn);

        // Test PRAGMA query
        let request = Sqlite3ReaderRequest {
            db_path: temp_file.path().to_str().unwrap().to_string(),
            query: "PRAGMA table_info(users)".to_string(),
            limit: None,
        };

        let base_path = Path::new("/tmp");
        let response = execute_sqlite3_reader(base_path, request).await?;

        match response {
            ToolResponse::Sqlite3Reader(result) => {
                assert!(result.row_count > 0);
                assert!(result.formatted_output.contains("id")
                     || result.formatted_output.contains("name"));
            }
            _ => panic!("Expected Sqlite3Reader response"),
        }

        Ok(())
    }
}
```

### Phase 6: Documentation

#### 6.1 Update manager-tools README

**File**: `manager-tools/README.md`

Add section documenting the new tool:
```markdown
### SQLite3 Reader Tool

Read-only SQLite database query tool for AI agents.

**Features:**
- Execute SELECT queries and PRAGMA statements
- Automatic query validation and SQL injection prevention
- Row limiting and query timeouts
- Schema introspection support
- Table-formatted output optimized for LLMs

**Usage Example:**
```rust
use manager_tools::{ToolExecutor, ToolRequest, Sqlite3ReaderRequest};

let executor = ToolExecutor::new(base_path);

let request = ToolRequest::Sqlite3Reader(Sqlite3ReaderRequest {
    db_path: "/path/to/database.db".to_string(),
    query: "SELECT * FROM users LIMIT 10".to_string(),
    limit: Some(10),
});

let response = executor.execute(request).await?;
```

**Security:**
- Only SELECT and PRAGMA statements allowed
- Blocks INSERT, UPDATE, DELETE, DROP, etc.
- SQL injection protection via AST parsing
- Path validation
```

#### 6.2 Add Code Examples

Create example usage documentation showing:
- Basic SELECT queries
- Schema introspection with PRAGMA
- Error handling
- Integration with LLM agents

## Files Changed

### New Files
- `manager-tools/src/sqlite/mod.rs` - Main module and executor wrapper
- `manager-tools/src/sqlite/executor.rs` - SqlExecutor implementation
- `manager-tools/src/sqlite/validator.rs` - Query validation logic
- `manager-tools/src/sqlite/formatter.rs` - Result formatting
- `manager-tools/src/types/sqlite.rs` - Request/Response types
- `manager-tools/tasks/add-sqlite3-reader-tool.md` - This task document

### Modified Files
- `manager-tools/Cargo.toml` - Add rusqlite and sqlparser dependencies
- `manager-tools/src/lib.rs` - Add sqlite module
- `manager-tools/src/types/mod.rs` - Add sqlite types export
- `manager-tools/src/types/core.rs` - Add Sqlite3Reader variants
- `manager-tools/src/tool_executor.rs` - Add sqlite3_reader execution
- `manager-tools/README.md` - Document new tool

## Testing & Validation

### Unit Tests
```bash
cd manager-tools
cargo test sqlite
```

### Integration Tests
```bash
cd manager-tools
cargo test --test '*'
```

### Full Build & Quality Checks
```bash
cd manager-tools
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo build
```

### Manual Testing Checklist
- [ ] SELECT query returns correct results
- [ ] PRAGMA statements work for schema introspection
- [ ] INSERT/UPDATE/DELETE are blocked
- [ ] Multiple statements are blocked
- [ ] SQL injection attempts are blocked
- [ ] Row limits are enforced
- [ ] Query timeouts work
- [ ] Formatted output is readable
- [ ] Non-existent database paths error gracefully
- [ ] Invalid queries return helpful error messages

## Success Criteria

- [ ] sqlite3_reader tool integrated into manager-tools
- [ ] All unit tests pass
- [ ] All integration tests pass
- [ ] No clippy warnings
- [ ] Code properly formatted
- [ ] Documentation complete
- [ ] SELECT queries execute successfully
- [ ] PRAGMA statements supported for schema introspection
- [ ] Write operations strictly blocked
- [ ] SQL injection protection working
- [ ] Ready for use in manager and Indistocks projects

## Migration Path for Indistocks

Once this tool is complete, Indistocks can migrate to use it:

1. Update `indistocks-ai/Cargo.toml` to depend on `manager-tools`
2. Replace local `SqlExecutor` with `manager_tools::sqlite`
3. Update tool creation to use `Sqlite3ReaderRequest`
4. Remove duplicate code from `indistocks-ai/src/executor.rs`
5. Test that existing functionality still works

This ensures both projects benefit from a single, well-tested implementation.

## References

- **Source implementation**: `~/Projects/Indistocks/indistocks-ai/src/executor.rs`
- **Source tool integration**: `~/Projects/Indistocks/indistocks-ai/src/tools.rs`
- **Rusqlite docs**: https://docs.rs/rusqlite/
- **Sqlparser docs**: https://docs.rs/sqlparser/

## Notes

- This is a pure addition - no breaking changes to existing tools
- The tool is designed for analysis, not data modification
- PRAGMA support enables dynamic schema discovery without pre-configuration
- Path validation can be enhanced later with allowlist/denylist if needed
- Connection pooling can be added in future if performance requires it
