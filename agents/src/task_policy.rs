use crate::storage::{AgentType, TaskStatus};

pub fn initial_state_for(agent_type: &AgentType) -> TaskStatus {
    match agent_type {
        AgentType::BackendEngineer | AgentType::FrontendEngineer => TaskStatus::NeedsTechnicalShaping,
        AgentType::DbEngineer | AgentType::UiDesigner => TaskStatus::Ready,
        _ => TaskStatus::Ready,
    }
}
