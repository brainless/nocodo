# Add PostgreSQL Reader Tool to manager-tools

**Status**: 📋 Not Started
**Priority**: Medium
**Created**: 2026-01-19

## Summary

Add a read-only PostgreSQL database query tool (`postgresql_reader`) to manager-tools, modeled after the proven `sqlite_analysis` implementation. This tool will enable AI agents to safely query PostgreSQL databases for analysis purposes with strict read-only guarantees.

## Problem Statement

AI agents need secure, read-only access to PostgreSQL databases for:
- Data analysis and exploration
- Schema introspection
- Report generation
- Data validation and quality checks

Without a dedicated tool:
- **Security risks**: Ad-hoc database access may allow write operations
- **No standardization**: Each project implements its own database access
- **Inconsistent validation**: Different security approaches across projects
- **Code duplication**: Same logic reimplemented multiple times

## Goals

1. **Create reusable postgresql_reader tool**: Single implementation in manager-tools
2. **Read-only safety**: Strictly enforce SELECT and schema query operations only
3. **Connection-based access**: Support standard PostgreSQL connection parameters
4. **Schema introspection**: Support information_schema queries for schema discovery
5. **Security first**: Comprehensive SQL injection protection and validation
6. **Proven architecture**: Follow the battle-tested sqlite_analysis design

## Architecture Overview

### Design Decisions

| Aspect | Decision | Rationale |
|--------|----------|-----------|
| **Connection parameters** | Standard PostgreSQL params (host, port, db, user, password) | Maximum flexibility, industry standard |
| **Write access** | Read-only (SELECT + information_schema) | Tool is for analysis only, not data modification |
| **Statement batching** | Single statement per call | Prevents SQL injection, keeps calls atomic |
| **Connection lifecycle** | One connection per request | Simpler implementation, stateless |
| **Schema discovery** | information_schema queries | Standard PostgreSQL approach, no custom metadata |
| **Library choice** | `tokio-postgres` + `sqlparser` | Async support, proven parser, excellent safety |
| **SSL/TLS** | Optional with configurable mode | Support secure and local connections |

### Tool Interface

```rust
// Request - supports two modes similar to sqlite_analysis
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PostgresqlReaderRequest {
    /// Connection parameters
    pub connection: PostgresqlConnection,
    /// Execution mode: either query or reflect
    pub mode: PostgresqlMode,
    /// Optional limit on rows returned (default: 100, max: 1000)
    pub limit: Option<usize>,
}

// Connection parameters
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PostgresqlConnection {
    pub host: String,
    pub port: Option<u16>,  // defaults to 5432
    pub database: String,
    pub user: String,
    pub password: String,
    pub ssl_mode: Option<String>,  // disable, prefer, require
}

// Mode enum similar to SqliteMode
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "mode")]
pub enum PostgresqlMode {
    #[serde(rename = "query")]
    Query {
        #[schemars(description = "SQL query to execute. Only SELECT queries are allowed.")]
        query: String,
    },

    #[serde(rename = "reflect")]
    Reflect {
        #[schemars(description = "Target of reflection: tables, schema, table_info, indexes, views, foreign_keys, stats")]
        target: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(description = "Optional: specific table name for table_info and foreign_keys modes")]
        table_name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(description = "Optional: specific schema name (defaults to 'public')")]
        schema_name: Option<String>,
    },
}

// Response
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PostgresqlReaderResponse {
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

### Phase 1: Create Core Components

#### 1.1 Create postgresql_reader Module Structure

Create new module in manager-tools:
```
manager-tools/
  src/
    postgresql_reader/
      mod.rs              # Public interface and executor wrapper
      executor.rs         # PostgresqlExecutor (similar to sqlite SqlExecutor)
      formatter.rs        # Result formatting for LLMs (reuse sqlite formatter)
    types/
      postgresql_reader.rs  # PostgresqlReaderRequest/Response types
```

#### 1.2 Implement PostgresqlExecutor

**File**: `manager-tools/src/postgresql_reader/executor.rs`

Core structure similar to `sqlite_analysis/executor.rs`:

```rust
use tokio_postgres::{Client, NoTls};
use serde_json::Value;
use sqlparser::ast::{SetExpr, Statement};
use sqlparser::dialect::PostgreSqlDialect;
use sqlparser::parser::Parser;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
    pub row_count: usize,
    pub truncated: bool,
    pub execution_time_ms: u64,
}

