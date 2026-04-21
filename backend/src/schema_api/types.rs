use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ListSchemasQuery {
    pub project_id: i64,
}

#[derive(Debug, Deserialize)]
pub struct GetTableDataQuery {
    /// Comma-separated list of table IDs
    pub table_ids: String,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
