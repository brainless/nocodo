//! Static rules: checks on a change set before any SQL exists.

use std::collections::BTreeSet;

use nocodo_sqlgen::{ApplyError, apply_one};
use nocodo_types::change::{ChangeSet, SchemaChange};
use nocodo_types::event::Finding;
use nocodo_types::naming::{expected_reference_name, is_reference_name};
use nocodo_types::schema::{Column, ColumnType, SchemaSpec, is_implicit_column};

use crate::keywords::is_keyword;

fn finding(code: &str, message: String) -> Finding {
    Finding {
        code: code.to_string(),
        message,
    }
}

fn name_problem(name: &str) -> Option<&'static str> {
    let mut chars = name.chars();
    let first_ok = chars.next().is_some_and(|c| c.is_ascii_lowercase());
    let rest_ok = chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    if !first_ok || !rest_ok || name.len() > 63 {
        Some(
            "must start with a lowercase letter and use only lowercase letters, digits and `_` (at most 63 characters)",
        )
    } else if name.starts_with("sqlite_") {
        Some("must not start with `sqlite_`, which SQLite reserves")
    } else if is_keyword(name) {
        Some("is an SQLite keyword; pick a more specific name")
    } else {
        None
    }
}

fn check_name(out: &mut Vec<Finding>, kind: &str, name: &str) {
    if let Some(problem) = name_problem(name) {
        out.push(finding(
            "invalid_name",
            format!("The {kind} name `{name}` {problem}."),
        ));
    }
}

fn check_column(out: &mut Vec<Finding>, table: &str, c: &Column) {
    check_name(out, "column", &c.name);
    if is_implicit_column(&c.name) {
        out.push(finding(
            "reserved_column",
            format!(
                "Column `{}` in `{table}` is added automatically to every table; remove it or choose another name.",
                c.name
            ),
        ));
    }
    if c.description.trim().is_empty() {
        out.push(finding(
            "missing_description",
            format!(
                "Column `{}` in `{table}` needs a plain-language description.",
                c.name
            ),
        ));
    }
}

fn check_reference(out: &mut Vec<Finding>, spec: &SchemaSpec, table: &str, c: &Column) {
    let Some(target) = &c.references else {
        if c.column_type == ColumnType::Integer && c.name.ends_with("_id") {
            out.push(finding(
                "unlinked_id",
                format!(
                    "Column `{}` in `{table}` is an integer ending in `_id` but has no `references`. Link each list it can point at with its own reference column (or a linking table). If the value is an outside code rather than a link, rename it (for example `_number`) and use type `text`.",
                    c.name
                ),
            ));
        }
        return;
    };
    if !is_reference_name(&c.name, target) {
        out.push(finding(
            "reference_name",
            format!(
                "Column `{}` in `{table}` references `{target}`, so it must be named `{exp}` or end with `_{exp}` (for example `paid_by_{exp}`).",
                c.name,
                exp = expected_reference_name(target)
            ),
        ));
    }
    // A table may reference itself, including while it is being created.
    if target != table && !spec.tables.iter().any(|t| &t.name == target) {
        out.push(finding(
            "bad_reference",
            format!(
                "Column `{}` in `{table}` references table `{target}`, which does not exist. Create `{target}` first or reference an existing table.",
                c.name
            ),
        ));
    }
    if c.column_type != ColumnType::Integer {
        out.push(finding(
            "bad_reference",
            format!(
                "Column `{}` in `{table}` references `{target}` but is not an integer column. A reference column must have type `integer`.",
                c.name
            ),
        ));
    }
}

fn apply_finding(e: &ApplyError) -> Finding {
    match e {
        ApplyError::UnknownTable { .. } => finding(
            "unknown_table",
            format!("{e}. Use an existing table name or create it first."),
        ),
        ApplyError::UnknownColumn { .. } => finding(
            "unknown_column",
            format!("{e}. Use an existing column name or add it first."),
        ),
        ApplyError::DuplicateTable { .. }
        | ApplyError::DuplicateColumn { .. }
        | ApplyError::DuplicateIndex { .. } => finding(
            "duplicate_name",
            format!("{e}. Choose a name that is not used yet."),
        ),
    }
}

/// Check `cs` against the `current` schema. Empty result means it passed.
pub fn check_change_set(
    current: &SchemaSpec,
    cs: &ChangeSet,
    confirm_destructive: bool,
) -> Vec<Finding> {
    let mut out = Vec::new();
    check_name(&mut out, "change set", &cs.name);
    if cs.changes.is_empty() {
        out.push(finding(
            "empty_change_set",
            "The change set has no changes; add at least one.".to_string(),
        ));
    }
    if cs.is_destructive() && !confirm_destructive {
        out.push(finding(
            "destructive_not_confirmed",
            "This change set removes data (drops a table or column). Ask the user to confirm it, then propose it again with confirmation.".to_string(),
        ));
    }

    let mut cur = current.clone();
    // Tables created by this change set hold no rows yet.
    let mut fresh: BTreeSet<String> = BTreeSet::new();
    let mut applying = true;
    for change in &cs.changes {
        check_one(&mut out, &cur, &fresh, change);
        if !applying {
            continue;
        }
        match apply_one(&cur, change) {
            Ok(next) => cur = next,
            Err(e) => {
                out.push(apply_finding(&e));
                applying = false;
                continue;
            }
        }
        match change {
            SchemaChange::CreateTable { table, .. } => {
                fresh.insert(table.clone());
            }
            SchemaChange::RenameTable { from, to } => {
                if fresh.remove(from) {
                    fresh.insert(to.clone());
                }
            }
            SchemaChange::DropTable { table } => {
                fresh.remove(table);
            }
            _ => {}
        }
    }
    out
}