pub struct PostgresqlExecutor {
    client: Client,
    max_rows: usize,
    timeout_ms: u64,
}

impl PostgresqlExecutor {
    /// Create new executor with connection parameters
    pub async fn new(
        connection: &PostgresqlConnection,
        max_rows: usize,
        timeout_ms: u64,
    ) -> anyhow::Result<Self> {
        // Build connection string
        let conn_str = format!(
            "host={} port={} dbname={} user={} password={}",
            connection.host,
            connection.port.unwrap_or(5432),
            connection.database,
            connection.user,
            connection.password
        );

        // Configure SSL mode
        let (client, connection) = match connection.ssl_mode.as_deref() {
            Some("require") => {
                // Use TLS for secure connections
                tokio_postgres::connect(&conn_str, tokio_postgres::tls::MakeTlsConnector::new()?)
                    .await?
            }
            _ => {
                // Use NoTls for local/dev connections
                tokio_postgres::connect(&conn_str, NoTls).await?
            }
        };

        // Spawn connection task
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        Ok(Self {
            client,
            max_rows,
            timeout_ms,
        })
    }

    /// Execute a validated query
    pub async fn execute(&self, query: &str, limit: Option<usize>) -> anyhow::Result<QueryResult> {
        let start_time = Instant::now();

        // Validate query for safety
        self.validate_query(query)?;

        let effective_limit = limit.unwrap_or(self.max_rows).min(self.max_rows);

        // Apply LIMIT clause if not present
        let final_query = self.apply_limit(query, effective_limit)?;

        // Execute query with timeout
        let rows = tokio::time::timeout(
            std::time::Duration::from_millis(self.timeout_ms),
            self.client.query(&final_query, &[])
        ).await??;

        // Extract columns
        let columns: Vec<String> = if let Some(first_row) = rows.first() {
            first_row
                .columns()
                .iter()
                .map(|col| col.name().to_string())
                .collect()
        } else {
            Vec::new()
        };

        // Convert rows to JSON values
        let mut result_rows = Vec::new();
        for row in rows.iter().take(effective_limit) {
            let mut row_values = Vec::new();
            for (idx, column) in row.columns().iter().enumerate() {
                let value = postgres_value_to_json(&row, idx, column.type_())?;
                row_values.push(value);
            }
            result_rows.push(row_values);
        }

        let row_count = result_rows.len();
        let truncated = row_count == effective_limit && rows.len() > effective_limit;
        let execution_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(QueryResult {
            columns,
            rows: result_rows,
            row_count,
            truncated,
            execution_time_ms,
        })
    }

    /// Validate query is read-only and safe
    pub fn validate_query(&self, query: &str) -> anyhow::Result<()> {
        // Check for multiple statements
        if query.trim().ends_with(';') && query.trim().matches(';').count() > 1 {
            return Err(anyhow::anyhow!("Multiple SQL statements are not allowed"));
        }

        // Parse SQL using PostgreSQL dialect
        let dialect = PostgreSqlDialect {};
        let statements = Parser::parse_sql(&dialect, query)
            .map_err(|e| anyhow::anyhow!("Failed to parse SQL: {}", e))?;

        if statements.is_empty() {
            return Err(anyhow::anyhow!("Empty SQL statement"));
        }

        if statements.len() > 1 {
            return Err(anyhow::anyhow!("Multiple SQL statements are not allowed"));
        }

        // Only allow SELECT queries
        match &statements[0] {
            Statement::Query(query) => {
                self.validate_query_body(&query.body)?;
            }
            _ => {
                return Err(anyhow::anyhow!("Only SELECT queries are allowed"));
            }
        }

        // Block dangerous keywords (similar to sqlite)
        let dangerous_keywords = [
            "INSERT", "UPDATE", "DELETE", "DROP", "CREATE", "ALTER",
            "TRUNCATE", "EXEC", "EXECUTE", "MERGE", "CALL", "GRANT",
            "REVOKE", "COPY",
        ];

        let query_upper = query.to_uppercase();
        for keyword in &dangerous_keywords {
            if query_upper.contains(keyword) && !is_safe_context(query, keyword) {
                return Err(anyhow::anyhow!(
                    "Use of '{}' is not allowed in queries",
                    keyword
                ));
            }
        }

        Ok(())
    }

