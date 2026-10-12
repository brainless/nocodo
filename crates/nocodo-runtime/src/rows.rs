//! `read_rows`: a read-only, paginated look at one table's rows.

use std::path::Path;

use diesel::prelude::*;
use diesel::row::NamedRow;
use diesel::sql_types::{Nullable, Text};
use nocodo_types::naming::words;
use nocodo_types::outcome::{CellValue, RowColumn, RowPage};
use nocodo_types::schema::{ColumnType, IMPLICIT_COLUMNS, SchemaSpec};

use crate::apply::{Count, DB_FILE, RuntimeError, quote_ident};
use crate::inspect::read_only_uri;

/// Most rows one page may hold.
pub const MAX_PAGE: u64 = 1000;

/// A row with every value read as text (`NULL` stays none); typed afterwards.
struct TextRow(Vec<Option<String>>);

impl<DB> QueryableByName<DB> for TextRow
where
    DB: diesel::backend::Backend,
    Option<String>: diesel::deserialize::FromSql<Nullable<Text>, DB>,
{
    fn build<'a>(row: &impl NamedRow<'a, DB>) -> diesel::deserialize::Result<Self> {
        let mut cells = Vec::new();
        let mut i = 0;
        // Columns are aliased c0, c1, ... so the width needs no static type.
        while let Ok(v) = NamedRow::get::<Nullable<Text>, Option<String>>(row, &format!("c{i}")) {
            cells.push(v);
            i += 1;
        }
        Ok(TextRow(cells))
    }
}

fn typed(t: ColumnType, v: Option<String>) -> CellValue {
    let Some(s) = v else {
        return CellValue::Null;
    };
    match t {
        ColumnType::Integer | ColumnType::Money => s
            .parse()
            .map(CellValue::Integer)
            .unwrap_or(CellValue::Text(s)),
        ColumnType::Decimal => s.parse().map(CellValue::Real).unwrap_or(CellValue::Text(s)),
        ColumnType::Boolean => match s.as_str() {
            "0" => CellValue::Bool(false),
            "1" => CellValue::Bool(true),
            _ => CellValue::Text(s),
        },
        ColumnType::Text | ColumnType::Date | ColumnType::DateTime => CellValue::Text(s),
    }
}

