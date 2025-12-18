use crate::{Agent, AgentTool};

#[cfg(test)]
mod tests;

/// Agent specialized in analyzing codebase structure and identifying architectural patterns
pub struct CodebaseAnalysisAgent;

impl Agent for CodebaseAnalysisAgent {
    fn objective(&self) -> &str {
        "Analyze codebase structure and identify architectural patterns"
    }

    fn system_prompt(&self) -> &str {
        "You are a codebase analysis expert. Your role is to examine code repositories, \
         understand their structure, identify architectural patterns, and provide clear insights \
         about the codebase organization. You should analyze file structures, dependencies, \
         design patterns, and architectural decisions."
    }

    fn tools(&self) -> Vec<AgentTool> {
        vec![AgentTool::ListFiles, AgentTool::ReadFile, AgentTool::Grep]
    }
}