fn check_one(
    out: &mut Vec<Finding>,
    cur: &SchemaSpec,
    fresh: &BTreeSet<String>,
    change: &SchemaChange,
) {
    match change {
        SchemaChange::CreateTable {
            table,
            description,
            columns,
        } => {
            check_name(out, "table", table);
            if description.trim().is_empty() {
                out.push(finding(
                    "missing_description",
                    format!("Table `{table}` needs a plain-language description."),
                ));
            }
            for c in columns {
                check_column(out, table, c);
                check_reference(out, cur, table, c);
            }
        }
        SchemaChange::AddColumn { table, column } => {
            check_column(out, table, column);
            check_reference(out, cur, table, column);
            let existing = cur.tables.iter().any(|t| &t.name == table) && !fresh.contains(table);
            if existing && !column.nullable {
                out.push(finding(
                    "added_column_not_nullable",
                    format!(
                        "Column `{}` is added to `{table}`, which may already hold rows, but is not nullable. Make it nullable.",
                        column.name
                    ),
                ));
            }
        }
        SchemaChange::AddForeignKey {
            table,
            column,
            references,
        } => {
            if !cur.tables.iter().any(|t| &t.name == references) {
                out.push(finding(
                    "bad_reference",
                    format!(
                        "Foreign key on `{table}.{column}` references table `{references}`, which does not exist. Create it first or reference an existing table."
                    ),
                ));
            }
            let col = cur
                .tables
                .iter()
                .find(|t| &t.name == table)
                .and_then(|t| t.columns.iter().find(|c| &c.name == column));
            if col.is_some_and(|c| c.column_type != ColumnType::Integer) {
                out.push(finding(
                    "bad_reference",
                    format!(
                        "Foreign key column `{table}.{column}` is not an integer column. Only integer columns can reference another table."
                    ),
                ));
            }
        }
        SchemaChange::RenameTable { to, .. } => check_name(out, "table", to),
        SchemaChange::RenameColumn { table, to, .. } => {
            check_name(out, "column", to);
            if is_implicit_column(to) {
                out.push(finding(
                    "reserved_column",
                    format!(
                        "Column `{to}` in `{table}` is added automatically to every table; choose another name."
                    ),
                ));
            }
        }
        SchemaChange::AddIndex { .. }
        | SchemaChange::DropColumn { .. }
        | SchemaChange::DropTable { .. } => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn col(name: &str, ty: ColumnType, references: Option<&str>) -> Column {
        Column {
            name: name.into(),
            description: "d".into(),
            column_type: ty,
            nullable: false,
            unique: false,
            references: references.map(Into::into),
        }
    }

    fn codes(cols: Vec<Column>) -> Vec<Finding> {
        let spec = SchemaSpec {
            spec_version: 1,
            tables: vec![],
        };
        let mut out = Vec::new();
        for c in &cols {
            check_reference(&mut out, &spec, "expenses", c);
        }
        out.retain(|f| f.code != "bad_reference");
        out
    }

    #[test]
    fn reference_name_rule() {
        for ok in [
            col("person_id", ColumnType::Integer, Some("people")),
            col("paid_by_person_id", ColumnType::Integer, Some("people")),
            col(
                "expense_category_id",
                ColumnType::Integer,
                Some("expense_categories"),
            ),
        ] {
            assert!(codes(vec![ok]).is_empty());
        }
        let f = codes(vec![col("payer_id", ColumnType::Integer, Some("people"))]);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].code, "reference_name");
        assert!(f[0].message.contains("person_id"));
        let f = codes(vec![col(
            "category_id",
            ColumnType::Integer,
            Some("expense_categories"),
        )]);
        assert_eq!(f[0].code, "reference_name");
        assert!(f[0].message.contains("expense_category_id"));
    }

    #[test]
    fn unlinked_id_rule() {
        let f = codes(vec![
            col("item_type", ColumnType::Text, None),
            col("item_id", ColumnType::Integer, None),
        ]);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].code, "unlinked_id");
        assert!(codes(vec![col("person_id", ColumnType::Integer, Some("people"))]).is_empty());
        assert!(codes(vec![col("external_id", ColumnType::Text, None)]).is_empty());
    }
}
