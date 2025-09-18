use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Represents a GitHub Actions workflow file
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Workflow {
    pub name: Option<String>,
    pub on: Trigger,
    pub jobs: std::collections::HashMap<String, Job>,
}

/// Workflow trigger configuration
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(untagged)]
pub enum Trigger {
    Single(String),
    Multiple(Vec<String>),
    Detailed(TriggerConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TriggerConfig {
    pub push: Option<BranchConfig>,
    pub pull_request: Option<BranchConfig>,
    pub schedule: Option<Vec<CronSchedule>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct BranchConfig {
    pub branches: Option<Vec<String>>,
    pub branches_ignore: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CronSchedule {
    pub cron: String,
}

/// Represents a job in a workflow
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Job {
    #[serde(rename = "runs-on")]
    pub runs_on: Runner,
    pub steps: Vec<Step>,
    pub needs: Option<Vec<String>>,
    pub environment: Option<String>,
    #[serde(rename = "working-directory")]
    pub working_directory: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(untagged)]
pub enum Runner {
    Single(String),
    Multiple(Vec<String>),
}

/// Represents a step in a job
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Step {
    pub id: Option<String>,
    pub name: Option<String>,
    pub uses: Option<String>,
    pub run: Option<String>,
    pub shell: Option<String>,
    #[serde(rename = "working-directory")]
    pub working_directory: Option<String>,
    pub env: Option<std::collections::HashMap<String, String>>,
    #[ts(skip)]
    pub with: Option<std::collections::HashMap<String, serde_json::Value>>,
}

/// An extracted executable command from a workflow
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WorkflowCommand {
    pub id: String,
    pub workflow_name: String,
    pub job_name: String,
    pub step_name: Option<String>,
    pub command: String,
    pub shell: Option<String>,
    pub working_directory: Option<String>,
    pub environment: Option<std::collections::HashMap<String, String>>,
    pub file_path: String,
}

/// Result of executing a workflow command
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CommandExecution {
    pub command_id: String,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    #[ts(type = "string")]
    pub executed_at: chrono::DateTime<chrono::Utc>,
    pub success: bool,
}

/// Request to scan workflows in a project
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ScanWorkflowsRequest {
    pub project_id: String,
}

/// Response containing scanned workflows
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ScanWorkflowsResponse {
    pub workflows: Vec<WorkflowInfo>,
    pub commands: Vec<WorkflowCommand>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WorkflowInfo {
    pub name: String,
    pub file_path: String,
    pub jobs_count: usize,
    pub commands_count: usize,
}

/// Request to execute a workflow command
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ExecuteCommandRequest {
    pub command_id: String,
    pub timeout_seconds: Option<u64>,
}

/// Response from command execution
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ExecuteCommandResponse {
    pub execution: CommandExecution,
}