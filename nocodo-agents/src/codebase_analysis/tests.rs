use super::*;

#[test]
fn test_codebase_analysis_agent_objective() {
    let agent = CodebaseAnalysisAgent;
    assert_eq!(
        agent.objective(),
        "Analyze codebase structure and identify architectural patterns"
    );
}

#[test]
fn test_codebase_analysis_agent_has_required_tools() {
    let agent = CodebaseAnalysisAgent;
    let tools = agent.tools();

    assert!(tools.contains(&AgentTool::ListFiles));
    assert!(tools.contains(&AgentTool::ReadFile));
    assert!(tools.contains(&AgentTool::Grep));
}

#[test]
fn test_codebase_analysis_agent_system_prompt_not_empty() {
    let agent = CodebaseAnalysisAgent;
    assert!(!agent.system_prompt().is_empty());
}

#[test]
fn test_codebase_analysis_agent_no_preconditions() {
    let agent = CodebaseAnalysisAgent;
    assert!(agent.pre_conditions().is_none());
}
