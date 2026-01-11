use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Project entity for project management
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Project {
    pub id: i32,
    pub name: String,
    pub description: String,
    pub created_at: i64,
}

/// Workflow entity
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Workflow {
    pub id: i32,
    pub project_id: i32,
    pub name: String,
    pub parent_workflow_id: Option<i32>,
    pub branch_condition: Option<String>,
    pub created_at: i64,
}

/// Workflow step entity
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WorkflowStep {
    pub id: i32,
    pub workflow_id: i32,
    pub step_number: i32,
    pub description: String,
    pub created_at: i64,
}

/// Response containing a single workflow with its steps
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WorkflowWithSteps {
    pub workflow: Workflow,
    pub steps: Vec<WorkflowStep>,
}

/// Response for saving workflow
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SaveWorkflowRequest {
    pub workflow: Vec<WorkflowStepData>,
}

/// Workflow step data for saving
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct WorkflowStepData {
    pub id: i32,
    pub step_number: i32,
    pub description: String,
}
