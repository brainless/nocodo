use nocodo_llm_sdk::tools::Tool;
use nocodo_tools::types::filesystem::*;
use nocodo_tools::types::{BashRequest, GrepRequest};
use shared_types::user_interaction::*;

/// Create tool definitions for LLM using manager-models types
pub fn create_tool_definitions() -> Vec<Tool> {
    let sqlite_schema = serde_json::json!({
        "type": "object",
        "required": ["query"],
        "properties": {
            "query": {
                "type": "string",
                "description": "SQL query to execute. Use SELECT to retrieve data, or PRAGMA statements to inspect database schema. PRAGMA commands include: table_list (list tables), table_info(table_name) (column info), index_list(table_name) (indexes), foreign_key_list(table_name) (foreign keys)."
            },
            "limit": {"type": "integer", "description": "Maximum number of rows to return. Defaults to 100, maximum 1000."}
        }
    });

    let sqlite_tool = Tool::from_json_schema(
        "sqlite3_reader".to_string(),
        "Read-only SQLite database tool. Use SELECT queries to retrieve data and PRAGMA statements to inspect database schema (tables, columns, indexes, foreign keys). The database path is pre-configured.".to_string(),
        sqlite_schema,
    ).expect("Failed to create sqlite3_reader tool schema");

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
        sqlite_tool,
    ]
}

/// Get tool definition by name
pub fn get_tool_definition(tool_name: &str) -> Option<Tool> {
    create_tool_definitions()
        .into_iter()
        .find(|tool| tool.name() == tool_name)
}