    // Helper methods similar to sqlite (validate_query_body, validate_table_factor, etc.)
    // ... (implementation details similar to sqlite_analysis/executor.rs)
}

/// Convert PostgreSQL value to JSON
fn postgres_value_to_json(
    row: &tokio_postgres::Row,
    idx: usize,
    type_: &tokio_postgres::types::Type,
) -> anyhow::Result<Value> {
    // Handle different PostgreSQL types
    // Implementation will use tokio_postgres type system
    // ...
}

fn is_safe_context(query: &str, keyword: &str) -> bool {
    // Similar to sqlite implementation
    let query_upper = query.to_uppercase();

    // UNION is allowed in SELECT queries
    if keyword == "UNION" {
        return query_upper.contains("SELECT");
    }

    let keyword_pattern = regex::Regex::new(&format!(r"\b{}\b", regex::escape(keyword))).unwrap();
    !keyword_pattern.is_match(&query_upper)
}
```

#### 1.3 Implement Reflection Queries

**File**: `manager-tools/src/postgresql_reader/mod.rs`

```rust
fn build_reflection_query(
    target: &str,
    table_name: Option<&str>,
    schema_name: Option<&str>,
) -> Result<String, ToolError> {
    let schema = schema_name.unwrap_or("public");

    let query = match target.to_lowercase().as_str() {
        "tables" => {
            format!(
                "SELECT table_name, table_type
                 FROM information_schema.tables
                 WHERE table_schema = '{}'
                 ORDER BY table_name",
                schema
            )
        }
        "schema" => {
            format!(
                "SELECT table_name, column_name, data_type, is_nullable
                 FROM information_schema.columns
                 WHERE table_schema = '{}'
                 ORDER BY table_name, ordinal_position",
                schema
            )
        }
        "table_info" => {
            match table_name {
                Some(name) => format!(
                    "SELECT column_name, data_type, is_nullable, column_default
                     FROM information_schema.columns
                     WHERE table_schema = '{}' AND table_name = '{}'
                     ORDER BY ordinal_position",
                    schema, name
                ),
                None => return Err(ToolError::InvalidInput(
                    "table_name is required for table_info reflection".to_string()
                )),
            }
        }
        "indexes" => {
            format!(
                "SELECT indexname, tablename, indexdef
                 FROM pg_indexes
                 WHERE schemaname = '{}'
                 ORDER BY tablename, indexname",
                schema
            )
        }
        "views" => {
            format!(
                "SELECT table_name, view_definition
                 FROM information_schema.views
                 WHERE table_schema = '{}'
                 ORDER BY table_name",
                schema
            )
        }
        "foreign_keys" => {
            match table_name {
                Some(name) => format!(
                    "SELECT
                        tc.constraint_name,
                        kcu.column_name,
                        ccu.table_name AS foreign_table_name,
                        ccu.column_name AS foreign_column_name
                     FROM information_schema.table_constraints AS tc
                     JOIN information_schema.key_column_usage AS kcu
                       ON tc.constraint_name = kcu.constraint_name
                     JOIN information_schema.constraint_column_usage AS ccu
                       ON ccu.constraint_name = tc.constraint_name
                     WHERE tc.constraint_type = 'FOREIGN KEY'
                       AND tc.table_schema = '{}'
                       AND tc.table_name = '{}'",
                    schema, name
                ),
                None => return Err(ToolError::InvalidInput(
                    "table_name is required for foreign_keys reflection".to_string()
                )),
            }
        }
        "stats" => {
            format!(
                "SELECT
                    schemaname,
                    COUNT(*) as table_count,
                    pg_size_pretty(SUM(pg_total_relation_size(schemaname||'.'||tablename))::bigint) as total_size
                 FROM pg_tables
                 WHERE schemaname = '{}'
                 GROUP BY schemaname",
                schema
            )
        }
        _ => {
            return Err(ToolError::InvalidInput(
                format!("Unknown reflection target: {}. Valid targets: tables, schema, table_info, indexes, views, foreign_keys, stats", target)
            ))
        }
    };

    Ok(query)
}
```

#### 1.4 Reuse Formatter from SQLite

The formatter can be shared between sqlite and postgresql:

**File**: `manager-tools/src/postgresql_reader/formatter.rs`

```rust
// Re-export formatter from sqlite_analysis
pub use crate::sqlite_analysis::formatter::*;
```

### Phase 2: Integrate with manager-tools Type System

#### 2.1 Create Type Definitions

**File**: `manager-tools/src/types/postgresql_reader.rs`

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PostgresqlConnection {
    #[schemars(description = "PostgreSQL server hostname or IP address")]
    pub host: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "PostgreSQL server port (default: 5432)")]
    pub port: Option<u16>,

    #[schemars(description = "Database name")]
    pub database: String,

    #[schemars(description = "Database user")]
    pub user: String,

    #[schemars(description = "Database password")]
    pub password: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "SSL mode: disable, prefer, require (default: prefer)")]
    pub ssl_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "mode")]
pub enum PostgresqlMode {
    #[serde(rename = "query")]
    Query {
        #[schemars(description = "SQL query to execute. Only SELECT queries are allowed.")]
        query: String,
    },

    #[serde(rename = "reflect")]
    Reflect {
        #[schemars(description = "Target of reflection: tables, schema, table_info, indexes, views, foreign_keys, stats")]
        target: String,

        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(description = "Optional: specific table name for table_info and foreign_keys modes")]
        table_name: Option<String>,

        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(description = "Optional: schema name (defaults to 'public')")]
        schema_name: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PostgresqlReaderRequest {
    #[schemars(description = "PostgreSQL connection parameters")]
    pub connection: PostgresqlConnection,

    #[schemars(description = "Execution mode: either query or reflect")]
    pub mode: PostgresqlMode,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Maximum number of rows to return. Defaults to 100, maximum 1000.")]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PostgresqlReaderResponse {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub row_count: usize,
    pub truncated: bool,
    pub execution_time_ms: u64,
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
    #[serde(rename = "postgresql_reader")]
    PostgresqlReader(super::postgresql_reader::PostgresqlReaderRequest),
}
```

