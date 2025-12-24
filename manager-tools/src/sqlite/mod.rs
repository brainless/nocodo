//! SQLite3 Reader Tool - Read-only Database Query Execution
//!
//! This module provides a secure, read-only interface for executing SQL queries against
//! SQLite databases. It is designed for AI agents and tools that need to analyze database
//! contents without risking data modification.
//!
//! # Security Model
//!
//! The sqlite3_reader tool implements multiple layers of security to ensure safe, read-only
//! database access:
//!
//! ## 1. Query Type Restrictions
//!
//! Only two types of SQL statements are allowed:
//! - **SELECT queries**: For retrieving data from tables
//! - **PRAGMA statements**: For schema introspection and database metadata
//!
//! All other statement types (INSERT, UPDATE, DELETE, DROP, CREATE, ALTER, etc.) are
//! strictly blocked and will result in validation errors.
//!
//! ## 2. SQL Injection Prevention
//!
//! Multiple techniques prevent SQL injection attacks:
//!
//! - **Single statement enforcement**: Queries containing multiple statements (separated by `;`)
//!   are rejected. This prevents attacks like `SELECT * FROM users; DROP TABLE users;`
//!
//! - **AST-based validation**: Queries are parsed into an Abstract Syntax Tree using the
//!   `sqlparser` crate. This ensures the query structure is valid and allows deep inspection
//!   of the query components.
//!
//! - **Recursive validation**: Subqueries, nested expressions, joins, and derived tables are
//!   recursively validated to ensure no dangerous operations are hidden in complex queries.
//!
//! - **Keyword blocking**: Dangerous SQL keywords (DROP, DELETE, UPDATE, INSERT, CREATE, ALTER,
//!   TRUNCATE, EXEC, EXECUTE, MERGE, CALL) are blocked even if they appear in contexts where
//!   the parser might allow them.
//!
//! ## 3. Resource Limits
//!
//! To prevent resource exhaustion and ensure responsive queries:
//!
//! - **Row limits**: Results are limited to a maximum of 1000 rows (default: 100).
//!   LIMIT clauses are automatically injected into SELECT queries if not present.
//!
//! - **Query timeouts**: Database operations timeout after 5000ms using SQLite's busy_timeout.
//!
//! - **Output truncation**: Formatted output displays a maximum of 20 rows, with a summary
//!   for additional rows.
//!
//! ## 4. Path Validation
//!
//! Database file paths are validated to ensure:
//! - Path is not empty
//! - File exists on the filesystem
//! - Path points to a file (not a directory)
//!
//! Future enhancement: Path allowlist/denylist for additional access control.
//!
//! ## 5. Connection Isolation
//!
//! Each query request creates a new database connection that is closed after the query
//! completes. This stateless design:
//! - Prevents connection state from affecting subsequent queries
//! - Simplifies error handling and cleanup
//! - Avoids connection pool complexity for this read-only use case
//!
//! # Usage Example
//!
//! ```rust,no_run
//! use manager_tools::types::Sqlite3ReaderRequest;
//! use manager_tools::sqlite::execute_sqlite3_reader;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Execute a SELECT query
//! let request = Sqlite3ReaderRequest {
//!     db_path: "/path/to/database.db".to_string(),
//!     query: "SELECT name, email FROM users WHERE active = 1".to_string(),
//!     limit: Some(50),
//! };
//!
//! let response = execute_sqlite3_reader(request).await?;
//!
//! // Introspect database schema with PRAGMA
//! let schema_request = Sqlite3ReaderRequest {
//!     db_path: "/path/to/database.db".to_string(),
//!     query: "PRAGMA table_info(users)".to_string(),
//!     limit: None,
//! };
//!
//! let schema_response = execute_sqlite3_reader(schema_request).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # PRAGMA Support
//!
//! PRAGMA statements are supported for schema introspection:
//!
//! - `PRAGMA table_list` - List all tables
//! - `PRAGMA table_info(table_name)` - Get column information
//! - `PRAGMA index_list(table_name)` - List indexes
//! - `PRAGMA foreign_key_list(table_name)` - List foreign keys
//!
//! PRAGMA statements undergo the same dangerous keyword validation as SELECT queries
//! but are not subject to LIMIT clause injection.
//!
//! # Error Handling
//!
//! The module returns `ToolError` for various failure conditions:
//!
//! - `ToolError::InvalidInput` - Invalid database path, empty query, or validation failure
//! - `ToolError::ExecutionError` - Database connection failure or query execution error
//!
//! All errors include descriptive messages to aid in debugging.

