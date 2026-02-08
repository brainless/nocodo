//! PostgreSQL Reader Tool - Read-only Database Query Execution
//!
//! This module provides a secure, read-only interface for executing SQL queries against
//! PostgreSQL databases. It is designed for AI agents and tools that need to analyze database
//! contents without risking data modification.
//!
//! # Security Model
//!
//! The postgres_reader tool implements multiple layers of security to ensure safe, read-only
//! database access:
//!
//! ## 1. Query Type Restrictions
//!
//! Only SELECT queries are allowed for retrieving data from tables. For schema introspection,
//! use the reflect mode which queries INFORMATION_SCHEMA and PostgreSQL system catalogs.
//!
//! All other statement types (INSERT, UPDATE, DELETE, DROP, CREATE, ALTER, etc.) are
//! strictly blocked and will result in validation errors.
//!
//! ## 2. SQL Injection Prevention
//!
//! Multiple techniques prevent SQL injection attacks:
//!
//! - **Single statement enforcement**: Queries containing multiple statements (separated by `;`)
//!   are rejected.
//!
//! - **AST-based validation**: Queries are parsed into an Abstract Syntax Tree using the
//!   `sqlparser` crate with PostgreSQL dialect support.
//!
//! - **Recursive validation**: Subqueries, nested expressions, joins, and derived tables are
//!   recursively validated.
//!
//! - **Keyword blocking**: Dangerous SQL keywords (DROP, DELETE, UPDATE, INSERT, CREATE, ALTER,
//!   TRUNCATE, EXEC, EXECUTE, MERGE, CALL, COPY, GRANT, REVOKE) are blocked.
//!
//! ## 3. Resource Limits
//!
//! To prevent resource exhaustion:
//!
//! - **Row limits**: Results are limited to a maximum of 1000 rows (default: 100).
//!   LIMIT clauses are automatically injected into SELECT queries if not present.
//!
//! - **Query timeouts**: Queries timeout after 5 seconds using `statement_timeout`.
//!
//! - **Output truncation**: Formatted output displays a maximum of 20 rows.
//!
//! ## 4. Transaction Isolation
//!
//! All queries execute in read-only transactions:
//! - `BEGIN READ ONLY` transaction mode enforced
//! - `statement_timeout` set for each query
//! - Transaction rolled back after query completion
//!
//! ## 5. Connection Pooling
//!
//! Uses sqlx connection pooling with conservative settings:
//! - Maximum 5 connections per pool
//! - 10 second acquire timeout
//! - Automatic connection cleanup
//!
//! # Usage Examples
//!
//! ## Query Mode - Basic SELECT
//!
//! ```rust,no_run
//! use nocodo_tools::types::{PostgresReaderRequest, PostgresMode};
//! use nocodo_tools::postgres_reader::execute_postgres_reader;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let request = PostgresReaderRequest {
//!     connection_string: "postgresql://user:pass@localhost:5432/mydb".to_string(),
//!     mode: PostgresMode::Query {
//!         query: "SELECT name, email FROM users WHERE active = true ORDER BY name".to_string(),
//!     },
//!     limit: Some(50),
//! };
//!
//! let response = execute_postgres_reader(request).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Reflect Mode - Discover Tables
//!
//! ```rust,no_run
//! # use nocodo_tools::types::{PostgresReaderRequest, PostgresMode};
//! # use nocodo_tools::postgres_reader::execute_postgres_reader;
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let request = PostgresReaderRequest {
//!     connection_string: "postgresql://user:pass@localhost:5432/mydb".to_string(),
//!     mode: PostgresMode::Reflect {
//!         target: "tables".to_string(),
//!         schema_name: Some("public".to_string()),
//!         table_name: None,
//!     },
//!     limit: None,
//! };
//!
//! let response = execute_postgres_reader(request).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Modes
//!
//! ## Query Mode
//!
//! Execute arbitrary SQL SELECT queries. The query is subject to validation to ensure it's
//! read-only and safe.
//!
//! ## Reflect Mode
//!
//! Introspect database schema without writing SQL queries. Supported targets:
//!
//! - `schema` - List all schemas in the database
//! - `tables` - List tables in a schema (defaults to 'public')
//! - `table_info` - Column information for a specific table (requires table_name)
//! - `indexes` - List indexes for a table or schema
//! - `views` - List views in a schema
//! - `foreign_keys` - Foreign key relationships for a table (requires table_name)
//! - `constraints` - All constraints for a table (requires table_name)
//! - `stats` - Table statistics (row counts, sizes)

use crate::tool_error::ToolError;
use crate::types::PostgresMode;

pub mod executor;
pub mod formatter;

pub use executor::{PostgresExecutor, QueryResult};

/// Get table names from a PostgreSQL database
pub async fn get_table_names(
    connection_string: &str,
    schema_name: Option<&str>,
) -> Result<Vec<String>, ToolError> {
    validate_connection_string(connection_string)?;

    const MAX_LIMIT: usize = 1000;
    const TIMEOUT_SECONDS: u64 = 5;

    let executor = PostgresExecutor::new(connection_string, MAX_LIMIT, TIMEOUT_SECONDS)
        .await
        .map_err(|e| ToolError::ExecutionError(format!("Failed to connect to database: {}", e)))?;

    let schema = schema_name.unwrap_or("public");
    let query = format!(
        "SELECT table_name FROM INFORMATION_SCHEMA.TABLES WHERE table_schema = '{}' AND table_type = 'BASE TABLE' ORDER BY table_name",
        schema
    );

    let result = executor
        .execute(&query, None)
        .await
        .map_err(|e| ToolError::ExecutionError(format!("Failed to get table names: {}", e)))?;

    let table_names: Vec<String> = result
        .rows
        .iter()
        .filter_map(|row| {
            if let Some(serde_json::Value::String(name)) = row.first() {
                Some(name.clone())
            } else {
                None
            }
        })
        .collect();

    Ok(table_names)
}

/// Main entry point for executing PostgreSQL reader requests
pub async fn execute_postgres_reader(
    request: crate::types::PostgresReaderRequest,
) -> Result<crate::types::ToolResponse, ToolError> {
    validate_connection_string(&request.connection_string)?;

    const DEFAULT_LIMIT: usize = 100;
    const MAX_LIMIT: usize = 1000;
    const TIMEOUT_SECONDS: u64 = 5;

    let limit = request.limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT);

    let executor = PostgresExecutor::new(&request.connection_string, MAX_LIMIT, TIMEOUT_SECONDS)
        .await
        .map_err(|e| ToolError::ExecutionError(format!("Failed to connect to database: {}", e)))?;

    let (query, target_label) = match request.mode {
        PostgresMode::Query { query } => (query, None),
        PostgresMode::Reflect {
            target,
            schema_name,
            table_name,
        } => (
            build_reflection_query(&target, schema_name.as_deref(), table_name.as_deref())?,
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

    Ok(crate::types::ToolResponse::PostgresReader(
        crate::types::PostgresReaderResponse {
            columns: result.columns,
            rows: result.rows,
            row_count: result.row_count,
            truncated: result.truncated,
            execution_time_ms: result.execution_time_ms,
            formatted_output,
        },
    ))
}

/// Builds INFORMATION_SCHEMA queries for database reflection
fn build_reflection_query(
    target: &str,
    schema_name: Option<&str>,
    table_name: Option<&str>,
) -> Result<String, ToolError> {
    let schema = schema_name.unwrap_or("public");

    let query = match target.to_lowercase().as_str() {
        "schema" => {
            "SELECT schema_name FROM INFORMATION_SCHEMA.SCHEMATA WHERE schema_name NOT IN ('pg_catalog', 'information_schema', 'pg_toast') ORDER BY schema_name".to_string()
        }
        "tables" => {
            format!(
                "SELECT table_name, table_type FROM INFORMATION_SCHEMA.TABLES WHERE table_schema = '{}' ORDER BY table_name",
                schema
            )
        }
        "table_info" => {
            match table_name {
                Some(name) => format!(
                    "SELECT column_name, data_type, is_nullable, column_default FROM INFORMATION_SCHEMA.COLUMNS WHERE table_schema = '{}' AND table_name = '{}' ORDER BY ordinal_position",
                    schema, name
                ),
                None => return Err(ToolError::InvalidInput(
                    "table_name is required for table_info reflection".to_string()
                )),
            }
        }
        "indexes" => {
            if let Some(table) = table_name {
                format!(
                    "SELECT indexname, indexdef FROM pg_indexes WHERE schemaname = '{}' AND tablename = '{}' ORDER BY indexname",
                    schema, table
                )
            } else {
                format!(
                    "SELECT indexname, tablename, indexdef FROM pg_indexes WHERE schemaname = '{}' ORDER BY tablename, indexname",
                    schema
                )
            }
        }
        "views" => {
            format!(
                "SELECT table_name, view_definition FROM INFORMATION_SCHEMA.VIEWS WHERE table_schema = '{}' ORDER BY table_name",
                schema
            )
        }
        "foreign_keys" => {
            match table_name {
                Some(name) => format!(
                    "SELECT tc.constraint_name, kcu.column_name, ccu.table_name AS foreign_table_name, ccu.column_name AS foreign_column_name \
                     FROM INFORMATION_SCHEMA.TABLE_CONSTRAINTS AS tc \
                     JOIN INFORMATION_SCHEMA.KEY_COLUMN_USAGE AS kcu ON tc.constraint_name = kcu.constraint_name \
                     JOIN INFORMATION_SCHEMA.CONSTRAINT_COLUMN_USAGE AS ccu ON ccu.constraint_name = tc.constraint_name \
                     WHERE tc.constraint_type = 'FOREIGN KEY' AND tc.table_schema = '{}' AND tc.table_name = '{}' \
                     ORDER BY tc.constraint_name",
                    schema, name
                ),
                None => return Err(ToolError::InvalidInput(
                    "table_name is required for foreign_keys reflection".to_string()
                )),
            }
        }
        "constraints" => {
            match table_name {
                Some(name) => format!(
                    "SELECT constraint_name, constraint_type FROM INFORMATION_SCHEMA.TABLE_CONSTRAINTS \
                     WHERE table_schema = '{}' AND table_name = '{}' ORDER BY constraint_type, constraint_name",
                    schema, name
                ),
                None => return Err(ToolError::InvalidInput(
                    "table_name is required for constraints reflection".to_string()
                )),
            }
        }
        "stats" => {
            if let Some(table) = table_name {
                format!(
                    "SELECT schemaname, tablename, n_live_tup AS row_count, n_dead_tup AS dead_tuples, last_autovacuum, last_analyze \
                     FROM pg_stat_user_tables WHERE schemaname = '{}' AND tablename = '{}' ORDER BY tablename",
                    schema, table
                )
            } else {
                format!(
                    "SELECT schemaname, tablename, n_live_tup AS row_count, n_dead_tup AS dead_tuples \
                     FROM pg_stat_user_tables WHERE schemaname = '{}' ORDER BY tablename",
                    schema
                )
            }
        }
        _ => {
            return Err(ToolError::InvalidInput(
                format!(
                    "Unknown reflection target: {}. Valid targets: schema, tables, table_info, indexes, views, foreign_keys, constraints, stats",
                    target
                )
            ))
        }
    };

    Ok(query)
}

/// Validates a PostgreSQL connection string format
pub fn validate_connection_string(connection_string: &str) -> Result<(), ToolError> {
    if connection_string.is_empty() {
        return Err(ToolError::InvalidInput(
            "Connection string cannot be empty".to_string(),
        ));
    }

    // Basic format validation
    if !connection_string.starts_with("postgres://")
        && !connection_string.starts_with("postgresql://")
    {
        return Err(ToolError::InvalidInput(
            "Connection string must start with 'postgresql://' or 'postgres://'".to_string(),
        ));
    }

    // Try to parse with url crate for additional validation
    match url::Url::parse(connection_string) {
        Ok(url) => {
            if url.host_str().is_none() {
                return Err(ToolError::InvalidInput(
                    "Connection string must include a host".to_string(),
                ));
            }
            Ok(())
        }
        Err(e) => Err(ToolError::InvalidInput(format!(
            "Invalid connection string format: {}",
            e
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_connection_string() {
        // Valid
        assert!(
            validate_connection_string("postgresql://localhost/test").is_ok()
        );
        assert!(validate_connection_string(
            "postgresql://user:pass@localhost:5432/testdb"
        )
        .is_ok());
        assert!(
            validate_connection_string("postgres://localhost/test").is_ok()
        );

        // Invalid
        assert!(validate_connection_string("").is_err());
        assert!(validate_connection_string("mysql://localhost/test").is_err());
        assert!(validate_connection_string("postgresql://").is_err());
    }

    #[test]
    fn test_build_reflection_query() {
        // schema target
        let query = build_reflection_query("schema", None, None).unwrap();
        assert!(query.contains("INFORMATION_SCHEMA.SCHEMATA"));

        // tables target
        let query = build_reflection_query("tables", Some("public"), None).unwrap();
        assert!(query.contains("INFORMATION_SCHEMA.TABLES"));
        assert!(query.contains("public"));

        // table_info with table_name
        let query = build_reflection_query("table_info", Some("public"), Some("users")).unwrap();
        assert!(query.contains("INFORMATION_SCHEMA.COLUMNS"));
        assert!(query.contains("users"));

        // table_info without table_name (should error)
        assert!(build_reflection_query("table_info", Some("public"), None).is_err());

        // foreign_keys with table_name
        let query = build_reflection_query("foreign_keys", Some("public"), Some("posts")).unwrap();
        assert!(query.contains("INFORMATION_SCHEMA.TABLE_CONSTRAINTS"));
        assert!(query.contains("FOREIGN KEY"));
        assert!(query.contains("posts"));

        // foreign_keys without table_name (should error)
        assert!(build_reflection_query("foreign_keys", Some("public"), None).is_err());

        // Invalid target
        assert!(build_reflection_query("invalid", Some("public"), None).is_err());
    }
}
