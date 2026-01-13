use super::*;
use crate::database::Database;
use manager_tools::ToolExecutor;
use nocodo_llm_sdk::client::LlmClient;
use nocodo_llm_sdk::error::LlmError;
use nocodo_llm_sdk::types::{CompletionRequest, CompletionResponse, ContentBlock, Role, Usage};
use std::path::PathBuf;
use std::sync::Arc;

struct MockLlmClient {
    response_content: String,
}

#[async_trait::async_trait]
impl LlmClient for MockLlmClient {
    async fn complete(&self, _request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        Ok(CompletionResponse {
            content: vec![ContentBlock::Text {
                text: self.response_content.clone(),
            }],
            role: Role::Assistant,
            usage: Usage {
                input_tokens: 10,
                output_tokens: 20,
            },
            stop_reason: Some("end_turn".to_string()),
            tool_calls: None,
        })
    }

    fn provider_name(&self) -> &str {
        "mock"
    }

    fn model_name(&self) -> &str {
        "mock-model"
    }
}

fn setup_test_agent(
    response_content: &str,
    type_names: Vec<&str>,
) -> anyhow::Result<(StructuredJsonAgent, Arc<Database>)> {
    let client: Arc<dyn LlmClient> = Arc::new(MockLlmClient {
        response_content: response_content.to_string(),
    });

    let database = Arc::new(Database::new(&PathBuf::from(":memory:"))?);

    let tool_executor = Arc::new(
        ToolExecutor::new(std::env::current_dir().unwrap()).with_max_file_size(10 * 1024 * 1024),
    );

    let config = StructuredJsonAgentConfig {
        type_names: type_names.into_iter().map(String::from).collect(),
        domain_description: "Test domain".to_string(),
    };

    let agent = StructuredJsonAgent::new(client, database.clone(), tool_executor, config)?;

    Ok((agent, database))
}

#[tokio::test]
async fn test_agent_creation() {
    let client: Arc<dyn LlmClient> = Arc::new(MockLlmClient {
        response_content: r#"{"id": 1, "name": "Test"}"#.to_string(),
    });

    let database = Arc::new(Database::new(&PathBuf::from(":memory:")).unwrap());

    let tool_executor = Arc::new(
        ToolExecutor::new(std::env::current_dir().unwrap()).with_max_file_size(10 * 1024 * 1024),
    );

    let config = StructuredJsonAgentConfig {
        type_names: vec!["PMProject".to_string()],
        domain_description: "Test domain".to_string(),
    };

    let agent = StructuredJsonAgent::new(client, database.clone(), tool_executor, config).unwrap();

    assert_eq!(
        agent.objective(),
        "Generate structured JSON responses conforming to specified TypeScript types"
    );

    let system_prompt = agent.system_prompt();
    assert!(system_prompt.contains("structured JSON"));
    assert!(system_prompt.contains("Test domain"));
}

#[tokio::test]
async fn test_agent_valid_json_response() {
    let (agent, database) = setup_test_agent(
        r#"{"id": 1, "name": "Test Project", "description": "A test project", "created_at": 1234567890}"#,
        vec!["PMProject"],
    )
    .unwrap();

    let session_id = database
        .create_session("structured-json", "test", "test", None, "test", None)
        .unwrap();

    let result = agent
        .execute("Create a test project", session_id)
        .await
        .unwrap();

    let json: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(json["id"], 1);
    assert_eq!(json["name"], "Test Project");
}

#[tokio::test]
async fn test_agent_invalid_json_syntax() {
    let (agent, database) = setup_test_agent(r#"{"id": 1, "name": }"#, vec!["PMProject"]).unwrap();

    let session_id = database
        .create_session("structured-json", "test", "test", None, "test", None)
        .unwrap();

    let result = agent.execute("Create a test project", session_id).await;

    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Failed to get valid JSON"));
}

#[tokio::test]
async fn test_agent_workflow_generation() {
    let valid_workflow_json = r#"{
        "workflow": {
            "id": 1,
            "project_id": 1,
            "name": "Test Workflow",
            "parent_workflow_id": null,
            "branch_condition": null,
            "created_at": 1234567890
        },
        "steps": [
            {
                "id": 1,
                "workflow_id": 1,
                "step_number": 1,
                "description": "First step",
                "created_at": 1234567890
            }
        ]
    }"#;

    let (agent, database) =
        setup_test_agent(valid_workflow_json, vec!["WorkflowWithSteps"]).unwrap();

    let session_id = database
        .create_session("structured-json", "test", "test", None, "test", None)
        .unwrap();

    let result = agent
        .execute("Create a workflow with steps", session_id)
        .await
        .unwrap();

    let json: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(json.get("workflow").is_some());
    assert!(json.get("steps").is_some());
}

#[tokio::test]
async fn test_agent_multiple_types() {
    let (agent, database) = setup_test_agent(
        r#"{"id": 1, "name": "Test Workflow", "project_id": 1, "parent_workflow_id": null, "branch_condition": null, "created_at": 1234567890}"#,
        vec!["PMProject", "Workflow"],
    )
    .unwrap();

    let session_id = database
        .create_session("structured-json", "test", "test", None, "test", None)
        .unwrap();

    let result = agent
        .execute("Create a workflow", session_id)
        .await
        .unwrap();

    let json: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(json["id"], 1);
    assert_eq!(json["name"], "Test Workflow");
}

#[test]
fn test_structured_json_agent_config() {
    let config = StructuredJsonAgentConfig {
        type_names: vec!["PMProject".to_string(), "Workflow".to_string()],
        domain_description: "Project management".to_string(),
    };

    assert_eq!(config.type_names.len(), 2);
    assert_eq!(config.type_names[0], "PMProject");
    assert_eq!(config.domain_description, "Project management");
}

#[tokio::test]
async fn test_agent_system_prompt_includes_type_definitions() {
    let (agent, _) = setup_test_agent(
        r#"{"id": 1, "name": "Test"}"#,
        vec!["PMProject", "Workflow"],
    )
    .unwrap();

    let system_prompt = agent.system_prompt();

    assert!(system_prompt.contains("TypeScript"));
    assert!(system_prompt.contains("PMProject"));
    assert!(system_prompt.contains("Workflow"));
    assert!(system_prompt.contains("IMPORTANT RULES"));
}

#[tokio::test]
async fn test_agent_no_tools() {
    let (agent, _) = setup_test_agent(r#"{"id": 1}"#, vec!["PMProject"]).unwrap();

    let tools = agent.tools();
    assert!(tools.is_empty());
}
