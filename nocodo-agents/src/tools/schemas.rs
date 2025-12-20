use nocodo_llm_sdk::tools::Tool;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// List files tool parameters
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ListFilesParams {
    /// Glob pattern to match files (e.g., "**/*.rs")
    pub pattern: String,
    /// Directory to search in (relative to base path)
    pub path: Option<String>,
}

/// Read file tool parameters
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ReadFileParams {
    /// Path to the file to read
    pub file_path: String,
    /// Optional line offset to start reading from
    pub offset: Option<usize>,
    /// Optional number of lines to read
    pub limit: Option<usize>,
}

/// Grep tool parameters
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GrepParams {
    /// Regular expression pattern to search for
    pub pattern: String,
    /// File type filter (e.g., "rs", "toml")
    #[serde(rename = "type")]
    pub file_type: Option<String>,
    /// Glob pattern to filter files
    pub glob: Option<String>,
    /// Case insensitive search
    #[serde(rename = "-i")]
    pub case_insensitive: Option<bool>,
}

/// Write file tool parameters
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct WriteFileParams {
    /// Path to the file to write
    pub file_path: String,
    /// Content to write to the file
    pub content: String,
}

/// Edit file tool parameters
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct EditFileParams {
    /// Path to the file to edit
    pub file_path: String,
    /// Old text to replace
    pub old_string: String,
    /// New text to replace with
    pub new_string: String,
    /// Replace all occurrences
    pub replace_all: Option<bool>,
}

/// Bash tool parameters
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct BashParams {
    /// Command to execute
    pub command: String,
    /// Working directory for command execution
    pub workdir: Option<String>,
    /// Timeout in milliseconds
    pub timeout: Option<u64>,
}

pub fn create_tool_definitions() -> Vec<Tool> {
    vec![
        Tool::from_type::<ListFilesParams>()
            .name("list_files")
            .description("List files matching a glob pattern in codebase")
            .build(),

        Tool::from_type::<ReadFileParams>()
            .name("read_file")
            .description("Read contents of a file")
            .build(),

        Tool::from_type::<GrepParams>()
            .name("grep")
            .description("Search for a pattern across files using ripgrep")
            .build(),

        Tool::from_type::<WriteFileParams>()
            .name("write_file")
            .description("Write content to a file")
            .build(),

        Tool::from_type::<EditFileParams>()
            .name("edit_file")
            .description("Replace text in a file")
            .build(),

        Tool::from_type::<BashParams>()
            .name("bash")
            .description("Execute a bash command")
            .build(),
    ]
}