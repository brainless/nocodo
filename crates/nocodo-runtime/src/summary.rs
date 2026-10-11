//! Plain-language summary of what the tool remembers.

use std::collections::BTreeMap;

use nocodo_types::schema::{Column, SchemaSpec};

/// "expense_categories" -> "Expense categories".
fn sentence_case(name: &str) -> String {
    let words = name.replace('_', " ");
    let mut chars = words.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// A detail in words; optional details are marked.
fn detail(c: &Column) -> String {
    let mut s = nocodo_types::naming::column_words(c);
    if c.nullable {
        s.push_str(" (optional)");
    }
    s
}

/// Describe each kind of thing the tool remembers, in schema order.
/// `rows` maps list names to how many entries are stored (missing means none).
pub fn summary(spec: &SchemaSpec, rows: &BTreeMap<String, u64>) -> String {
    let mut out = String::new();
    for t in &spec.tables {
        let n = rows.get(&t.name).copied().unwrap_or(0);
        let count = match n {
            0 => "no entries stored yet".to_string(),
            1 => "1 entry stored so far".to_string(),
            n => format!("{n} entries stored so far"),
        };
        out.push_str(&format!("{} ({count})\n", sentence_case(&t.name)));
        out.push_str(&format!("  {}\n", t.description));
        if !t.columns.is_empty() {
            out.push_str("  Each one has:\n");
            for c in &t.columns {
                out.push_str(&format!("    - {}\n", detail(c)));
            }
        }
        out.push('\n');
    }
    out.push_str("Every entry also remembers when it was added and when it last changed.\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use nocodo_types::schema::{ColumnType, Table};

    fn col(name: &str, nullable: bool, r: Option<&str>) -> Column {
        Column {
            name: name.into(),
            description: String::new(),
            column_type: ColumnType::Text,
            nullable,
            unique: false,
            references: r.map(Into::into),
        }
    }

    #[test]
    fn reads_in_words() {
        let spec = SchemaSpec {
            spec_version: 1,
            tables: vec![Table {
                name: "expense_categories".into(),
                description: "Kinds of spending.".into(),
                columns: vec![
                    col("paid_by_person_id", false, Some("people")),
                    col("note", true, None),
                ],
                indexes: vec![],
            }],
        };
        let rows = BTreeMap::from([("expense_categories".to_string(), 3)]);
        let s = summary(&spec, &rows);
        assert!(s.starts_with("Expense categories (3 entries stored so far)\n"));
        assert!(s.contains("    - paid by person\n"));
        assert!(s.contains("    - note (optional)\n"));
        assert!(s.ends_with("when it last changed.\n"));
    }
}
