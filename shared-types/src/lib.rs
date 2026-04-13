use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

// ============================================================================
// Core Project Types
// ============================================================================

/// A Project is a container for related sheets and agent chat sessions.
/// It represents a workspace with its own data storage path.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Project {
    #[ts(type = "number")]
    pub id: i64,
    pub name: String,
    /// Path to folder where project data is stored
    pub path: String,
    #[ts(type = "number")]
    pub created_at: i64,
}

// ============================================================================
// Core Sheet Types
// ============================================================================

/// A Sheet is a collection of related tabs (like a database/schema)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Sheet {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub project_id: i64,
    pub name: String,
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number")]
    pub updated_at: i64,
}

/// A SheetTab is a tab within a sheet (like a table/spreadsheet page)
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SheetTab {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub sheet_id: i64,
    pub name: String,
    pub display_order: i32,
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number")]
    pub updated_at: i64,
}

/// Column data types for sheet tabs
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[ts(export)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum ColumnType {
    Text,
    Number,
    Integer,
    Boolean,
    Date,
    DateTime,
    Currency,
    /// Relations shown as clickable links in UI
    #[serde(rename = "relation")]
    Relation {
        #[ts(type = "number")]
        target_sheet_tab_id: i64,
        display_column: String,
    },
    /// Lookup pulls value from related table
    #[serde(rename = "lookup")]
    Lookup {
        relation_column: String,
        lookup_column: String,
    },
    /// Formula with expression
    #[serde(rename = "formula")]
    Formula {
        expression: String,
    },
}

/// Column definition (schema) for a sheet tab
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SheetTabColumn {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub sheet_tab_id: i64,
    pub name: String,
    pub column_type: ColumnType,
    pub is_required: bool,
    pub is_unique: bool,
    pub default_value: Option<String>,
    pub display_order: i32,
    #[ts(type = "number")]
    pub created_at: i64,
    /// Column width in pixels (user-resizable), default 120
    #[ts(type = "number")]
    pub width: i32,
}

// ============================================================================
// Read-Only Schema API Types
// ============================================================================

/// List all sheets in a project
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ListSheetsRequest {
    #[ts(type = "number")]
    pub project_id: i64,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ListSheetsResponse {
    pub sheets: Vec<Sheet>,
}

/// Get a single sheet with its tabs
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GetSheetRequest {
    #[ts(type = "number")]
    pub sheet_id: i64,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GetSheetResponse {
    pub sheet: Sheet,
    pub sheet_tabs: Vec<SheetTab>,
}

/// Get a sheet tab's schema (columns)
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GetSheetTabSchemaRequest {
    #[ts(type = "number")]
    pub sheet_tab_id: i64,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GetSheetTabSchemaResponse {
    pub sheet_tab: SheetTab,
    pub columns: Vec<SheetTabColumn>,
}

// ============================================================================
// Dynamic Data API Types
// ============================================================================

/// Request data for one or more sheet tabs
/// Returns positional row data (not column-id keyed) for flexible querying
#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GetSheetDataRequest {
    /// Sheet tab IDs to query (supports multi-table in future)
    #[ts(type = "number[]")]
    pub sheet_tab_ids: Vec<i64>,
    #[ts(type = "number | null")]
    pub limit: Option<i64>,
    #[ts(type = "number | null")]
    pub offset: Option<i64>,
}

/// Pagination metadata
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct PaginationInfo {
    #[ts(type = "number")]
    pub total_count: i64,
    #[ts(type = "number")]
    pub limit: i64,
    #[ts(type = "number")]
    pub offset: i64,
    #[ts(type = "boolean")]
    pub has_more: bool,
}

/// Data result for a single sheet tab
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct SheetTabDataResult {
    #[ts(type = "number")]
    pub sheet_tab_id: i64,
    /// Column definitions (same order as row data)
    pub columns: Vec<SheetTabColumn>,
    /// Rows as positional arrays (not keyed by column_id)
    /// Each inner array matches the order of `columns`
    /// TypeScript: unknown[][] (any JSON value)
    #[ts(type = "unknown[][]")]
    pub rows: Vec<Vec<Value>>,
    pub pagination: PaginationInfo,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GetSheetDataResponse {
    /// Results for each requested sheet_tab_id
    pub results: Vec<SheetTabDataResult>,
}

// ============================================================================
// Legacy Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct HeartbeatResponse {
    pub status: String,
    pub service: String,
}
