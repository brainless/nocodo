// Sheets API types - re-export from shared_types where possible
use serde::Deserialize;

// Backend-specific request/response wrappers if needed
// Most types come from shared_types crate

#[derive(Debug, Deserialize)]
pub struct ListSheetsQuery {
    pub project_id: i64,
}

#[derive(Debug, Deserialize)]
pub struct GetSheetTabDataQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}
