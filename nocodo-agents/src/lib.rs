pub mod codebase_analysis;
pub mod database;
pub mod factory;
pub mod tools;

use async_trait::async_trait;
use manager_models::{ToolRequest, ToolResponse};
use manager_models::tools::filesystem::*;
use manager_models::tools::user_interaction::*;


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
        }
    }

    /// Convert AgentTool to nocodo-llm-sdk Tool definition for LLM
    pub fn to_tool_definition(&self) -> nocodo_llm_sdk::tools::Tool {
        // Use the schema generation from llm_schemas
        tools::llm_schemas::get_tool_definition(self.name())
            .expect("Tool definition must exist")
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
                let req: manager_models::GrepRequest = serde_json::from_value(arguments)?;
                ToolRequest::Grep(req)
            }
            "apply_patch" => {
                let req: ApplyPatchRequest = serde_json::from_value(arguments)?;
                ToolRequest::ApplyPatch(req)
            }
            "bash" => {
                let req: manager_models::BashRequest = serde_json::from_value(arguments)?;
                ToolRequest::Bash(req)
            }
            "ask_user" => {
                let req: AskUserRequest = serde_json::from_value(arguments)?;
                ToolRequest::AskUser(req)
            }
            _ => anyhow::bail!("Unknown tool: {}", name),
        };

        Ok(request)
    }
}

/// Format ToolResponse for display to LLM
pub fn format_tool_response(response: &manager_models::ToolResponse) -> String {
    match response {
        ToolResponse::ListFiles(r) => format!("Found {} files:\n{}", r.files.len(), r.files),
        ToolResponse::ReadFile(r) => format!("File contents ({} bytes):\n{}", r.content.len(), r.content),
        ToolResponse::WriteFile(r) => format!("Wrote {} bytes to {}", r.bytes_written, r.path),
        ToolResponse::Grep(r) => format!("Found {} matches:\n{:#?}", r.matches.len(), r.matches),
        ToolResponse::ApplyPatch(r) => format!("Applied patch: {:?}", r),
        ToolResponse::Bash(r) => format!("Exit code: {}\nStdout:\n{}\nStderr:\n{}", r.exit_code, r.stdout, r.stderr),
        ToolResponse::AskUser(r) => format!("User response: {:?}", r.responses),
        ToolResponse::Error(e) => format!("Error: {}", e.message),
    }
}

/// Trait defining the structure and behavior of an AI agent
#[async_trait]
pub trait Agent: Send + Sync {
    /// Returns the agent's clear objective
    fn objective(&self) -> &str;

    /// Returns the system prompt for the agent
    fn system_prompt(&self) -> &str;

    /// Returns optional pre-conditions that must be met before the agent can start
    /// Pre-conditions will be checked by an executor
    fn pre_conditions(&self) -> Option<Vec<String>> {
        None
    }

    /// Returns the list of tools available to this agent
    fn tools(&self) -> Vec<AgentTool>;

    /// Execute the agent with the given user prompt
    /// Optional method with default implementation that returns an error
    async fn execute(&self, _user_prompt: &str) -> anyhow::Result<String> {
        anyhow::bail!("Execute method not implemented for this agent")
    }
}
