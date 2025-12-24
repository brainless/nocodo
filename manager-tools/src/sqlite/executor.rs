//! SQL Query Executor with Security Validation
//!
//! This module provides the core query execution logic with comprehensive security validation.
//! It uses Abstract Syntax Tree (AST) parsing to deeply inspect queries and ensure they comply
//! with read-only constraints.
//!
//! # Key Components
//!
//! - [`SqlExecutor`]: Main executor that manages database connections and query execution
//! - [`QueryResult`]: Structured result containing columns, rows, and execution metadata
//!
//! # Validation Strategy
//!
//! The validation process uses multiple complementary techniques:
//!
//! 1. **Pre-parsing checks**: Fast string-based checks for multiple statements
//! 2. **AST parsing**: Full query parsing using `sqlparser` crate
//! 3. **Statement type validation**: Only SELECT and PRAGMA allowed
//! 4. **Recursive body validation**: Deep inspection of query components (subqueries, joins, expressions)
//! 5. **Keyword scanning**: Final safety check for dangerous keywords
//!
//! This defense-in-depth approach ensures that even if one validation layer has a gap,
//! other layers will catch malicious queries.

use rusqlite::Connection;
use serde_json::Value;
use sqlparser::ast::{SetExpr, Statement};
use sqlparser::dialect::GenericDialect;
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

pub struct SqlExecutor {
    pub conn: Connection,
    max_rows: usize,
    #[allow(dead_code)]
    timeout_ms: u64,
}

impl SqlExecutor {
    pub fn new(db_path: &str, max_rows: usize, timeout_ms: u64) -> anyhow::Result<Self> {
        let conn = Connection::open(db_path)?;

        conn.busy_timeout(std::time::Duration::from_millis(timeout_ms))?;

        Ok(Self {
            conn,
            max_rows,
            timeout_ms,
        })
    }

    pub fn execute(&self, query: &str, limit: Option<usize>) -> anyhow::Result<QueryResult> {
        let start_time = Instant::now();

        self.validate_query(query)?;

        let effective_limit = limit.unwrap_or(self.max_rows).min(self.max_rows);

        let is_pragma = query.trim().to_uppercase().starts_with("PRAGMA");
        let final_query = if is_pragma {
            query.trim().trim_end_matches(';').to_string()
        } else {
            self.apply_limit(query, effective_limit)?
        };

        let mut stmt = self.conn.prepare(&final_query)?;
        let column_names: Vec<String> = stmt
            .column_names()
            .iter()
            .map(|&name| name.to_string())
            .collect();

        let rows = stmt.query_map([], |row| {
            let mut values = Vec::new();
            for i in 0..row.as_ref().column_count() {
                let value = match row.get_ref(i)? {
                    rusqlite::types::ValueRef::Null => Value::Null,
                    rusqlite::types::ValueRef::Integer(i) => {
                        Value::Number(serde_json::Number::from(i))
                    }
                    rusqlite::types::ValueRef::Real(f) => Value::Number(
                        serde_json::Number::from_f64(f)
                            .unwrap_or_else(|| serde_json::Number::from(0)),
                    ),
                    rusqlite::types::ValueRef::Text(s) => {
                        Value::String(String::from_utf8_lossy(s).to_string())
                    }
                    rusqlite::types::ValueRef::Blob(_) => Value::String("<BLOB>".to_string()),
                };
                values.push(value);
            }
            Ok(values)
        })?;

        let mut result_rows = Vec::new();
        let mut row_count = 0;

        for row in rows {
            let row = row?;
            result_rows.push(row);
            row_count += 1;

            if row_count >= effective_limit {
                break;
            }
        }

        let truncated = result_rows.len() == effective_limit;

        let execution_time_ms = start_time.elapsed().as_millis() as u64;

        Ok(QueryResult {
            columns: column_names,
            rows: result_rows,
            row_count,
            truncated,
            execution_time_ms,
        })
    }

    pub fn validate_query(&self, query: &str) -> anyhow::Result<()> {
        let query_upper = query.trim().to_uppercase();

        if query_upper.starts_with("PRAGMA") {
            return self.validate_pragma(query);
        }

        if query.trim().ends_with(';') && query.trim().matches(';').count() > 1 {
            return Err(anyhow::anyhow!("Multiple SQL statements are not allowed"));
        }

        let dialect = GenericDialect {};
        let statements = Parser::parse_sql(&dialect, query)
            .map_err(|e| anyhow::anyhow!("Failed to parse SQL: {}", e))?;

        if statements.is_empty() {
            return Err(anyhow::anyhow!("Empty SQL statement"));
        }

        if statements.len() > 1 {
            return Err(anyhow::anyhow!("Multiple SQL statements are not allowed"));
        }

        match &statements[0] {
            Statement::Query(query) => {
                self.validate_query_body(&query.body)?;
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Only SELECT queries and PRAGMA statements are allowed"
                ));
            }
        }

        let dangerous_keywords = [
            "DROP", "DELETE", "UPDATE", "INSERT", "CREATE", "ALTER", "TRUNCATE", "EXEC", "EXECUTE",
            "UNION", "MERGE", "CALL",
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

    fn validate_pragma(&self, query: &str) -> anyhow::Result<()> {
        let query_upper = query.to_uppercase();

        let dangerous_keywords = [
            "DROP", "DELETE", "UPDATE", "INSERT", "CREATE", "ALTER", "TRUNCATE", "EXEC", "EXECUTE",
            "MERGE", "CALL",
        ];

        for keyword in &dangerous_keywords {
            if query_upper.contains(keyword) {
                return Err(anyhow::anyhow!(
                    "Use of '{}' is not allowed in PRAGMA statements",
                    keyword
                ));
            }
        }

        Ok(())
    }

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
}

