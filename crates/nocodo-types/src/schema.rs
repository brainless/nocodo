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
    /// Fractional number that is not money (scores, weights, percentages).
    Decimal,
}

/// A column every table has automatically. Never listed in a `Table`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImplicitColumn {
    pub name: &'static str,
    pub column_type: ColumnType,
    /// True for the table's primary key (`id`).
    pub primary_key: bool,
    /// Plain language: what the column holds.
    pub description: &'static str,
}

/// The single definition of the implicit columns, in table order. sqlgen,
/// the checker and the Schema agent's preamble all read this list.
pub const IMPLICIT_COLUMNS: [ImplicitColumn; 3] = [
    ImplicitColumn {
        name: "id",
        column_type: ColumnType::Integer,
        primary_key: true,
        description: "Identifies the entry. Other tables point at an entry by this number.",
    },
    ImplicitColumn {
        name: "created_at",
        column_type: ColumnType::DateTime,
        primary_key: false,
        description: "When the entry was added.",
    },
    ImplicitColumn {
        name: "updated_at",
        column_type: ColumnType::DateTime,
        primary_key: false,
        description: "When the entry was last changed.",
    },
];

/// True when `name` is one of the implicit columns.
pub fn is_implicit_column(name: &str) -> bool {
    IMPLICIT_COLUMNS.iter().any(|c| c.name == name)
}

/// The index name is derived by sqlgen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Index {
    pub columns: Vec<String>,
    pub unique: bool,
}