Add new variant to `ToolResponse`:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolResponse {
    // ... existing variants
    #[serde(rename = "postgresql_reader")]
    PostgresqlReader(super::postgresql_reader::PostgresqlReaderResponse),
}
```

#### 2.3 Update Type Module Exports

**File**: `manager-tools/src/types/mod.rs`

Add:
```rust
pub mod postgresql_reader;
// ... existing modules

pub use postgresql_reader::{
    PostgresqlConnection, PostgresqlMode, PostgresqlReaderRequest, PostgresqlReaderResponse
};
```

### Phase 3: Implement Tool Executor Integration

#### 3.1 Create Main PostgreSQL Tool Function

**File**: `manager-tools/src/postgresql_reader/mod.rs`

```rust
use crate::types::{PostgresqlReaderRequest, PostgresqlReaderResponse, ToolResponse};
use crate::tool_error::ToolError;
use anyhow::Result;

mod executor;
mod formatter;

pub use executor::{QueryResult, PostgresqlExecutor};

pub async fn execute_postgresql_reader(
    request: PostgresqlReaderRequest,
) -> Result<ToolResponse, ToolError> {
    const DEFAULT_LIMIT: usize = 100;
    const MAX_LIMIT: usize = 1000;
    const TIMEOUT_MS: u64 = 5000;

    let limit = request.limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT);

    let executor = PostgresqlExecutor::new(&request.connection, MAX_LIMIT, TIMEOUT_MS)
        .await
        .map_err(|e| ToolError::ExecutionError(format!("Failed to connect to database: {}", e)))?;

    let (query, target_label) = match request.mode {
        PostgresqlMode::Query { query } => (query, None),
        PostgresqlMode::Reflect { target, table_name, schema_name } => (
            build_reflection_query(&target, table_name.as_deref(), schema_name.as_deref())?,
            Some(target),
        ),
    };

    let result = executor
        .execute(&query, Some(limit))
        .await
        .map_err(|e| ToolError::ExecutionError(format!("Query execution failed: {}", e)))?;

    let formatted_output = match target_label {
        Some(target) => {
            format!(
                "Schema Reflection ({}):\n{}",
                target,
                formatter::format_query_result(&result)
            )
        }
        None => formatter::format_query_result(&result),
    };

    Ok(ToolResponse::PostgresqlReader(PostgresqlReaderResponse {
        columns: result.columns,
        rows: result.rows,
        row_count: result.row_count,
        truncated: result.truncated,
        execution_time_ms: result.execution_time_ms,
        formatted_output,
    }))
}
```

#### 3.2 Update ToolExecutor

**File**: `manager-tools/src/tool_executor.rs`

Add postgresql import:
```rust
use crate::postgresql_reader;
```

Add match arm in `execute()` method:
```rust
pub async fn execute(&self, request: ToolRequest) -> Result<ToolResponse> {
    match request {
        // ... existing match arms
        ToolRequest::PostgresqlReader(req) => {
            postgresql_reader::execute_postgresql_reader(req).await
        }
    }
}
```

### Phase 4: Add Dependencies

#### 4.1 Update Cargo.toml

**File**: `manager-tools/Cargo.toml`

Add dependencies:
```toml
[dependencies]
# ... existing dependencies
tokio-postgres = "0.7"
postgres-types = "0.2"
```

Note: `sqlparser` is already a dependency from sqlite_analysis.

### Phase 5: Security Considerations

#### 5.1 Password Handling

**Security measures:**
1. Passwords are passed in request but not logged
2. Connection strings built securely
3. Connections are short-lived (closed after query)
4. No connection pooling reduces attack surface

#### 5.2 Query Validation

**Multi-layer validation:**
1. AST parsing using `sqlparser` with PostgreSQL dialect
2. Only SELECT statements allowed
3. Dangerous keywords blocked
4. Multiple statements blocked
5. Row limits enforced
6. Query timeouts enforced

#### 5.3 SSL/TLS Support

**Connection security:**
- Support `disable` mode for local development
- Support `prefer` mode (attempt SSL, fallback to plain)
- Support `require` mode for production (SSL required)
- Future: Support certificate validation

### Phase 6: Testing

#### 6.1 Unit Tests

**File**: `manager-tools/src/postgresql_reader/executor.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_validation() {
        // Test SELECT queries are allowed
        // Test INSERT/UPDATE/DELETE are blocked
        // Test multiple statements are blocked
        // Test dangerous keywords are blocked
    }

    // Additional tests similar to sqlite
}
```

#### 6.2 Integration Tests

**File**: `manager-tools/src/postgresql_reader/mod.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Note: Integration tests require a running PostgreSQL instance
    // Consider using testcontainers-rs for automated testing

    #[tokio::test]
    async fn test_postgresql_reader_query_mode() {
        // Setup test database
        // Execute SELECT query
        // Verify results
    }

    #[tokio::test]
    async fn test_postgresql_reader_reflect_mode() {
        // Setup test database with tables
        // Execute reflection queries
        // Verify schema information
    }
}
```

### Phase 7: Documentation

#### 7.1 Module Documentation

**File**: `manager-tools/src/postgresql_reader/mod.rs`

Add comprehensive module documentation similar to sqlite_analysis:
- Security model
- Usage examples
- Supported reflection targets
- Error handling
- Connection parameters

#### 7.2 Update manager-tools README

**File**: `manager-tools/README.md`

Add section documenting the new tool:
```markdown
### PostgreSQL Reader Tool

