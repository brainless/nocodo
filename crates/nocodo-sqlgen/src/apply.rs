//! `apply`: fold schema changes into a spec. Pure.

use std::fmt;

use nocodo_types::change::SchemaChange;
use nocodo_types::schema::{Index, SchemaSpec, Table};

use crate::naming::{index_name, table_indexes};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyError {
    UnknownTable { table: String },
    UnknownColumn { table: String, column: String },
    DuplicateTable { table: String },
    DuplicateColumn { table: String, column: String },
    DuplicateIndex { table: String, index: String },
}

impl fmt::Display for ApplyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownTable { table } => write!(f, "table `{table}` does not exist"),
            Self::UnknownColumn { table, column } => {
                write!(f, "column `{column}` does not exist in table `{table}`")
            }
            Self::DuplicateTable { table } => write!(f, "table `{table}` already exists"),
            Self::DuplicateColumn { table, column } => {
                write!(f, "column `{column}` already exists in table `{table}`")
            }
            Self::DuplicateIndex { table, index } => {
                write!(f, "index `{index}` already exists on table `{table}`")
            }
        }
    }
}

impl std::error::Error for ApplyError {}

/// Apply `changes` in order to a copy of `spec`.
pub fn apply(spec: &SchemaSpec, changes: &[SchemaChange]) -> Result<SchemaSpec, ApplyError> {
    let mut out = spec.clone();
    for change in changes {
        apply_in_place(&mut out, change)?;
    }
    Ok(out)
}

/// Apply a single change to a copy of `spec`.
pub fn apply_one(spec: &SchemaSpec, change: &SchemaChange) -> Result<SchemaSpec, ApplyError> {
    apply(spec, std::slice::from_ref(change))
}

fn table_pos(spec: &SchemaSpec, name: &str) -> Result<usize, ApplyError> {
    spec.tables
        .iter()
        .position(|t| t.name == name)
        .ok_or_else(|| ApplyError::UnknownTable {
            table: name.to_string(),
        })
}

fn column_pos(table: &Table, name: &str) -> Result<usize, ApplyError> {
    table
        .columns
        .iter()
        .position(|c| c.name == name)
        .ok_or_else(|| ApplyError::UnknownColumn {
            table: table.name.clone(),
            column: name.to_string(),
        })
}

fn apply_in_place(spec: &mut SchemaSpec, change: &SchemaChange) -> Result<(), ApplyError> {
    match change {
        SchemaChange::CreateTable {
            table,
            description,
            columns,
        } => {
            if spec.tables.iter().any(|t| &t.name == table) {
                return Err(ApplyError::DuplicateTable {
                    table: table.clone(),
                });
            }
            for (i, c) in columns.iter().enumerate() {
                if columns[..i].iter().any(|p| p.name == c.name) {
                    return Err(ApplyError::DuplicateColumn {
                        table: table.clone(),
                        column: c.name.clone(),
                    });
                }
                if let Some(r) = &c.references
                    && r != table
                    && !spec.tables.iter().any(|t| &t.name == r)
                {
                    return Err(ApplyError::UnknownTable { table: r.clone() });
                }
            }
            spec.tables.push(Table {
                name: table.clone(),
                description: description.clone(),
                columns: columns.clone(),
                indexes: Vec::new(),
            });
        }
        SchemaChange::AddColumn { table, column } => {
            if let Some(r) = &column.references
                && r != table
                && !spec.tables.iter().any(|t| &t.name == r)
            {
                return Err(ApplyError::UnknownTable { table: r.clone() });
            }
            let i = table_pos(spec, table)?;
            let t = &mut spec.tables[i];
            if t.columns.iter().any(|c| c.name == column.name) {
                return Err(ApplyError::DuplicateColumn {
                    table: table.clone(),
                    column: column.name.clone(),
                });
            }
            t.columns.push(column.clone());
        }
        SchemaChange::AddIndex {
            table,
            columns,
            unique,
        } => {
            let i = table_pos(spec, table)?;
            let t = &mut spec.tables[i];
            for c in columns {
                column_pos(t, c)?;
            }
            let name = index_name(table, columns, *unique);
            if table_indexes(t).iter().any(|d| d.name == name) {
                return Err(ApplyError::DuplicateIndex {
                    table: table.clone(),
                    index: name,
                });
            }
            t.indexes.push(Index {
                columns: columns.clone(),
                unique: *unique,
            });
        }
        SchemaChange::AddForeignKey {
            table,
            column,
            references,
        } => {
            table_pos(spec, references)?;
            let i = table_pos(spec, table)?;
            let c = column_pos(&spec.tables[i], column)?;
            spec.tables[i].columns[c].references = Some(references.clone());
        }
        SchemaChange::RenameTable { from, to } => {
            let i = table_pos(spec, from)?;
            if spec.tables.iter().any(|t| &t.name == to) {
                return Err(ApplyError::DuplicateTable { table: to.clone() });
            }
            spec.tables[i].name = to.clone();
            for c in spec.tables.iter_mut().flat_map(|t| t.columns.iter_mut()) {
                if c.references.as_deref() == Some(from.as_str()) {
                    c.references = Some(to.clone());
                }
            }
        }
        SchemaChange::RenameColumn { table, from, to } => {
            let i = table_pos(spec, table)?;
            let t = &mut spec.tables[i];
            let c = column_pos(t, from)?;
            if t.columns.iter().any(|c| &c.name == to) {
                return Err(ApplyError::DuplicateColumn {
                    table: table.clone(),
                    column: to.clone(),
                });
            }
            t.columns[c].name = to.clone();
            for idx in &mut t.indexes {
                for col in &mut idx.columns {
                    if col == from {
                        *col = to.clone();
                    }
                }
            }
        }
        SchemaChange::DropColumn { table, column } => {
            let i = table_pos(spec, table)?;
            let t = &mut spec.tables[i];
            let c = column_pos(t, column)?;
            t.columns.remove(c);
            // Indexes that used the column go away with it.
            t.indexes.retain(|idx| !idx.columns.contains(column));
        }
        SchemaChange::DropTable { table } => {
            let i = table_pos(spec, table)?;
            spec.tables.remove(i);
        }
    }
    Ok(())
}
