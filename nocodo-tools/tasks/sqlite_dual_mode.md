# Task: Implement Dual-Mode Pattern for SQLite Tool

## Overview
Add query and reflect modes to the SQLite tool, following the existing HackerNews dual-mode pattern. This allows the tool to both execute SQL queries and introspect database schema.

## Reference Implementation
See `manager-tools/src/types/hackernews.rs` and `manager-tools/src/hackernews/mod.rs` for the dual-mode pattern example.

## Files to Modify

### 1. `manager-tools/src/types/sqlite.rs`

#### Current Structure
```rust
pub struct Sqlite3ReaderRequest {
    pub db_path: String,
    pub query: String,
    pub limit: Option<usize>,
}
```

#### New Structure
```rust
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "mode")]
pub enum SqliteMode {
    /// Execute arbitrary SQL queries (SELECT or PRAGMA)
    #[serde(rename = "query")]
    Query {
        query: String,
    },

    /// Reflect/introspect database schema
    #[serde(rename = "reflect")]
    Reflect {
        /// Target of reflection: "tables", "schema", "table_info", "indexes", "views"
        target: String,
        /// Optional: specific table name for table_info mode
        table_name: Option<String>,
    },
}

pub struct Sqlite3ReaderRequest {
    pub db_path: String,
    pub mode: SqliteMode,
    pub limit: Option<usize>,
}
```

### 2. `manager-tools/src/sqlite/mod.rs`

#### Update Main Execution Function

**Current:**
```rust
pub async fn execute_sqlite3_reader(
    request: crate::types::Sqlite3ReaderRequest,
) -> Result<crate::types::ToolResponse, ToolError> {
    validate_db_path(&request.db_path)?;
    let executor = SqlExecutor::new(&request.db_path, MAX_LIMIT, TIMEOUT_MS)?;
    let limit = request.limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT);
    let result = executor.execute(&request.query, Some(limit))?;
    // ... format response
}
```

**New:**
```rust
pub async fn execute_sqlite3_reader(
    request: crate::types::Sqlite3ReaderRequest,
) -> Result<crate::types::ToolResponse, ToolError> {
    validate_db_path(&request.db_path)?;
    let executor = SqlExecutor::new(&request.db_path, MAX_LIMIT, TIMEOUT_MS)?;

    match request.mode {
        SqliteMode::Query { query } => {
            execute_query_mode(&executor, &query, request.limit).await
        }
        SqliteMode::Reflect { target, table_name } => {
            execute_reflect_mode(&executor, &target, table_name.as_deref(), request.limit).await
        }
    }
}
```

#### Add Mode-Specific Handlers

```rust
async fn execute_query_mode(
    executor: &SqlExecutor,
    query: &str,
    limit: Option<usize>,
) -> Result<ToolResponse, ToolError> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT);
    let result = executor.execute(query, Some(limit))?;

    Ok(ToolResponse {
        output: result.output,
        error: None,
    })
}

async fn execute_reflect_mode(
    executor: &SqlExecutor,
    target: &str,
    table_name: Option<&str>,
    limit: Option<usize>,
) -> Result<ToolResponse, ToolError> {
    let query = build_reflection_query(target, table_name)?;
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT);
    let result = executor.execute(&query, Some(limit))?;

    Ok(ToolResponse {
        output: format!("Schema Reflection ({}): {}", target, result.output),
        error: None,
    })
}
```

#### Add Helper Functions for Schema Introspection

