//! Deterministic word forms used by naming rules.

use crate::schema::Column;

/// A name in words: `expense_categories` -> `expense categories`.
pub fn words(name: &str) -> String {
    name.replace('_', " ")
}

/// A detail in words: references drop `_id`. Shared by the `inspect` summary
/// and the removal confirmation, so both say the same thing.
pub fn column_words(c: &Column) -> String {
    let mut name = c.name.as_str();
    if c.references.is_some() {
        name = name.strip_suffix("_id").unwrap_or(name);
    }
    words(name)
}

const IRREGULAR: [(&str, &str); 8] = [
    ("people", "person"),
    ("children", "child"),
    ("men", "man"),
    ("women", "woman"),
    ("statuses", "status"),
    ("indices", "index"),
    ("leaves", "leaf"),
    ("data", "datum"),
];

/// Singular form of a snake_case plural table name. Only the last word is
/// changed: `expense_categories` -> `expense_category`.
pub fn singular(name: &str) -> String {
    let (head, last) = match name.rfind('_') {
        Some(i) => (&name[..=i], &name[i + 1..]),
        None => ("", name),
    };
    format!("{head}{}", singular_word(last))
}

fn singular_word(w: &str) -> String {
    if let Some((_, s)) = IRREGULAR.iter().find(|(p, _)| *p == w) {
        return (*s).to_string();
    }
    if let Some(stem) = w.strip_suffix("ies").filter(|s| !s.is_empty()) {
        return format!("{stem}y");
    }
    for suffix in ["sses", "xes", "ches", "shes", "zes"] {
        if w.ends_with(suffix) {
            return w[..w.len() - 2].to_string();
        }
    }
    if w.ends_with("ss") || w.ends_with("us") || w.ends_with("is") {
        return w.to_string();
    }
    match w.strip_suffix('s') {
        Some(stem) if !stem.is_empty() => stem.to_string(),
        _ => w.to_string(),
    }
}

/// True when `column` is `<singular>_id` or `<qualifier>_<singular>_id`
/// for the referenced `table`.
pub fn is_reference_name(column: &str, table: &str) -> bool {
    let expected = expected_reference_name(table);
    column == expected || column.ends_with(&format!("_{expected}"))
}

/// The plain form of a reference column name: `people` -> `person_id`.
pub fn expected_reference_name(table: &str) -> String {
    format!("{}_id", singular(table))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn singular_forms() {
        for (p, s) in [
            ("people", "person"),
            ("expense_categories", "expense_category"),
            ("trips", "trip"),
            ("addresses", "address"),
            ("boxes", "box"),
            ("batches", "batch"),
            ("statuses", "status"),
            ("status", "status"),
            ("trip_children", "trip_child"),
            ("accounts", "account"),
        ] {
            assert_eq!(singular(p), s, "{p}");
        }
    }

    #[test]
    fn reference_names() {
        assert!(is_reference_name("person_id", "people"));
        assert!(is_reference_name("paid_by_person_id", "people"));
        assert!(is_reference_name(
            "expense_category_id",
            "expense_categories"
        ));
        assert!(!is_reference_name("payer_id", "people"));
        assert!(!is_reference_name("category_id", "expense_categories"));
        assert!(!is_reference_name("xperson_id", "people"));
    }
}
