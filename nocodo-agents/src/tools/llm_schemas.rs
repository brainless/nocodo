use manager_models::tools::filesystem::*;
use manager_models::tools::user_interaction::*;
use manager_models::{GrepRequest, BashRequest};
use nocodo_llm_sdk::tools::{Tool};

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
            .description("Ask the user a list of questions to gather information or confirm actions")
            .build(),
    ]
}

/// Get tool definition by name
pub fn get_tool_definition(tool_name: &str) -> Option<Tool> {
    create_tool_definitions()
        .into_iter()
        .find(|tool| tool.name() == tool_name)
}