//! Schema spec types: the typed description of a project's tables.
//!
//! `id`, `created_at` and `updated_at` are implicit on every table and are
//! never listed. Index names are derived by sqlgen.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const SPEC_VERSION: u32 = 1;

/// The whole schema of a project. Tables are ordered by creation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaSpec {
    pub spec_version: u32,
    pub tables: Vec<Table>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Table {
    /// snake_case plural, e.g. "transactions".
    pub name: String,
    /// Plain language: why this table exists.
    pub description: String,
    /// User columns only; `id`, `created_at`, `updated_at` are implicit.
    pub columns: Vec<Column>,
    pub indexes: Vec<Index>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Column {
    pub name: String,
    pub description: String,
    pub column_type: ColumnType,
    #[serde(default)]
    pub nullable: bool,
    #[serde(default)]
    pub unique: bool,
    /// Table name; implies an Integer foreign key to that table's `id`.
    #[serde(default)]
    pub references: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColumnType {
    Text,
    Integer,
    Boolean,
    /// ISO-8601 date text.
    Date,
    /// ISO-8601 date-time text.
    DateTime,
    /// Integer minor units.
    Money,
}

/// The index name is derived by sqlgen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Index {
    pub columns: Vec<String>,
    pub unique: bool,
}
