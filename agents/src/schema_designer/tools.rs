use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// SQLite column data types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "UPPERCASE")]
pub enum SqliteDataType {
    Integer,
    Text,
    Real,
    Blob,
    Numeric,
}

// ---------------------------------------------------------------------------
// Schema shape
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ForeignKey {
    /// Name of the referenced table.
    pub table: String,
    /// Name of the referenced column (usually "id").
    pub column: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ColumnDef {
    pub name: String,
    pub data_type: SqliteDataType,
    #[serde(default)]
    pub nullable: bool,
    #[serde(default)]
    pub primary_key: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreign_key: Option<ForeignKey>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TableDef {
    pub name: String,
    pub columns: Vec<ColumnDef>,
}

// ---------------------------------------------------------------------------
// Tool parameter types
// ---------------------------------------------------------------------------

/// Argument type for the `generate_schema` tool.
///
/// The model calls this tool to emit a normalized SQLite schema
/// based on the user's requirements.  Each call produces a new
/// versioned snapshot stored in `project_schema`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GenerateSchemaParams {
    /// Normalized set of tables that make up the schema.
    pub tables: Vec<TableDef>,
    /// Optional notes about design decisions or trade-offs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// Argument type for the `stop_agent` tool.
///
/// The model calls this when the user's request cannot be expressed
/// as a relational database schema (e.g. it is a prose question, a
/// math problem, or anything outside the schema-design domain).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StopAgentParams {
    /// Human-readable reply to show the user explaining why no schema was produced.
    pub reply: String,
}
