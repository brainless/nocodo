use serde::{Deserialize, Serialize};
use shared_types::SchemaDef;
pub use shared_types::{EpicItem, ListEpicsResponse, ListTasksResponse, TaskItem};

#[derive(Debug, Deserialize)]
pub struct ListTasksQuery {
    pub project_id: i64,
}

#[derive(Debug, Deserialize)]
pub struct EpicListQuery {
    pub project_id: i64,
}

#[derive(Debug, Deserialize)]
pub struct SchemaPreviewQuery {
    pub version: Option<i64>,
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

#[derive(Debug, Deserialize)]
pub struct BoardQuery {
    pub project_id: i64,
    pub since: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct BoardResponse {
    pub tasks: Vec<TaskItem>,
    pub epics: Vec<EpicItem>,
    pub updated_at: i64,
    pub project_name: String,
}
