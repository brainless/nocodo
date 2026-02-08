use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "mode")]
pub enum PostgresMode {
    #[serde(rename = "query")]
    Query {
        #[schemars(
            description = "SQL query to execute. Only SELECT queries are allowed. For schema information, use the reflect mode or query INFORMATION_SCHEMA tables."
        )]
        query: String,
    },

    #[serde(rename = "reflect")]
    Reflect {
        #[schemars(
            description = "Target of reflection: tables, schema, table_info, indexes, views, foreign_keys, constraints, stats. Use 'schema' to list all schemas, 'tables' to list tables in a schema."
        )]
        target: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(
            description = "Optional: specific table name for table_info, indexes, foreign_keys, constraints, and stats modes"
        )]
        table_name: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(
            description = "Optional: schema name to query. Defaults to 'public' if not specified."
        )]
        schema_name: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PostgresReaderRequest {
    #[serde(default)]
    #[schemars(
        description = "PostgreSQL connection string (postgresql://user:password@host:port/database). This field is injected by the agent and should not be provided by the LLM."
    )]
    pub connection_string: String,

    #[schemars(description = "Execution mode: either query or reflect")]
    pub mode: PostgresMode,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Maximum number of rows to return. Defaults to 100, maximum 1000."
    )]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PostgresReaderResponse {
    /// Column names from the query result
    pub columns: Vec<String>,

    /// Rows of data, each row is a vector of JSON values
    pub rows: Vec<Vec<serde_json::Value>>,

    /// Total number of rows returned
    pub row_count: usize,

    /// Whether the result was truncated due to limit
    pub truncated: bool,

    /// Query execution time in milliseconds
    pub execution_time_ms: u64,

    /// Human-readable formatted output for LLM consumption
    pub formatted_output: String,
}
