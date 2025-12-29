use manager_tools::types::filesystem::*;
use manager_tools::types::user_interaction::*;
use manager_tools::types::{BashRequest, GrepRequest};
use nocodo_llm_sdk::tools::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// LLM-facing schema for Sqlite3Reader mode (excludes db_path since it's auto-injected)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "mode")]
enum SqliteModeLlm {
    /// Execute arbitrary SQL queries (SELECT or PRAGMA)
    #[serde(rename = "query")]
    Query {
        #[schemars(
            description = "SQL query to execute. Only SELECT queries and PRAGMA statements are allowed."
        )]
        query: String,
    },

    /// Reflect/introspect database schema
    #[serde(rename = "reflect")]
    Reflect {
        #[schemars(
            description = "Target of reflection: tables (list all tables), schema (full schema dump), table_info (column info for a table), indexes (list indexes), views (list views), foreign_keys (foreign key relationships), stats (database statistics)"
        )]
        target: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[schemars(description = "Optional: specific table name for table_info and foreign_keys modes")]
        table_name: Option<String>,
    },
}

/// LLM-facing schema for Sqlite3Reader tool (excludes db_path since it's auto-injected)
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct Sqlite3ReaderRequestLlm {
    #[schemars(description = "Execution mode: either query or reflect")]
    mode: SqliteModeLlm,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Maximum number of rows to return. Defaults to 100, maximum 1000.")]
    limit: Option<usize>,
}

/// Create tool definitions for LLM using manager-models types
pub fn create_tool_definitions() -> Vec<Tool> {
    vec![
        Tool::from_type::<ListFilesRequest>()
            .name("list_files")
            .description("List files and directories in a given path")
            .build(),
        Tool::from_type::<ReadFileRequest>()
            .name("read_file")
            .description("Read the contents of a file")
            .build(),
        Tool::from_type::<WriteFileRequest>()
            .name("write_file")
            .description("Write or modify a file")
            .build(),
        Tool::from_type::<GrepRequest>()
            .name("grep")
            .description("Search for patterns in files using grep")
            .build(),
        Tool::from_type::<ApplyPatchRequest>()
            .name("apply_patch")
            .description("Apply a patch to create, modify, delete, or move multiple files")
            .build(),
        Tool::from_type::<BashRequest>()
            .name("bash")
            .description("Execute bash commands with timeout and permission checking")
            .build(),
        Tool::from_type::<AskUserRequest>()
            .name("ask_user")
            .description(
                "Ask the user a list of questions to gather information or confirm actions",
            )
            .build(),
        Tool::from_type::<Sqlite3ReaderRequestLlm>()
            .name("sqlite3_reader")
            .description("Read-only SQLite database tool with two modes: 1) query mode - execute SELECT/PRAGMA statements, 2) reflect mode - introspect schema (tables, indexes, views, foreign keys, stats). The database path is pre-configured.")
            .build(),
    ]
}

/// Get tool definition by name
pub fn get_tool_definition(tool_name: &str) -> Option<Tool> {
    create_tool_definitions()
        .into_iter()
        .find(|tool| tool.name() == tool_name)
}