/// Read up to `limit` rows of `table` from `offset`, ordered by `id`.
///
/// Column names and types come from `spec`; headings are in plain words.
/// `limit` is capped at [`MAX_PAGE`]. The database is opened read-only.
/// A table that is not in `spec` is refused.
pub fn read_rows(
    project_dir: &Path,
    spec: &SchemaSpec,
    table: &str,
    offset: u64,
    limit: u64,
) -> Result<RowPage, RuntimeError> {
    let t = spec
        .tables
        .iter()
        .find(|t| t.name == table)
        .ok_or_else(|| {
            let known: Vec<_> = spec.tables.iter().map(|t| t.name.as_str()).collect();
            RuntimeError::Refused(if known.is_empty() {
                format!("there is no list called `{table}`; nothing has been set up yet")
            } else {
                format!(
                    "there is no list called `{table}`; the lists are: {}",
                    known.join(", ")
                )
            })
        })?;
    let limit = limit.min(MAX_PAGE);
    let mut columns: Vec<RowColumn> = IMPLICIT_COLUMNS
        .iter()
        .map(|c| RowColumn {
            name: c.name.to_string(),
            label: words(c.name),
            column_type: c.column_type,
        })
        .collect();
    columns.extend(t.columns.iter().map(|c| RowColumn {
        name: c.name.clone(),
        label: nocodo_types::naming::column_words(c),
        column_type: c.column_type,
    }));

    let db = project_dir.join(DB_FILE);
    if !db.exists() {
        return Err(RuntimeError::Refused(
            "data.sqlite does not exist yet: a migration is pending (run `apply`)".into(),
        ));
    }
    let mut conn = SqliteConnection::establish(&read_only_uri(&db))
        .map_err(|e| RuntimeError::Database(e.to_string()))?;
    let q = quote_ident(table);
    let total = diesel::sql_query(format!("SELECT COUNT(*) AS n FROM {q}"))
        .get_result::<Count>(&mut conn)
        .map_err(|e| RuntimeError::Database(e.to_string()))?
        .n as u64;
    let select = columns
        .iter()
        .enumerate()
        .map(|(i, c)| format!("CAST({} AS TEXT) AS c{i}", quote_ident(&c.name)))
        .collect::<Vec<_>>()
        .join(", ");
    let rows = diesel::sql_query(format!(
        "SELECT {select} FROM {q} ORDER BY id LIMIT {limit} OFFSET {offset}"
    ))
    .load::<TextRow>(&mut conn)
    .map_err(|e| RuntimeError::Database(e.to_string()))?
    .into_iter()
    .map(|r| {
        r.0.into_iter()
            .zip(&columns)
            .map(|(v, c)| typed(c.column_type, v))
            .collect()
    })
    .collect();
    Ok(RowPage {
        table: table.to_string(),
        columns,
        rows,
        total,
        offset,
        limit,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{add_sample_rows, apply_pending};
    use nocodo_types::schema::{Column, Table};
    use std::fs;

    fn col(name: &str, t: ColumnType, nullable: bool) -> Column {
        Column {
            name: name.into(),
            description: String::new(),
            column_type: t,
            nullable,
            unique: false,
            references: None,
        }
    }

    fn spec() -> SchemaSpec {
        SchemaSpec {
            spec_version: 1,
            tables: vec![Table {
                name: "expense_items".into(),
                description: String::new(),
                columns: vec![
                    col("label", ColumnType::Text, false),
                    col("amount", ColumnType::Money, false),
                    col("weight", ColumnType::Decimal, false),
                    col("paid", ColumnType::Boolean, false),
                    col("note", ColumnType::Text, true),
                ],
                indexes: vec![],
            }],
        }
    }

    const UP: &str = "CREATE TABLE expense_items (id INTEGER PRIMARY KEY, \
        created_at TEXT NOT NULL DEFAULT 'x', updated_at TEXT NOT NULL DEFAULT 'x', \
        label TEXT NOT NULL, amount INTEGER NOT NULL, weight REAL NOT NULL, \
        paid INTEGER NOT NULL, note TEXT);";

    fn project() -> tempfile::TempDir {
        let p = tempfile::tempdir().unwrap();
        let m = p.path().join("migrations/0001_a");
        fs::create_dir_all(&m).unwrap();
        fs::write(m.join("up.sql"), UP).unwrap();
        fs::write(m.join("down.sql"), "SELECT 1;").unwrap();
        fs::write(m.join("metadata.toml"), "run_in_transaction = false\n").unwrap();
        apply_pending(p.path()).unwrap();
        p
    }

    #[test]
    fn empty_table_has_columns_and_no_rows() {
        let p = project();
        let page = read_rows(p.path(), &spec(), "expense_items", 0, 10).unwrap();
        assert_eq!(page.total, 0);
        assert!(page.rows.is_empty());
        let labels: Vec<_> = page.columns.iter().map(|c| c.label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "id",
                "created at",
                "updated at",
                "label",
                "amount",
                "weight",
                "paid",
                "note"
            ]
        );
    }

    #[test]
    fn sample_rows_are_typed_and_paged() {
        let p = project();
        add_sample_rows(p.path(), &spec(), 5).unwrap();
        let page = read_rows(p.path(), &spec(), "expense_items", 0, 2).unwrap();
        assert_eq!(page.total, 5);
        assert_eq!(page.rows.len(), 2);
        let first = &page.rows[0];
        assert_eq!(first[0], CellValue::Integer(1));
        assert!(matches!(first[1], CellValue::Text(_)));
        assert_eq!(first[3], CellValue::Text("sample 1".into()));
        assert_eq!(first[4], CellValue::Integer(1));
        assert_eq!(first[5], CellValue::Real(1.5));
        assert_eq!(first[6], CellValue::Bool(true));
        assert_eq!(first[7], CellValue::Text("sample 1".into()));
        let mut c = SqliteConnection::establish(&p.path().join(DB_FILE).to_string_lossy()).unwrap();
        diesel::sql_query("UPDATE expense_items SET note = NULL WHERE id = 1")
            .execute(&mut c)
            .unwrap();
        let page1 = read_rows(p.path(), &spec(), "expense_items", 0, 1).unwrap();
        assert_eq!(page1.rows[0][7], CellValue::Null);

        let next = read_rows(p.path(), &spec(), "expense_items", 2, 2).unwrap();
        assert_eq!(next.rows[0][0], CellValue::Integer(3));
        let last = read_rows(p.path(), &spec(), "expense_items", 4, 2).unwrap();
        assert_eq!(last.rows.len(), 1);
        let beyond = read_rows(p.path(), &spec(), "expense_items", 9, 2).unwrap();
        assert!(beyond.rows.is_empty());
        assert_eq!(beyond.total, 5);
        let json = serde_json::to_string(&page.rows[0]).unwrap();
        assert!(json.starts_with("[1,"), "{json}");
    }

    #[test]
    fn unknown_table_is_refused() {
        let p = project();
        let err = read_rows(p.path(), &spec(), "nope", 0, 10).unwrap_err();
        assert!(matches!(err, RuntimeError::Refused(_)));
        assert!(err.to_string().contains("expense_items"), "{err}");
        // A table in the database but not in the schema is refused too.
        let empty = SchemaSpec {
            spec_version: 1,
            tables: vec![],
        };
        assert!(read_rows(p.path(), &empty, "expense_items", 0, 10).is_err());
    }
}