use crate::tool_error::ToolError;

pub mod executor;
pub mod formatter;

pub use executor::{QueryResult, SqlExecutor};

pub async fn execute_sqlite3_reader(
    request: crate::types::Sqlite3ReaderRequest,
) -> Result<crate::types::ToolResponse, ToolError> {
    validate_db_path(&request.db_path)?;

    const DEFAULT_LIMIT: usize = 100;
    const MAX_LIMIT: usize = 1000;
    const TIMEOUT_MS: u64 = 5000;

    let limit = request.limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT);

    let executor = SqlExecutor::new(&request.db_path, MAX_LIMIT, TIMEOUT_MS)
        .map_err(|e| ToolError::ExecutionError(format!("Failed to open database: {}", e)))?;

    let result = executor
        .execute(&request.query, Some(limit))
        .map_err(|e| ToolError::ExecutionError(format!("Query execution failed: {}", e)))?;

    let formatted_output = formatter::format_query_result(&result);

    Ok(crate::types::ToolResponse::Sqlite3Reader(
        crate::types::Sqlite3ReaderResponse {
            columns: result.columns,
            rows: result.rows,
            row_count: result.row_count,
            truncated: result.truncated,
            execution_time_ms: result.execution_time_ms,
            formatted_output,
        },
    ))
}

fn validate_db_path(db_path: &str) -> Result<(), ToolError> {
    if db_path.is_empty() {
        return Err(ToolError::InvalidInput(
            "Database path cannot be empty".to_string(),
        ));
    }

    let path = std::path::Path::new(db_path);
    if !path.exists() {
        return Err(ToolError::InvalidInput(format!(
            "Database file not found: {}",
            db_path
        )));
    }

    if !path.is_file() {
        return Err(ToolError::InvalidInput(format!(
            "Path is not a file: {}",
            db_path
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_sqlite3_reader_tool() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = Connection::open(temp_file.path()).unwrap();

        conn.execute_batch(
            r#"
            CREATE TABLE test_table (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                value REAL
            );
            INSERT INTO test_table (name, value) VALUES ('test1', 100.5);
            INSERT INTO test_table (name, value) VALUES ('test2', 200.3);
        "#,
        )
        .unwrap();
        drop(conn);

        let request = crate::types::Sqlite3ReaderRequest {
            db_path: temp_file.path().to_str().unwrap().to_string(),
            query: "SELECT name, value FROM test_table ORDER BY value DESC".to_string(),
            limit: None,
        };

        let response = execute_sqlite3_reader(request).await.unwrap();

        match response {
            crate::types::ToolResponse::Sqlite3Reader(result) => {
                assert_eq!(result.columns, vec!["name", "value"]);
                assert_eq!(result.row_count, 2);
                assert!(!result.truncated);
                assert!(result.formatted_output.contains("test2"));
            }
            _ => panic!("Expected Sqlite3Reader response"),
        }
    }

    #[tokio::test]
    async fn test_pragma_support() {
        use rusqlite::Connection;
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().unwrap();
        let conn = Connection::open(temp_file.path()).unwrap();

        conn.execute_batch(
            r#"
            CREATE TABLE users (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL
            );
        "#,
        )
        .unwrap();
        drop(conn);

        let request = crate::types::Sqlite3ReaderRequest {
            db_path: temp_file.path().to_str().unwrap().to_string(),
            query: "PRAGMA table_info(users)".to_string(),
            limit: None,
        };

        let response = execute_sqlite3_reader(request).await.unwrap();

        match response {
            crate::types::ToolResponse::Sqlite3Reader(result) => {
                assert!(result.row_count > 0);
                assert!(
                    result.formatted_output.contains("id")
                        || result.formatted_output.contains("name")
                );
            }
            _ => panic!("Expected Sqlite3Reader response"),
        }
    }
}