Read-only PostgreSQL database query tool for AI agents.

**Features:**
- Execute SELECT queries with full PostgreSQL SQL support
- Automatic query validation and SQL injection prevention
- Row limiting and query timeouts
- Schema introspection via information_schema
- SSL/TLS connection support
- Table-formatted output optimized for LLMs

**Usage Example:**
\`\`\`rust
use manager_tools::{ToolExecutor, ToolRequest, PostgresqlReaderRequest, PostgresqlConnection, PostgresqlMode};

let request = ToolRequest::PostgresqlReader(PostgresqlReaderRequest {
    connection: PostgresqlConnection {
        host: "localhost".to_string(),
        port: Some(5432),
        database: "mydb".to_string(),
        user: "readonly_user".to_string(),
        password: "secure_password".to_string(),
        ssl_mode: Some("prefer".to_string()),
    },
    mode: PostgresqlMode::Query {
        query: "SELECT * FROM users LIMIT 10".to_string(),
    },
    limit: Some(10),
});

let response = executor.execute(request).await?;
\`\`\`

**Security:**
- Only SELECT queries allowed
- Blocks INSERT, UPDATE, DELETE, DROP, etc.
- SQL injection protection via AST parsing
- SSL/TLS connection support
- Short-lived connections (no pooling)
```

