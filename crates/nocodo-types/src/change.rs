//! Change types: what the Schema agent is allowed to say.
//!
//! Types only. Names and references are validated by the checker, not here.

use serde::{Deserialize, Serialize};

use crate::schema::Column;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum SchemaChange {
    CreateTable {
        table: String,
        description: String,
        columns: Vec<Column>,
    },
    AddColumn {
        table: String,
        column: Column,
    },
    AddIndex {
        table: String,
        columns: Vec<String>,
        unique: bool,
    },
    AddForeignKey {
        table: String,
        column: String,
        references: String,
    },
    RenameTable {
        from: String,
        to: String,
    },
    RenameColumn {
        table: String,
        from: String,
        to: String,
    },
    DropColumn {
        table: String,
        column: String,
    },
    DropTable {
        table: String,
    },
}

impl SchemaChange {
    /// True for changes that remove user data.
    pub fn is_destructive(&self) -> bool {
        matches!(self, Self::DropColumn { .. } | Self::DropTable { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangeSet {
    /// snake_case; becomes the migration folder suffix.
    pub name: String,
    /// One or two plain-language sentences.
    pub summary: String,
    pub changes: Vec<SchemaChange>,
}

impl ChangeSet {
    /// True if any change in the set is destructive.
    pub fn is_destructive(&self) -> bool {
        self.changes.iter().any(SchemaChange::is_destructive)
    }
}
