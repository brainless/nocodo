use serde::{Deserialize, Serialize};
use shared_types::SchemaDef;
pub use shared_types::{EpicItem, ListEpicsResponse, ListTasksResponse, TaskItem};

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub project_id: i64,
    /// None = new task (creates task + session); Some = continue existing task
    pub task_id: Option<i64>,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub task_id: i64,
    pub message_id: i64,
    pub status: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum AgentResponsePayload {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "schema_generated")]
    SchemaGenerated {
        text: String,
        schema: SchemaDef,
        preview: bool,
    },
    #[serde(rename = "stopped")]
    Stopped { text: String },
    #[serde(rename = "question")]
    Question { text: String },
    #[serde(rename = "pending")]
    Pending,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub message_id: i64,
    pub response: AgentResponsePayload,
}

#[derive(Debug, Serialize)]
pub struct ChatHistoryMessage {
    pub id: i64,
    pub role: String,
    pub content: String,
    pub created_at: i64,
    pub schema_version: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct SchemaPreviewQuery {
    pub version: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ChatHistoryResponse {
    pub task_id: i64,
    pub messages: Vec<ChatHistoryMessage>,
}

#[derive(Debug, Deserialize)]
pub struct ListTasksQuery {
    pub project_id: i64,
}

#[derive(Debug, Deserialize)]
pub struct EpicListQuery {
    pub project_id: i64,
}

#[derive(Debug, Serialize)]
pub struct SchemaPreviewResponse {
    pub schema: SchemaDef,
    pub version: i64,
}

#[derive(Debug, Serialize)]
pub struct SchemaCodegenResponse {
    pub rust_code: String,
    pub sql_ddl: String,
}
