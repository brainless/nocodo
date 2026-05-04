use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

// ============================================================================
// Core Project Types
// ============================================================================

/// A Project is a container for related schemas and agent chat sessions.
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
// Core Relational Types (persisted)
// ============================================================================

/// A Schema is a named collection of tables within a project.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Schema {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub project_id: i64,
    pub name: String,
    #[ts(type = "number")]
    pub created_at: i64,
}

/// A Table is a relational table within a schema.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Table {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub schema_id: i64,
    pub name: String,
    #[ts(type = "number")]
    pub created_at: i64,
}

/// Storage-level column data type.
#[derive(Debug, Clone, Serialize, Deserialize, TS, JsonSchema, PartialEq)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum DataType {
    Text,
    Integer,
    Real,
    Boolean,
    Date,
    DateTime,
}

/// A Column in a relational table.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct Column {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub table_id: i64,
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
    pub primary_key: bool,
    /// Defines column order in SELECT queries and UI display.
    pub display_order: i32,
    #[ts(type = "number")]
    pub created_at: i64,
}

/// A persisted foreign key constraint on a column.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ForeignKey {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub column_id: i64,
    /// SQL name of the referenced table.
    pub ref_table: String,
    /// Name of the referenced column (usually "id").
    pub ref_column: String,
}

/// UI display metadata for a column. Decoupled from the relational schema.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ColumnDisplay {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub column_id: i64,
    /// Column width in pixels (user-resizable), default 120.
    pub width: i32,
    /// For FK columns: which column of the referenced table to show as the link label.
    pub display_column: Option<String>,
}

// ============================================================================
// Agent Type Registry
// ============================================================================

/// Available agent types in the multi-agent system.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    ProjectManager,
    SchemaDesigner,
    BackendDeveloper,
    FrontendDeveloper,
}

// ============================================================================
// Agent Definition Types
// (pre-persistence; used as LLM tool parameters via JsonSchema)
// ============================================================================

/// Foreign key reference by name — resolved to IDs on persist.
#[derive(Debug, Clone, Serialize, Deserialize, TS, JsonSchema)]
#[ts(export)]
pub struct ForeignKeyDef {
    /// SQL name of the referenced table.
    #[serde(default)]
    pub ref_table: String,
    /// Name of the referenced column (usually "id").
    #[serde(default)]
    pub ref_column: String,
}

/// Column definition as emitted by the agent.
#[derive(Debug, Clone, Serialize, Deserialize, TS, JsonSchema)]
#[ts(export)]
pub struct ColumnDef {
    pub name: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(deserialize_with = "deserialize_data_type")]
    #[schemars(schema_with = "data_type_loose_schema")]
    pub data_type: DataType,
    #[serde(default)]
    pub nullable: bool,
    #[serde(default)]
    pub primary_key: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreign_key: Option<ForeignKeyDef>,
}

/// Loose schema for the tool: accept any string with a description of valid values.
/// Prevents provider-side enum validation from rejecting LLM aliases like "varchar".
fn data_type_loose_schema(_gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
    use schemars::schema::*;
    SchemaObject {
        instance_type: Some(InstanceType::String.into()),
        metadata: Some(Box::new(Metadata {
            description: Some(
                "Column storage type. Use exactly one of: text, integer, real, boolean, date, date_time".to_string(),
            ),
            ..Default::default()
        })),
        ..Default::default()
    }
    .into()
}

/// Lenient deserializer: maps SQL type aliases to canonical DataType variants.
fn deserialize_data_type<'de, D>(deserializer: D) -> Result<DataType, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(match s.to_lowercase().replace(['-', ' '], "_").as_str() {
        "text" | "varchar" | "string" | "char" | "nvarchar" | "uuid" | "json" | "blob" => DataType::Text,
        "integer" | "int" | "bigint" | "smallint" | "tinyint" | "int4" | "int8" | "number" => DataType::Integer,
        "real" | "float" | "double" | "decimal" | "numeric" | "float4" | "float8" => DataType::Real,
        "boolean" | "bool" | "bit" => DataType::Boolean,
        "date" => DataType::Date,
        "date_time" | "datetime" | "timestamp" | "timestamptz" | "time" => DataType::DateTime,
        _ => DataType::Text,
    })
}

/// Table definition as emitted by the agent.
#[derive(Debug, Clone, Serialize, Deserialize, TS, JsonSchema)]
#[ts(export)]
pub struct TableDef {
    pub name: String,
    #[serde(default)]
    pub label: Option<String>,
    pub columns: Vec<ColumnDef>,
}

/// Complete schema definition — the agent emits this via the `generate_schema` tool.
/// Each call produces a new versioned snapshot stored in `project_schema`.
#[derive(Debug, Clone, Serialize, Deserialize, TS, JsonSchema)]
#[ts(export)]
pub struct SchemaDef {
    /// Human-readable schema name.
    pub name: String,
    #[serde(default)]
    pub label: Option<String>,
    /// Normalized set of tables that make up the schema.
    pub tables: Vec<TableDef>,
}

// ============================================================================
// Schema API Response Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ListSchemasResponse {
    pub schemas: Vec<Schema>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GetSchemaResponse {
    pub schema: Schema,
    pub tables: Vec<Table>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GetTableColumnsResponse {
    pub table: Table,
    pub columns: Vec<Column>,
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
    pub has_more: bool,
}

/// Data result for a single table
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TableDataResult {
    #[ts(type = "number")]
    pub table_id: i64,
    /// Column definitions in display order
    pub columns: Vec<Column>,
    /// Rows as positional arrays matching the order of `columns`
    #[ts(type = "unknown[][]")]
    pub rows: Vec<Vec<Value>>,
    pub pagination: PaginationInfo,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct GetTableDataResponse {
    pub results: Vec<TableDataResult>,
}

// ============================================================================
// Project API Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CreateProjectRequest {
    pub name: String,
    pub path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CreateProjectResponse {
    pub project: Project,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ListProjectsResponse {
    pub projects: Vec<Project>,
}

// ============================================================================
// Misc Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct HeartbeatResponse {
    pub status: String,
    pub service: String,
}

// ============================================================================
// Agent Task/Epic API Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TaskItem {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub project_id: i64,
    #[ts(type = "number | null")]
    pub epic_id: Option<i64>,
    pub title: String,
    pub source_prompt: String,
    pub assigned_to_agent: String,
    pub status: String,
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number")]
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ListTasksResponse {
    pub tasks: Vec<TaskItem>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct EpicItem {
    #[ts(type = "number")]
    pub id: i64,
    #[ts(type = "number")]
    pub project_id: i64,
    pub title: String,
    pub description: String,
    pub status: String,
    pub created_by_agent: String,
    #[ts(type = "number | null")]
    pub created_by_task_id: Option<i64>,
    #[ts(type = "number")]
    pub created_at: i64,
    #[ts(type = "number")]
    pub updated_at: i64,
}

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct ListEpicsResponse {
    pub epics: Vec<EpicItem>,
}
