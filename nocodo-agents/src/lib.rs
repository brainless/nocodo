pub mod codebase_analysis;

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
}

/// Trait defining the structure and behavior of an AI agent
pub trait Agent {
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
}
