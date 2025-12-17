//! # Nocodo Workflow
//!
//! Workflow orchestration for building AI agents with nocodo.
//! This crate helps users define their agent requirements through interactive questioning,
//! then connects data sources (APIs, databases, files) to build functional agents.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Input requirement for an agent workflow
/// Used by the LLM to request API keys, URLs, database names, etc.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkflowInput {
    /// Name/identifier for this input
    pub name: String,
    /// Human-readable label describing what this input is for
    pub label: String,
}

/// Response structure for workflow definition conversations
/// The LLM must respond with this exact structure in JSON format
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkflowResponse {
    /// Questions the LLM wants to ask the user for clarification
    #[serde(default)]
    pub questions: Vec<String>,

    /// Input requirements (API keys, URLs, database names, etc.)
    #[serde(default)]
    pub inputs: Vec<WorkflowInput>,
}

impl WorkflowResponse {
    /// Create a new empty workflow response
    pub fn new() -> Self {
        Self {
            questions: Vec::new(),
            inputs: Vec::new(),
        }
    }

    /// Create a workflow response with questions
    pub fn with_questions(questions: Vec<String>) -> Self {
        Self {
            questions,
            inputs: Vec::new(),
        }
    }

    /// Create a workflow response with inputs
    pub fn with_inputs(inputs: Vec<WorkflowInput>) -> Self {
        Self {
            questions: Vec::new(),
            inputs,
        }
    }

    /// Get JSON schema for this type
    pub fn json_schema() -> serde_json::Value {
        let schema = schemars::schema_for!(WorkflowResponse);
        serde_json::to_value(&schema).expect("Failed to serialize schema")
    }
}

impl Default for WorkflowResponse {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_response_creation() {
        let response = WorkflowResponse::new();
        assert!(response.questions.is_empty());
        assert!(response.inputs.is_empty());
    }

    #[test]
    fn test_workflow_response_with_questions() {
        let questions = vec![
            "What data sources do you need?".to_string(),
            "How often should the agent run?".to_string(),
        ];
        let response = WorkflowResponse::with_questions(questions.clone());
        assert_eq!(response.questions, questions);
        assert!(response.inputs.is_empty());
    }

    #[test]
    fn test_workflow_response_with_inputs() {
        let inputs = vec![
            WorkflowInput {
                name: "api_key".to_string(),
                label: "API Key for service".to_string(),
            },
            WorkflowInput {
                name: "db_url".to_string(),
                label: "Database connection URL".to_string(),
            },
        ];
        let response = WorkflowResponse::with_inputs(inputs.clone());
        assert_eq!(response.inputs.len(), 2);
        assert!(response.questions.is_empty());
    }

    #[test]
    fn test_json_serialization() {
        let response = WorkflowResponse {
            questions: vec!["What is your use case?".to_string()],
            inputs: vec![WorkflowInput {
                name: "api_key".to_string(),
                label: "API Key".to_string(),
            }],
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: WorkflowResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.questions, response.questions);
        assert_eq!(deserialized.inputs.len(), response.inputs.len());
    }

    #[test]
    fn test_json_schema_generation() {
        let schema = WorkflowResponse::json_schema();
        assert!(schema.is_object());

        // Verify it's a valid JSON schema
        let schema_str = serde_json::to_string_pretty(&schema).unwrap();
        assert!(schema_str.contains("WorkflowResponse"));
    }
}
