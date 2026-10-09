//! Shared naming and type mapping, also used by the checker's introspection.

use nocodo_types::schema::{ColumnType, Table};

/// SQLite storage type for a column type.
pub fn column_sql_type(t: ColumnType) -> &'static str {
    match t {
        ColumnType::Text | ColumnType::Date | ColumnType::DateTime => "TEXT",
        ColumnType::Integer | ColumnType::Money | ColumnType::Boolean => "INTEGER",
        ColumnType::Decimal => "REAL",
    }
}

/// Derived index name: `idx_<table>_<col1>_<col2>` or `uidx_…`.
pub fn index_name(table: &str, columns: &[String], unique: bool) -> String {
    let prefix = if unique { "uidx" } else { "idx" };
    format!("{prefix}_{table}_{}", columns.join("_"))
}

/// An index as it exists in SQLite, with its derived name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexDef {
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
}

/// Every index of a table: first one unique index per `unique` column
/// (in column order), then the explicit indexes (in creation order).
pub fn table_indexes(table: &Table) -> Vec<IndexDef> {
    let from_columns = table
        .columns
        .iter()
        .filter(|c| c.unique)
        .map(|c| (vec![c.name.clone()], true));
    let explicit = table.indexes.iter().map(|i| (i.columns.clone(), i.unique));
    from_columns
        .chain(explicit)
        .map(|(columns, unique)| IndexDef {
            name: index_name(&table.name, &columns, unique),
            columns,
            unique,
        })
        .collect()
}
