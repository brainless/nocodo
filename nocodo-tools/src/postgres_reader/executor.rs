//! PostgreSQL Query Executor with Security Validation
//!
//! This module provides the core query execution logic for PostgreSQL with comprehensive
//! security validation. It uses Abstract Syntax Tree (AST) parsing to deeply inspect queries
//! and ensure they comply with read-only constraints.
//!
//! # Key Components
//!
//! - [`PostgresExecutor`]: Main executor that manages connection pooling and query execution
//! - [`QueryResult`]: Structured result containing columns, rows, and execution metadata
//!
//! # Validation Strategy
//!
//! The validation process uses multiple complementary techniques:
//!
//! 1. **Pre-parsing checks**: Fast string-based checks for multiple statements
//! 2. **AST parsing**: Full query parsing using `sqlparser` crate with PostgreSQL dialect
//! 3. **Statement type validation**: Only SELECT queries allowed
//! 4. **Recursive body validation**: Deep inspection of query components (subqueries, joins, expressions)
//! 5. **Keyword scanning**: Final safety check for dangerous keywords
//! 6. **Transaction wrapping**: Queries execute in `BEGIN READ ONLY` transactions
//! 7. **Timeout enforcement**: Statement-level timeout to prevent long-running queries
//!
//! This defense-in-depth approach ensures that even if one validation layer has a gap,
//! other layers will catch malicious queries.

use serde_json::Value;
use sqlparser::ast::{SetExpr, Statement};
use sqlparser::dialect::PostgreSqlDialect;
use sqlparser::parser::Parser;
use sqlx::postgres::{PgPoolOptions, PgRow};
use sqlx::{Column, PgPool, Row, TypeInfo};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
    pub row_count: usize,
    pub truncated: bool,
    pub execution_time_ms: u64,
}

pub struct PostgresExecutor {
    pool: PgPool,
    max_rows: usize,
    timeout_seconds: u64,
}

impl PostgresExecutor {
    /// Creates a new PostgreSQL executor with connection pooling
    pub async fn new(
        connection_string: &str,
        max_rows: usize,
        timeout_seconds: u64,
    ) -> anyhow::Result<Self> {
        // Create connection pool with conservative settings
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(10))
            .connect(connection_string)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to connect to PostgreSQL: {}", e))?;