```rust
/// Build SQL query for schema reflection based on target type
fn build_reflection_query(target: &str, table_name: Option<&str>) -> Result<String, ToolError> {
    let query = match target.to_lowercase().as_str() {
        "tables" => {
            // List all user tables (exclude sqlite internal tables)
            "SELECT name, sql FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name".to_string()
        }
        "schema" => {
            // Full schema dump
            "SELECT type, name, sql FROM sqlite_master WHERE sql IS NOT NULL ORDER BY type, name".to_string()
        }
        "table_info" => {
            // Get column information for a specific table
            match table_name {
                Some(name) => format!("PRAGMA table_info({})", name),
                None => return Err(ToolError::InvalidInput(
                    "table_name is required for table_info reflection".to_string()
                )),
            }
        }
        "indexes" => {
            // List all indexes
            "SELECT name, tbl_name, sql FROM sqlite_master WHERE type='index' AND name NOT LIKE 'sqlite_%' ORDER BY tbl_name, name".to_string()
        }
        "views" => {
            // List all views
            "SELECT name, sql FROM sqlite_master WHERE type='view' ORDER BY name".to_string()
        }
        _ => {
            return Err(ToolError::InvalidInput(
                format!("Unknown reflection target: {}. Valid targets: tables, schema, table_info, indexes, views", target)
            ))
        }
    };

    Ok(query)
}
```

## Implementation Steps

1. Update `Sqlite3ReaderRequest` struct to use `SqliteMode` enum
2. Add mode dispatch logic to `execute_sqlite3_reader()`
3. Extract existing query execution into `execute_query_mode()`
4. Implement `execute_reflect_mode()` with reflection logic
5. Add `build_reflection_query()` helper function
6. Test both modes independently

## Reflection Targets

### Supported Targets

1. **tables** - List all user tables with CREATE statements
   - Query: `SELECT name, sql FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'`
   - Use case: Discover available tables

2. **schema** - Full schema dump (tables, indexes, views)
   - Query: `SELECT type, name, sql FROM sqlite_master WHERE sql IS NOT NULL`
   - Use case: Complete database structure overview

3. **table_info** - Column information for specific table (requires table_name)
   - Query: `PRAGMA table_info(table_name)`
   - Use case: Get column names, types, nullability, defaults

4. **indexes** - List all indexes
   - Query: `SELECT name, tbl_name, sql FROM sqlite_master WHERE type='index'`
   - Use case: Understand query optimization

5. **views** - List all views
   - Query: `SELECT name, sql FROM sqlite_master WHERE type='view'`
   - Use case: Discover available views

## Testing

### Test Cases

1. **Query Mode - Basic SELECT**
```json
{
  "db_path": "/path/to/test.db",
  "mode": {
    "mode": "query",
    "query": "SELECT * FROM users LIMIT 5"
  }
}
```

2. **Reflect Mode - List Tables**
```json
{
  "db_path": "/path/to/test.db",
  "mode": {
    "mode": "reflect",
    "target": "tables"
  }
}
```

3. **Reflect Mode - Table Info**
```json
{
  "db_path": "/path/to/test.db",
  "mode": {
    "mode": "reflect",
    "target": "table_info",
    "table_name": "users"
  }
}
```

4. **Error Case - Invalid Target**
```json
{
  "db_path": "/path/to/test.db",
  "mode": {
    "mode": "reflect",
    "target": "invalid_target"
  }
}
```
Expected: ToolError with helpful message

## LLM Tool Definition Impact

The LLM tool definition in `nocodo-agents/src/tools/llm_schemas.rs` will need to be updated to expose both modes. This is handled in Task 2 (agents folder).

## Notes

- Maintain backward compatibility if possible (though breaking changes are acceptable)
- Follow existing error handling patterns from HackerNews tool
- Ensure all reflection queries are read-only (SELECT/PRAGMA only)
- The `SqlExecutor` already handles query validation for safety
- Both modes should respect the `limit` parameter

## Success Criteria

- [ ] SqliteMode enum implemented with query and reflect variants
- [ ] Mode dispatch logic works correctly
- [ ] All 5 reflection targets implemented and tested
- [ ] Error handling for invalid targets
- [ ] table_info requires table_name validation
- [ ] Both modes return ToolResponse in consistent format
- [ ] No breaking changes to existing query functionality
