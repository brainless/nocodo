use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
}

/// Returns a list of all supported agents
/// Currently only SQLite agent is enabled
pub fn list_supported_agents() -> Vec<AgentInfo> {
    vec![
        AgentInfo {
            id: "sqlite".to_string(),
            name: "SQLite Analysis Agent".to_string(),
            description: "Agent for analyzing SQLite databases and running SQL queries".to_string(),
            enabled: true,
        },
    ]
}
