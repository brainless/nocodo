pub mod codebase_analysis;
pub mod config;
pub mod database;
pub mod factory;
pub mod sqlite_analysis;
pub mod structured_json;
pub mod tesseract;
pub mod tools;
pub mod user_clarification;

use async_trait::async_trait;
use manager_tools::types::filesystem::*;
use manager_tools::types::{ToolRequest, ToolResponse};
use shared_types::user_interaction::*;

/// Represents the types of tools available to agents
#[derive(Debug, Clone, PartialEq)]
pub enum AgentTool {
    ListFiles,
    ReadFile,
    WriteFile,
    Grep,
    ApplyPatch,
    Bash,
    AskUser,
    Sqlite3Reader,
}

impl AgentTool {
    /// Returns the tool name as used in ToolRequest
    pub fn name(&self) -> &'static str {
        match self {
            AgentTool::ListFiles => "list_files",
            AgentTool::ReadFile => "read_file",
            AgentTool::WriteFile => "write_file",
            AgentTool::Grep => "grep",
            AgentTool::ApplyPatch => "apply_patch",
            AgentTool::Bash => "bash",
            AgentTool::AskUser => "ask_user",
            AgentTool::Sqlite3Reader => "sqlite3_reader",
        }
    }

    /// Convert AgentTool to nocodo-llm-sdk Tool definition for LLM
    pub fn to_tool_definition(&self) -> nocodo_llm_sdk::tools::Tool {
        // Use the schema generation from llm_schemas
        tools::llm_schemas::get_tool_definition(self.name()).expect("Tool definition must exist")
    }

    /// Parse LLM tool call into typed ToolRequest
    pub fn parse_tool_call(
        name: &str,
        arguments: serde_json::Value,
    ) -> anyhow::Result<ToolRequest> {
        let request = match name {
            "list_files" => {
                let req: ListFilesRequest = serde_json::from_value(arguments)?;
                ToolRequest::ListFiles(req)
            }
            "read_file" => {
                let req: ReadFileRequest = serde_json::from_value(arguments)?;
                ToolRequest::ReadFile(req)
            }
            "write_file" => {
                let req: WriteFileRequest = serde_json::from_value(arguments)?;
                ToolRequest::WriteFile(req)
            }
            "grep" => {
                let req: manager_tools::types::GrepRequest = serde_json::from_value(arguments)?;
                ToolRequest::Grep(req)
            }
            "apply_patch" => {
                let req: ApplyPatchRequest = serde_json::from_value(arguments)?;
                ToolRequest::ApplyPatch(req)
            }
            "bash" => {
                let req: manager_tools::types::BashRequest = serde_json::from_value(arguments)?;
                ToolRequest::Bash(req)
            }
            "ask_user" => {
                let req: AskUserRequest = serde_json::from_value(arguments)?;
                ToolRequest::AskUser(req)
            }
            "sqlite3_reader" => {
                let value: serde_json::Value = arguments;

                let query = value
                    .get("query")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'query' field in sqlite3_reader call"))?
                    .to_string();

                let limit = value
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as usize);

                ToolRequest::Sqlite3Reader(manager_tools::types::Sqlite3ReaderRequest {
                    db_path: String::new(),
                    mode: manager_tools::types::SqliteMode::Query { query },
                    limit,
                })
            }
            _ => anyhow::bail!("Unknown tool: {}", name),
        };

        Ok(request)
    }
}

/// Format ToolResponse for display to LLM
pub fn format_tool_response(response: &manager_tools::types::ToolResponse) -> String {
    match response {
        ToolResponse::ListFiles(r) => format!("Found {} files:\n{}", r.files.len(), r.files),
        ToolResponse::ReadFile(r) => {
            format!("File contents ({} bytes):\n{}", r.content.len(), r.content)
        }
        ToolResponse::WriteFile(r) => format!("Wrote {} bytes to {}", r.bytes_written, r.path),
        ToolResponse::Grep(r) => format!("Found {} matches:\n{:#?}", r.matches.len(), r.matches),
        ToolResponse::ApplyPatch(r) => format!("Applied patch: {:?}", r),
        ToolResponse::Bash(r) => format!(
            "Exit code: {}\nStdout:\n{}\nStderr:\n{}",
            r.exit_code, r.stdout, r.stderr
        ),
        ToolResponse::AskUser(r) => format!("User response: {:?}", r.responses),
        ToolResponse::Sqlite3Reader(r) => r.formatted_output.clone(),
        ToolResponse::HackerNewsResponse(r) => r.message.clone(),
        ToolResponse::Error(e) => format!("Error: {}", e.message),
    }
}

/// Trait defining the structure and behavior of an AI agent
#[async_trait]
pub trait Agent: Send + Sync {
    /// Returns the agent's clear objective
    fn objective(&self) -> &str;

    /// Returns the system prompt for the agent
    fn system_prompt(&self) -> String;

    /// Returns optional pre-conditions that must be met before the agent can start
    /// Pre-conditions will be checked by an executor
    fn pre_conditions(&self) -> Option<Vec<String>> {
        None
    }

    /// Returns the list of tools available to this agent
    fn tools(&self) -> Vec<AgentTool>;

    /// Execute the agent with the given user prompt and session ID
    /// Optional method with default implementation that returns an error
    async fn execute(&self, _user_prompt: &str, _session_id: i64) -> anyhow::Result<String> {
        anyhow::bail!("Execute method not implemented for this agent")
    }
}
