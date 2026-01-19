//! Query Result Formatting for LLM Consumption
//!
//! This module formats SQL query results into human-readable table format optimized for
//! Large Language Models and human readers.
//!
//! # Features
//!
//! - **Table formatting**: Results displayed in aligned column format with separators
//! - **Metadata**: Includes row count, execution time, and truncation notices
//! - **Smart truncation**: Long strings truncated to 50 characters, large result sets show first 20 rows
//! - **Type-aware formatting**: Proper handling of NULL, numbers, strings, booleans, and BLOBs
//!
//! # Example Output
//!
//! ```text
//! Query executed successfully. Returned 3 rows.
//! Execution time: 15ms
//!
//! id | name  | email
//! ---+-------+------------------
//! 1  | Alice | alice@example.com
//! 2  | Bob   | bob@example.com
//! 3  | Carol | carol@example.com
//! ```

pub fn format_query_result(result: &crate::sqlite_reader::executor::QueryResult) -> String {
    if result.row_count == 0 {
        return "Query executed successfully but returned no rows.".to_string();
    }

    let mut output = String::new();

    output.push_str(&format!(
        "Query executed successfully. Returned {} rows",
        result.row_count
    ));

    if result.truncated {
        output.push_str(&format!(
            " (results truncated to {} rows)",
            result.row_count
        ));
    }

    output.push_str(&format!(
        ".\nExecution time: {}ms\n\n",
        result.execution_time_ms
    ));

    if !result.columns.is_empty() && !result.rows.is_empty() {
        let mut col_widths: Vec<usize> = result.columns.iter().map(|c| c.len()).collect();

        for row in &result.rows {
            for (i, cell) in row.iter().enumerate() {
                if i < col_widths.len() {
                    let cell_str = format_cell_value(cell);
                    col_widths[i] = col_widths[i].max(cell_str.len());
                }
            }
        }

        let header_row: Vec<String> = result
            .columns
            .iter()
            .enumerate()
            .map(|(i, col)| format!("{:<width$}", col, width = col_widths[i]))
            .collect();
        output.push_str(&header_row.join(" | "));
        output.push('\n');

        let separator: Vec<String> = col_widths
            .iter()
            .map(|&width| format!("{:<width$}", "-".repeat(width), width = width))
            .collect();
        output.push_str(&separator.join("-+-"));
        output.push('\n');

        let rows_to_show = result.rows.iter().take(20);
        for row in rows_to_show {
            let formatted_row: Vec<String> = row
                .iter()
                .enumerate()
                .map(|(i, cell)| {
                    let cell_str = format_cell_value(cell);
                    format!("{:<width$}", cell_str, width = col_widths[i])
                })
                .collect();
            output.push_str(&formatted_row.join(" | "));
            output.push('\n');
        }

        if result.row_count > 20 {
            output.push_str(&format!("\n... and {} more rows", result.row_count - 20));
        }
    }

    output
}

fn format_cell_value(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "NULL".to_string(),
        serde_json::Value::String(s) => {
            if s.len() > 50 {
                format!("{}...", &s[..47])
            } else {
                s.clone()
            }
        }
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        _ => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sqlite_reader::executor::QueryResult;
    use serde_json::Value;

    #[test]
    fn test_format_cell_value() {
        assert_eq!(format_cell_value(&Value::Null), "NULL");
        assert_eq!(
            format_cell_value(&Value::String("test".to_string())),
            "test"
        );
        assert_eq!(
            format_cell_value(&Value::String("a".repeat(60))),
            format!("{}...", "a".repeat(47))
        );
        assert_eq!(
            format_cell_value(&Value::Number(serde_json::Number::from(42))),
            "42"
        );
        assert_eq!(format_cell_value(&Value::Bool(true)), "true");
    }

    #[test]
    fn test_format_query_result() {
        let result = QueryResult {
            columns: vec!["id".to_string(), "name".to_string()],
            rows: vec![
                vec![
                    Value::Number(serde_json::Number::from(1)),
                    Value::String("test1".to_string()),
                ],
                vec![
                    Value::Number(serde_json::Number::from(2)),
                    Value::String("test2".to_string()),
                ],
            ],
            row_count: 2,
            truncated: false,
            execution_time_ms: 15,
        };

        let formatted = format_query_result(&result);

        assert!(formatted.contains("Query executed successfully"));
        assert!(formatted.contains("Returned 2 rows"));
        assert!(formatted.contains("Execution time: 15ms"));
        assert!(formatted.contains("id | name"));
        assert!(formatted.contains("test1"));
        assert!(formatted.contains("test2"));
    }

    #[test]
    fn test_format_empty_query_result() {
        let result = QueryResult {
            columns: vec!["id".to_string()],
            rows: vec![],
            row_count: 0,
            truncated: false,
            execution_time_ms: 5,
        };

        let formatted = format_query_result(&result);
        assert_eq!(
            formatted,
            "Query executed successfully but returned no rows."
        );
    }
}