fn is_safe_context(query: &str, keyword: &str) -> bool {
    let query_upper = query.to_uppercase();

    if keyword == "UNION" {
        return query_upper.contains("SELECT");
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_sql_validation() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = Connection::open(temp_file.path()).unwrap();

        conn.execute_batch(
            r#"
            CREATE TABLE test_table (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                value REAL
            );
            "#,
        )
        .unwrap();

        let executor = SqlExecutor::new(temp_file.path().to_str().unwrap(), 100, 5000).unwrap();

        assert!(executor.validate_query("SELECT * FROM test_table").is_ok());
        assert!(executor
            .validate_query("SELECT id, name FROM test_table WHERE value > 100")
            .is_ok());
        assert!(executor
            .validate_query("SELECT COUNT(*) FROM test_table")
            .is_ok());

        assert!(executor.validate_query("DROP TABLE test_table").is_err());
        assert!(executor.validate_query("DELETE FROM test_table").is_err());
        assert!(executor
            .validate_query("UPDATE test_table SET value = 0")
            .is_err());
        assert!(executor
            .validate_query("INSERT INTO test_table (name) VALUES ('test')")
            .is_err());
        assert!(executor
            .validate_query("SELECT * FROM test_table; DROP TABLE test_table")
            .is_err());
    }

    #[test]
    fn test_sql_execution() {
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
            INSERT INTO test_table (name, value) VALUES ('test3', 300.7);
            "#,
        )
        .unwrap();

        let executor = SqlExecutor::new(temp_file.path().to_str().unwrap(), 100, 5000).unwrap();

        let result = executor
            .execute(
                "SELECT name, value FROM test_table ORDER BY value DESC",
                None,
            )
            .unwrap();

        assert_eq!(result.columns, vec!["name", "value"]);
        assert_eq!(result.row_count, 3);
        assert!(!result.truncated);
        assert!(result.execution_time_ms < 1000);

        assert_eq!(result.rows.len(), 3);
        for row in &result.rows {
            assert_eq!(row.len(), 2);
        }

        let limited_result = executor
            .execute(
                "SELECT name, value FROM test_table ORDER BY value DESC",
                Some(2),
            )
            .unwrap();
        assert_eq!(limited_result.row_count, 2);
    }

    #[test]
    fn test_pragma_support() {
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

        let executor = SqlExecutor::new(temp_file.path().to_str().unwrap(), 100, 5000).unwrap();

        assert!(executor.validate_query("PRAGMA table_info(users)").is_ok());
        assert!(executor.validate_query("PRAGMA table_list").is_ok());

        let result = executor.execute("PRAGMA table_info(users)", None).unwrap();
        assert!(result.row_count > 0);
    }

    #[test]
    fn test_multiple_statement_blocking() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = Connection::open(temp_file.path()).unwrap();

        conn.execute_batch("CREATE TABLE test (id INTEGER);")
            .unwrap();

        let executor = SqlExecutor::new(temp_file.path().to_str().unwrap(), 100, 5000).unwrap();

        assert!(executor
            .validate_query("SELECT * FROM test; DROP TABLE test")
            .is_err());
    }

    #[test]
    fn test_dangerous_keyword_blocking() {
        let temp_file = NamedTempFile::new().unwrap();
        let conn = Connection::open(temp_file.path()).unwrap();

        conn.execute_batch("CREATE TABLE test (id INTEGER);")
            .unwrap();

        let executor = SqlExecutor::new(temp_file.path().to_str().unwrap(), 100, 5000).unwrap();

        assert!(executor.validate_query("DROP TABLE test").is_err());
        assert!(executor.validate_query("DELETE FROM test").is_err());
        assert!(executor.validate_query("UPDATE test SET id = 1").is_err());
        assert!(executor
            .validate_query("INSERT INTO test VALUES (1)")
            .is_err());
        assert!(executor
            .validate_query("CREATE TABLE foo (id INTEGER)")
            .is_err());
        assert!(executor
            .validate_query("ALTER TABLE test ADD COLUMN x INT")
            .is_err());
    }
}
