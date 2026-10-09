//! Read the shape of a SQLite DB and derive the expected shape from a spec.

use std::collections::BTreeMap;

use diesel::prelude::*;
use diesel::sql_types::{BigInt, Text};
use nocodo_sqlgen::{column_sql_type, table_indexes};
use nocodo_types::schema::SchemaSpec;

/// Normalized shape of one table. Everything is sorted so shapes compare by value.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct TableShape {
    /// (name, sql type, not null, primary key), sorted by name.
    pub columns: Vec<(String, String, bool, bool)>,
    /// (column, referenced table, referenced column), sorted.
    pub foreign_keys: Vec<(String, String, String)>,
    /// (name, columns in order, unique), sorted by name. Only explicitly created indexes.
    pub indexes: Vec<(String, Vec<String>, bool)>,
}

pub(crate) type Shape = BTreeMap<String, TableShape>;

#[derive(QueryableByName)]
struct TableListRow {
    #[diesel(sql_type = Text)]
    name: String,
    #[diesel(sql_type = Text, column_name = "type")]
    kind: String,
}

#[derive(QueryableByName)]
struct TableInfoRow {
    #[diesel(sql_type = Text)]
    name: String,
    #[diesel(sql_type = Text, column_name = "type")]
    kind: String,
    #[diesel(sql_type = BigInt)]
    notnull: i64,
    #[diesel(sql_type = BigInt)]
    pk: i64,
}

#[derive(QueryableByName)]
struct ForeignKeyRow {
    #[diesel(sql_type = Text, column_name = "table")]
    target: String,
    #[diesel(sql_type = Text)]
    from: String,
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    to: Option<String>,
}

#[derive(QueryableByName)]
struct IndexListRow {
    #[diesel(sql_type = Text)]
    name: String,
    #[diesel(sql_type = BigInt)]
    unique: i64,
    #[diesel(sql_type = Text)]
    origin: String,
}

#[derive(QueryableByName)]
struct IndexInfoRow {
    #[diesel(sql_type = diesel::sql_types::Nullable<Text>)]
    name: Option<String>,
}

fn ignored(name: &str) -> bool {
    name.starts_with("sqlite_") || name == "__diesel_schema_migrations"
}

pub(crate) fn introspect(conn: &mut SqliteConnection) -> QueryResult<Shape> {
    let tables: Vec<TableListRow> = diesel::sql_query("PRAGMA table_list").load(conn)?;
    let mut shape = Shape::new();
    for t in tables
        .iter()
        .filter(|t| t.kind == "table" && !ignored(&t.name))
    {
        let q = |pragma: &str| format!("PRAGMA {pragma}(\"{}\")", t.name);
        let mut s = TableShape::default();

        let cols: Vec<TableInfoRow> = diesel::sql_query(q("table_info")).load(conn)?;
        s.columns = cols
            .into_iter()
            .map(|c| (c.name, c.kind, c.notnull != 0, c.pk != 0))
            .collect();
        s.columns.sort();

        let fks: Vec<ForeignKeyRow> = diesel::sql_query(q("foreign_key_list")).load(conn)?;
        s.foreign_keys = fks
            .into_iter()
            .map(|f| (f.from, f.target, f.to.unwrap_or_else(|| "id".into())))
            .collect();
        s.foreign_keys.sort();

        let idx: Vec<IndexListRow> = diesel::sql_query(q("index_list")).load(conn)?;
        for i in idx.into_iter().filter(|i| i.origin == "c") {
            let info: Vec<IndexInfoRow> =
                diesel::sql_query(format!("PRAGMA index_info(\"{}\")", i.name)).load(conn)?;
            let columns = info.into_iter().filter_map(|r| r.name).collect();
            s.indexes.push((i.name, columns, i.unique != 0));
        }
        s.indexes.sort();
        shape.insert(t.name.clone(), s);
    }
    Ok(shape)
}

/// The shape SQLite should have for `spec`, using sqlgen's type mapping.
pub(crate) fn expected_shape(spec: &SchemaSpec) -> Shape {
    let mut shape = Shape::new();
    for t in &spec.tables {
        let mut s = TableShape::default();
        s.columns.push(("id".into(), "INTEGER".into(), false, true));
        for c in &t.columns {
            s.columns.push((
                c.name.clone(),
                column_sql_type(c.column_type).into(),
                !c.nullable,
                false,
            ));
            if let Some(r) = &c.references {
                s.foreign_keys
                    .push((c.name.clone(), r.clone(), "id".into()));
            }
        }
        for ts in ["created_at", "updated_at"] {
            s.columns.push((ts.into(), "TEXT".into(), true, false));
        }
        for i in table_indexes(t) {
            s.indexes.push((i.name, i.columns, i.unique));
        }
        s.columns.sort();
        s.foreign_keys.sort();
        s.indexes.sort();
        shape.insert(t.name.clone(), s);
    }
    shape
}

/// Human-readable differences, empty when the shapes are equal.
pub(crate) fn diff(expected: &Shape, actual: &Shape) -> Vec<String> {
    let mut out = Vec::new();
    for name in expected.keys().filter(|n| !actual.contains_key(*n)) {
        out.push(format!("table `{name}` is missing in the database"));
    }
    for name in actual.keys().filter(|n| !expected.contains_key(*n)) {
        out.push(format!(
            "table `{name}` exists in the database but not in the spec"
        ));
    }
    for (name, e) in expected {
        let Some(a) = actual.get(name) else { continue };
        if e.columns != a.columns {
            out.push(format!(
                "columns of `{name}` differ: expected {:?}, found {:?}",
                e.columns, a.columns
            ));
        }
        if e.foreign_keys != a.foreign_keys {
            out.push(format!(
                "foreign keys of `{name}` differ: expected {:?}, found {:?}",
                e.foreign_keys, a.foreign_keys
            ));
        }
        if e.indexes != a.indexes {
            out.push(format!(
                "indexes of `{name}` differ: expected {:?}, found {:?}",
                e.indexes, a.indexes
            ));
        }
    }
    out
}

/// Differences between two snapshots of the same database.
pub(crate) fn diff_shapes(before: &Shape, after: &Shape) -> Vec<String> {
    diff(before, after)
}