## Files Changed

### New Files
- `manager-tools/src/postgresql_reader/mod.rs` - Main module
- `manager-tools/src/postgresql_reader/executor.rs` - PostgresqlExecutor implementation
- `manager-tools/src/postgresql_reader/formatter.rs` - Result formatting (reuses sqlite)
- `manager-tools/src/types/postgresql_reader.rs` - Request/Response types
- `manager-tools/tasks/add-postgresql-reader-tool.md` - This task document

### Modified Files
- `manager-tools/Cargo.toml` - Add tokio-postgres dependency
- `manager-tools/src/lib.rs` - Add postgresql_reader module
- `manager-tools/src/types/mod.rs` - Add postgresql_reader types export
- `manager-tools/src/types/core.rs` - Add PostgresqlReader variants
- `manager-tools/src/tool_executor.rs` - Add postgresql_reader execution
- `manager-tools/README.md` - Document new tool

## Testing & Validation

### Unit Tests
```bash
cd manager-tools
cargo test postgresql_reader
```

### Integration Tests (requires PostgreSQL)
```bash
# Start PostgreSQL container for testing
docker run -d --name postgres-test \
  -e POSTGRES_PASSWORD=test \
  -e POSTGRES_DB=testdb \
  -p 5432:5432 \
  postgres:16

# Run tests
cd manager-tools
cargo test postgresql_reader -- --ignored

# Cleanup
docker stop postgres-test && docker rm postgres-test
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
- [ ] Connection to PostgreSQL succeeds
- [ ] SELECT query returns correct results
- [ ] Reflection mode returns schema information
- [ ] INSERT/UPDATE/DELETE are blocked
- [ ] Multiple statements are blocked
- [ ] SQL injection attempts are blocked
- [ ] Row limits are enforced
- [ ] Query timeouts work
- [ ] SSL connections work
- [ ] Invalid credentials error gracefully
- [ ] Network errors handled gracefully
- [ ] Formatted output is readable

## Success Criteria

- [ ] postgresql_reader tool integrated into manager-tools
- [ ] All unit tests pass
- [ ] Integration tests pass (with PostgreSQL)
- [ ] No clippy warnings
- [ ] Code properly formatted
- [ ] Documentation complete
- [ ] SELECT queries execute successfully
- [ ] information_schema queries work for schema introspection
- [ ] Write operations strictly blocked
- [ ] SQL injection protection working
- [ ] SSL/TLS connections supported
- [ ] Ready for use in production

## Comparison with SQLite Tool

| Feature | SQLite | PostgreSQL |
|---------|--------|------------|
| Connection | File path | Host/port/credentials |
| Async | No (blocking) | Yes (tokio) |
| Schema queries | PRAGMA | information_schema |
| SSL/TLS | N/A | Supported |
| Parser dialect | GenericDialect | PostgreSqlDialect |
| Connection lifecycle | Per-request | Per-request (consistent) |
| Formatter | Custom | Reused from SQLite |

## Future Enhancements

1. **Connection pooling**: For high-frequency queries
2. **Certificate validation**: For production SSL connections
3. **Query caching**: For repeated queries
4. **Prepared statements**: For parameterized queries
5. **Transaction support**: For multi-query analysis (read-only)
6. **Connection string support**: Alternative to individual parameters
7. **Schema caching**: Cache information_schema results

## References

- **Similar implementation**: `manager-tools/src/sqlite_analysis/`
- **tokio-postgres docs**: https://docs.rs/tokio-postgres/
- **PostgreSQL information_schema**: https://www.postgresql.org/docs/current/information-schema.html
- **sqlparser docs**: https://docs.rs/sqlparser/

## Notes

- This is a pure addition - no breaking changes to existing tools
- The tool is designed for analysis, not data modification
- Follows proven architecture from sqlite_analysis
- SSL/TLS support enables production use
- information_schema provides standard schema introspection
- Password security: connections are short-lived, passwords not logged
- Consider using read-only database users in production
