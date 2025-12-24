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