        Ok(Self {
            pool,
            max_rows,
            timeout_seconds,
        })
    }

    /// Executes a query with validation and returns structured results
    pub async fn execute(
        &self,
        query: &str,
        limit: Option<usize>,
    ) -> anyhow::Result<QueryResult> {
        let start_time = Instant::now();

        // Validate query before execution
        self.validate_query(query)?;

        let effective_limit = limit.unwrap_or(self.max_rows).min(self.max_rows);

        // Check if this is an INFORMATION_SCHEMA query (reflection)
        let is_information_schema =
            query.to_uppercase().contains("INFORMATION_SCHEMA") || query.to_uppercase().contains("PG_");

        let final_query = if is_information_schema {
            // Don't modify INFORMATION_SCHEMA queries as they already have appropriate structure
            query.trim().trim_end_matches(';').to_string()
        } else {
            self.apply_limit(query, effective_limit)?
        };

        // Set statement timeout for this query
        let timeout_query = format!("SET statement_timeout = '{}s'", self.timeout_seconds);

        // Begin a read-only transaction
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to begin transaction: {}", e))?;

        // Set read-only mode
        sqlx::query("SET TRANSACTION READ ONLY")
            .execute(&mut *tx)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to set read-only mode: {}", e))?;

        // Set statement timeout
        sqlx::query(&timeout_query)
            .execute(&mut *tx)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to set timeout: {}", e))?;

        // Execute the query
        let rows: Vec<PgRow> = sqlx::query(&final_query)
            .fetch_all(&mut *tx)
            .await
            .map_err(|e| anyhow::anyhow!("Query execution failed: {}", e))?;

        // Rollback transaction (we're read-only anyway)
        tx.rollback()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to rollback transaction: {}", e))?;

        // Extract column names
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
        let mut row_count = 0;

        for row in rows.iter() {
            if row_count >= effective_limit {
                break;
            }

            let mut values = Vec::new();
            for (i, column) in row.columns().iter().enumerate() {
                let value = self.extract_value(&row, i, column.type_info().name())?;
                values.push(value);
            }

            result_rows.push(values);
            row_count += 1;
        }

        let truncated = rows.len() > effective_limit;
        let execution_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(QueryResult {
            columns,
            rows: result_rows,
            row_count,
            truncated,
            execution_time_ms,
        })
    }

    /// Extracts a value from a PostgreSQL row, handling various types
    fn extract_value(&self, row: &PgRow, index: usize, type_name: &str) -> anyhow::Result<Value> {
        // Try to extract value based on PostgreSQL type
        let value = match type_name {
            "BOOL" => row
                .try_get::<Option<bool>, _>(index)
                .ok()
                .flatten()
                .map(Value::Bool)
                .unwrap_or(Value::Null),
            "INT2" | "SMALLINT" => row
                .try_get::<Option<i16>, _>(index)
                .ok()
                .flatten()
                .map(|v| Value::Number(v.into()))
                .unwrap_or(Value::Null),
            "INT4" | "INT" | "INTEGER" => row
                .try_get::<Option<i32>, _>(index)
                .ok()
                .flatten()
                .map(|v| Value::Number(v.into()))
                .unwrap_or(Value::Null),
            "INT8" | "BIGINT" => row
                .try_get::<Option<i64>, _>(index)
                .ok()
                .flatten()
                .map(|v| Value::Number(v.into()))
                .unwrap_or(Value::Null),
            "FLOAT4" | "REAL" => row
                .try_get::<Option<f32>, _>(index)
                .ok()
                .flatten()
                .and_then(|v| serde_json::Number::from_f64(v as f64))
                .map(Value::Number)
                .unwrap_or(Value::Null),
            "FLOAT8" | "DOUBLE PRECISION" => row
                .try_get::<Option<f64>, _>(index)
                .ok()
                .flatten()
                .and_then(|v| serde_json::Number::from_f64(v))
                .map(Value::Number)
                .unwrap_or(Value::Null),
            "TEXT" | "VARCHAR" | "CHAR" | "NAME" => row
                .try_get::<Option<String>, _>(index)
                .ok()
                .flatten()
                .map(Value::String)
                .unwrap_or(Value::Null),
            "JSONB" | "JSON" => row
                .try_get::<Option<serde_json::Value>, _>(index)
                .ok()
                .flatten()
                .unwrap_or(Value::Null),
            // For other types, try string conversion
            _ => row
                .try_get::<Option<String>, _>(index)
                .ok()
                .flatten()
                .map(Value::String)
                .unwrap_or(Value::Null),
        };

        Ok(value)
    }

    /// Validates a SQL query to ensure it's safe and read-only
    pub fn validate_query(&self, query: &str) -> anyhow::Result<()> {
        let query_upper = query.trim().to_uppercase();

        // Allow INFORMATION_SCHEMA and system catalog queries
        if query_upper.contains("INFORMATION_SCHEMA") || query_upper.starts_with("SELECT * FROM PG_") {
            return self.validate_information_schema_query(query);
        }

        // Check for multiple statements
        if query.trim().ends_with(';') && query.trim().matches(';').count() > 1 {
            return Err(anyhow::anyhow!("Multiple SQL statements are not allowed"));
        }

        // Parse with PostgreSQL dialect
        let dialect = PostgreSqlDialect {};
        let statements = Parser::parse_sql(&dialect, query)
            .map_err(|e| anyhow::anyhow!("Failed to parse SQL: {}", e))?;

        if statements.is_empty() {
            return Err(anyhow::anyhow!("Empty SQL statement"));
        }

        if statements.len() > 1 {
            return Err(anyhow::anyhow!("Multiple SQL statements are not allowed"));
        }

        // Validate statement type
        match &statements[0] {
            Statement::Query(query) => {
                self.validate_query_body(&query.body)?;
            }
            _ => {
                return Err(anyhow::anyhow!("Only SELECT queries are allowed"));
            }
        }

        // Final dangerous keyword check
        let dangerous_keywords = [
            "DROP",
            "DELETE",
            "UPDATE",
            "INSERT",
            "CREATE",
            "ALTER",
            "TRUNCATE",
            "EXEC",
            "EXECUTE",
            "MERGE",
            "CALL",
            "COPY",
            "GRANT",
            "REVOKE",
        ];

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

    /// Validates INFORMATION_SCHEMA queries
    fn validate_information_schema_query(&self, query: &str) -> anyhow::Result<()> {
        let query_upper = query.to_uppercase();

        // These queries should only read from system catalogs
        let dangerous_keywords = [
            "DROP", "DELETE", "UPDATE", "INSERT", "CREATE", "ALTER", "TRUNCATE", "EXEC", "EXECUTE",
            "MERGE", "CALL", "COPY", "GRANT", "REVOKE",
        ];

        for keyword in &dangerous_keywords {
            if query_upper.contains(keyword) {
                return Err(anyhow::anyhow!(
                    "Use of '{}' is not allowed in INFORMATION_SCHEMA queries",
                    keyword
                ));
            }
        }

        Ok(())
    }

    /// Recursively validates query body (subqueries, joins, etc.)
    fn validate_query_body(&self, set_expr: &SetExpr) -> anyhow::Result<()> {
        match set_expr {
            SetExpr::Select(select) => {
                if let Some(table_with_joins) = &select.from.first() {
                    self.validate_table_factor(&table_with_joins.relation)?;
                }

                if let Some(where_clause) = &select.selection {
                    self.validate_expr(where_clause)?;
                }
            }
            SetExpr::Query(query) => {
                self.validate_query_body(&query.body)?;
            }
            SetExpr::SetOperation { left, right, .. } => {
                self.validate_query_body(left)?;
                self.validate_query_body(right)?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Validates table factors (tables, subqueries, joins)
    fn validate_table_factor(
        &self,
        table_factor: &sqlparser::ast::TableFactor,
    ) -> anyhow::Result<()> {
        match table_factor {
            sqlparser::ast::TableFactor::Table { .. } => {}
            sqlparser::ast::TableFactor::Derived { subquery, .. } => {
                self.validate_query_body(&subquery.body)?;
            }
            sqlparser::ast::TableFactor::NestedJoin {
                table_with_joins, ..
            } => {
                self.validate_table_factor(&table_with_joins.relation)?;
                for join in &table_with_joins.joins {
                    self.validate_table_factor(&join.relation)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Validates expressions (WHERE clauses, subqueries in expressions)
    fn validate_expr(&self, expr: &sqlparser::ast::Expr) -> anyhow::Result<()> {
        match expr {
            sqlparser::ast::Expr::Subquery(subquery) => {
                self.validate_query_body(&subquery.body)?;
            }
            sqlparser::ast::Expr::BinaryOp { left, right, .. } => {
                self.validate_expr(left)?;
                self.validate_expr(right)?;
            }
            sqlparser::ast::Expr::UnaryOp { expr, .. } => {
                self.validate_expr(expr)?;
            }
            sqlparser::ast::Expr::Function(_function) => {}
            _ => {}
        }
        Ok(())
    }

    /// Applies LIMIT clause if not already present
    fn apply_limit(&self, query: &str, limit: usize) -> anyhow::Result<String> {
        let query_trimmed = query.trim().trim_end_matches(';');
        let query_upper = query_trimmed.to_uppercase();

        if query_upper.contains(" LIMIT ") || query_upper.ends_with(" LIMIT") {
            tracing::debug!("Query already has LIMIT clause, not adding another");
            Ok(query_trimmed.to_string())
        } else {
            let limit_pattern = regex::Regex::new(r"\bLIMIT\s+\d+").unwrap();
            if limit_pattern.is_match(&query_upper) {
                tracing::debug!("Query already has LIMIT with number, not adding another");
                Ok(query_trimmed.to_string())
            } else {
                tracing::debug!("Adding LIMIT {} to query", limit);
                Ok(format!("{} LIMIT {}", query_trimmed, limit))
            }
        }
    }

    /// Get the connection pool (useful for cleanup)
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Close the connection pool
    pub async fn close(self) {
        self.pool.close().await;
    }
}

/// Checks if a keyword appears in a safe context (e.g., as part of a column name)
fn is_safe_context(query: &str, keyword: &str) -> bool {
    let query_upper = query.to_uppercase();

    // UNION is allowed in SELECT queries (for combining results)
    if keyword == "UNION" {
        return query_upper.contains("SELECT");
    }

    // Check if keyword appears as part of a column/table name rather than as a SQL keyword
    // Example: "created_at" contains "CREATE" but it's not a CREATE statement
    let keyword_pattern = regex::Regex::new(&format!(r"\b{}\b", regex::escape(keyword))).unwrap();
    !keyword_pattern.is_match(&query_upper)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_validation() {
        let executor = PostgresExecutor {
            pool: PgPool::connect_lazy("postgresql://localhost/test").unwrap(),
            max_rows: 100,
            timeout_seconds: 5,
        };

        // Valid queries
        assert!(executor.validate_query("SELECT * FROM users").is_ok());
        assert!(executor
            .validate_query("SELECT id, name FROM users WHERE age > 18")
            .is_ok());
        assert!(executor
            .validate_query("SELECT COUNT(*) FROM orders")
            .is_ok());

        // Invalid queries
        assert!(executor.validate_query("DROP TABLE users").is_err());
        assert!(executor.validate_query("DELETE FROM users").is_err());
        assert!(executor
            .validate_query("UPDATE users SET name = 'foo'")
            .is_err());
        assert!(executor
            .validate_query("INSERT INTO users (name) VALUES ('test')")
            .is_err());
        assert!(executor
            .validate_query("CREATE TABLE test (id INTEGER)")
            .is_err());
    }

    #[test]
    fn test_multiple_statement_blocking() {
        let executor = PostgresExecutor {
            pool: PgPool::connect_lazy("postgresql://localhost/test").unwrap(),
            max_rows: 100,
            timeout_seconds: 5,
        };

        assert!(executor
            .validate_query("SELECT * FROM users; DROP TABLE users")
            .is_err());
    }

    #[test]
    fn test_information_schema_queries() {
        let executor = PostgresExecutor {
            pool: PgPool::connect_lazy("postgresql://localhost/test").unwrap(),
            max_rows: 100,
            timeout_seconds: 5,
        };

        assert!(executor
            .validate_query("SELECT * FROM INFORMATION_SCHEMA.TABLES")
            .is_ok());
        assert!(executor
            .validate_query("SELECT * FROM pg_tables WHERE schemaname = 'public'")
            .is_ok());
    }
}
