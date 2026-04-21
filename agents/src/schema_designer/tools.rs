use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// Re-export the canonical definition types from shared-types so agent code
// can import from one place.
pub use shared_types::{ColumnDef, DataType, ForeignKeyDef, SchemaDef, TableDef};

/// Argument type for the `stop_agent` tool.
///
/// The model calls this when the user's request cannot be expressed as a
/// relational database schema (e.g. a prose question or out-of-domain task).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StopAgentParams {
    /// Human-readable reply explaining why no schema was produced.
    pub reply: String,
}
