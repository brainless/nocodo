use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "mode")]
pub enum SqliteMode {
    #[serde(rename = "query")]
    Query {
        #[schemars(
            description = "SQL query to execute. Only SELECT queries and PRAGMA statements are allowed."
        )]
        query: String,
    },

    #[serde(rename = "reflect")]
    Reflect {
        #[schemars(
            description = "Target of reflection: tables, schema, table_info, indexes, views, foreign_keys, stats"
        )]
        target: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(
            description = "Optional: specific table name for table_info and foreign_keys modes"
        )]
        table_name: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Sqlite3ReaderRequest {
    #[serde(default)]
    #[schemars(description = "Absolute path to the SQLite database file")]
    pub db_path: String,

    #[schemars(description = "Execution mode: either query or reflect")]
    pub mode: SqliteMode,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Maximum number of rows to return. Defaults to 100, maximum 1000.")]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Sqlite3ReaderResponse {
    pub columns: Vec<String>,

    pub rows: Vec<Vec<serde_json::Value>>,

    pub row_count: usize,

    pub truncated: bool,

    pub execution_time_ms: u64,

    pub formatted_output: String,
}
