//! Deterministic code generator: `SchemaDef` → Rust structs + SQLite DDL.
//!
//! The pipeline is intentionally split into granular stages so callers can
//! inspect or transform the intermediate representation.
//!
//! ```text
//! SchemaDef ──► Vec<TableModel> ──► Rust source code
//!                    │
//!                    └──► SQLite DDL
//! ```

use shared_types::{ColumnDef, DataType, SchemaDef, TableDef};
use std::fmt::Write;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Intermediate model of a table, decoupled from SchemaDef.
#[derive(Debug, Clone, PartialEq)]
pub struct TableModel {
    /// Original SQL table name (plural snake_case).
    pub sql_name: String,
    /// Optional human-readable table label for display usage.
    pub label: Option<String>,
    /// Rust struct name (singular PascalCase derived from sql_name).
    pub rust_name: String,
    pub columns: Vec<ColumnModel>,
}

/// Intermediate model of a column.
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnModel {
    /// SQL column name (snake_case).
    pub sql_name: String,
    /// Optional human-readable column label for display usage.
    pub label: Option<String>,
    /// Rust field name (snake_case, same as sql_name).
    pub rust_name: String,
    /// Rust type expression (e.g. `String`, `Option<i64>`).
    pub rust_type: String,
    /// SQLite affinity type (TEXT, INTEGER, REAL, NUMERIC, BLOB).
    pub sql_type: String,
    /// Original source data type — preserved so Diesel generators can
    /// distinguish Date from Integer, DateTime from Integer, etc.
    pub data_type: DataType,
    pub nullable: bool,
    pub primary_key: bool,
    pub foreign_key: Option<ForeignKeyModel>,
}

/// Foreign key reference in the intermediate model.
#[derive(Debug, Clone, PartialEq)]
pub struct ForeignKeyModel {
    pub ref_table: String,
    pub ref_column: String,
}

/// Result of running both generators.
#[derive(Debug, Clone, PartialEq)]
pub struct CodegenResult {
    pub rust_code: String,
    pub sql_ddl: String,
}

/// Result of the Diesel code generator.
#[derive(Debug, Clone, PartialEq)]
pub struct DieselCodegenResult {
    /// Complete schema.rs content: all `diesel::table!` blocks, `joinable!`,
    /// and `allow_tables_to_appear_in_same_query!`.
    pub schema_code: String,
    /// Per-table model file contents.
    pub model_files: Vec<DieselModelFile>,
    /// Complete `models/mod.rs` content registering all tables.
    pub model_mod: String,
}

/// Content for one Diesel model file.
#[derive(Debug, Clone, PartialEq)]
pub struct DieselModelFile {
    /// SQL table name (e.g. "users").
    pub table_name: String,
    /// File path relative to the project backend (e.g. "backend/src/models/user.rs").
    pub file_path: String,
    /// Complete file content (imports, struct, impl block).
    pub content: String,
}

/// Display labels extracted from schema metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct SchemaLabels {
    pub schema_name: String,
    pub schema_label: Option<String>,
    pub tables: Vec<TableLabels>,
}

/// Display labels for one table.
#[derive(Debug, Clone, PartialEq)]
pub struct TableLabels {
    pub table_name: String,
    pub table_label: Option<String>,
    pub columns: Vec<ColumnLabels>,
}

/// Display labels for one column.
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnLabels {
    pub column_name: String,
    pub column_label: Option<String>,
}

// ---------------------------------------------------------------------------
// Stage 1: SchemaDef → AST
// ---------------------------------------------------------------------------

/// Convert a `SchemaDef` into a stable, queryable AST (`Vec<TableModel>`).
pub fn parse_schema_def(schema: &SchemaDef) -> Vec<TableModel> {
    schema.tables.iter().map(parse_table_def).collect()
}

fn parse_table_def(table: &TableDef) -> TableModel {
    let sql_name = table.name.clone();
    let rust_name = sql_name_to_rust_struct(&sql_name);
    let columns = table.columns.iter().map(parse_column_def).collect();
    TableModel {
        sql_name,
        label: table.label.clone(),
        rust_name,
        columns,
    }
}

fn parse_column_def(col: &ColumnDef) -> ColumnModel {
    let rust_type = data_type_to_rust(&col.data_type, col.nullable);
    let sql_type = data_type_to_sql(&col.data_type);
    ColumnModel {
        sql_name: col.name.clone(),
        label: col.label.clone(),
        rust_name: col.name.clone(),
        rust_type,
        sql_type: sql_type.to_string(),
        data_type: col.data_type.clone(),
        nullable: col.nullable,
        primary_key: col.primary_key,
        foreign_key: col.foreign_key.as_ref().and_then(|fk| {
            if fk.ref_table.is_empty() || fk.ref_column.is_empty() {
                None
            } else {
                Some(ForeignKeyModel {
                    ref_table: fk.ref_table.clone(),
                    ref_column: fk.ref_column.clone(),
                })
            }
        }),
    }
}

// ---------------------------------------------------------------------------
// Stage 2a: AST → Rust source code
// ---------------------------------------------------------------------------

/// Generate a single Rust struct from a `TableModel`.
pub fn table_model_to_rust_struct(table: &TableModel) -> String {
    let mut out = String::new();
    writeln!(
        &mut out,
        "#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]"
    )
    .unwrap();
    writeln!(&mut out, "pub struct {} {{", table.rust_name).unwrap();
    for col in &table.columns {
        writeln!(&mut out, "    pub {}: {},", col.rust_name, col.rust_type).unwrap();
    }
    writeln!(&mut out, "}}").unwrap();
    out
}

/// Generate a Rust module containing all structs plus shared imports.
pub fn tables_to_rust_module(tables: &[TableModel]) -> String {
    let mut out = String::new();
    writeln!(&mut out, "// Generated by schema-codegen").unwrap();
    writeln!(&mut out, "// Do not edit manually").unwrap();
    writeln!(&mut out).unwrap();
    for table in tables {
        out.push_str(&table_model_to_rust_struct(table));
        out.push('\n');
    }
    out
}

// ---------------------------------------------------------------------------
// Stage 2b: AST → SQLite DDL
// ---------------------------------------------------------------------------

/// Generate a `CREATE TABLE` statement for a single `TableModel`.
pub fn table_model_to_sql_create(table: &TableModel) -> String {
    let mut out = String::new();
    writeln!(&mut out, "CREATE TABLE IF NOT EXISTS {} (", table.sql_name).unwrap();

    let col_count = table.columns.len();
    let mut fks: Vec<String> = Vec::new();

    for (i, col) in table.columns.iter().enumerate() {
        let comma = if i < col_count - 1 || !fks.is_empty() {
            ","
        } else {
            ""
        };
        let mut constraints = String::new();
        if col.primary_key {
            constraints.push_str(" PRIMARY KEY AUTOINCREMENT");
        }
        if !col.nullable && !col.primary_key {
            constraints.push_str(" NOT NULL");
        }
        writeln!(
            &mut out,
            "    {} {}{}{}",
            col.sql_name, col.sql_type, constraints, comma
        )
        .unwrap();

        if let Some(fk) = &col.foreign_key {
            fks.push(format!(
                "    FOREIGN KEY ({}) REFERENCES {}({})",
                col.sql_name, fk.ref_table, fk.ref_column
            ));
        }
    }

    // Append foreign key constraints at the end
    let fk_count = fks.len();
    for (i, fk) in fks.iter().enumerate() {
        let comma = if i < fk_count - 1 { "," } else { "" };
        writeln!(&mut out, "{}{}", fk, comma).unwrap();
    }

    writeln!(&mut out, ");").unwrap();
    out
}

/// Generate a full SQLite DDL script from a slice of `TableModel`s.
pub fn tables_to_sql_ddl(tables: &[TableModel]) -> String {
    let mut out = String::new();
    writeln!(&mut out, "-- Generated by schema-codegen").unwrap();
    writeln!(&mut out, "-- Do not edit manually").unwrap();
    writeln!(&mut out).unwrap();
    for table in tables {
        out.push_str(&table_model_to_sql_create(table));
        out.push('\n');
    }
    out
}

// ---------------------------------------------------------------------------
// Convenience: SchemaDef → both outputs
// ---------------------------------------------------------------------------

/// Run the full pipeline: `SchemaDef` → Rust code + SQL DDL.
pub fn generate(schema: &SchemaDef) -> CodegenResult {
    let tables = parse_schema_def(schema);
    CodegenResult {
        rust_code: tables_to_rust_module(&tables),
        sql_ddl: tables_to_sql_ddl(&tables),
    }
}

/// Extract name/label metadata for schema/table/column display usage.
pub fn schema_labels(schema: &SchemaDef) -> SchemaLabels {
    SchemaLabels {
        schema_name: schema.name.clone(),
        schema_label: schema.label.clone(),
        tables: schema
            .tables
            .iter()
            .map(|t| TableLabels {
                table_name: t.name.clone(),
                table_label: t.label.clone(),
                columns: t
                    .columns
                    .iter()
                    .map(|c| ColumnLabels {
                        column_name: c.name.clone(),
                        column_label: c.label.clone(),
                    })
                    .collect(),
            })
            .collect(),
    }
}

// ---------------------------------------------------------------------------
// Stage 3a: TableModel → Diesel schema (table! + joinable! + allow_…)
// ---------------------------------------------------------------------------

/// Map `DataType` to the Diesel SQL type used in `diesel::table!` blocks.
pub fn data_type_to_diesel_sql(dt: &DataType) -> &'static str {
    match dt {
        DataType::Text => "Text",
        DataType::Integer => "BigInt",
        DataType::Real => "Float",
        DataType::Boolean => "Bool",
        DataType::Date => "Date",
        DataType::DateTime => "Timestamp",
    }
}

/// Map `DataType` to the Rust type used in Diesel model struct fields.
/// Not wrapped in `Option<>` — callers handle nullability separately.
pub fn data_type_to_diesel_rust(dt: &DataType, nullable: bool) -> String {
    let base = match dt {
        DataType::Text => "String",
        DataType::Integer => "i64",
        DataType::Real => "f64",
        DataType::Boolean => "bool",
        DataType::Date => "NaiveDate",
        DataType::DateTime => "NaiveDateTime",
    };
    if nullable {
        format!("Option<{base}>")
    } else {
        base.to_string()
    }
}

/// Return the Diesel SQL type expression for a column, wrapping with
/// `Nullable<>` when the column is nullable.
fn column_to_diesel_sql_type(col: &ColumnModel) -> String {
    let dt = data_type_to_diesel_sql(&col.data_type);
    if col.nullable {
        format!("Nullable<{dt}>")
    } else {
        dt.to_string()
    }
}

/// Generate one `diesel::table!` block.
pub fn table_model_to_diesel_table(table: &TableModel) -> String {
    let pk_cols: Vec<&str> = table
        .columns
        .iter()
        .filter(|c| c.primary_key)
        .map(|c| c.sql_name.as_str())
        .collect();
    let pk = if pk_cols.is_empty() {
        "id".to_string()
    } else {
        pk_cols.join(", ")
    };

    let mut out = String::new();
    out.push_str("diesel::table! {\n");
    out.push_str(&format!("    {} ({}) {{\n", table.sql_name, pk));

    for col in &table.columns {
        // SQLite has no native Boolean type; some Diesel schemas use Bool
        // and some use Integer.  We map Boolean → Bool, everything else via
        // the column's inferred Diesel type.
        let sql_type = if col.data_type == DataType::Boolean {
            "Bool".to_string()
        } else {
            column_to_diesel_sql_type(col)
        };
        out.push_str(&format!("        {} -> {},\n", col.sql_name, sql_type));
    }

    out.push_str("    }\n");
    out.push_str("}\n");
    out
}

/// Generate the complete `schema.rs` file content from all tables.
///
/// Groups all `diesel::table!` blocks, all `joinable!` declarations (one per
/// foreign-key column), and a sorted `allow_tables_to_appear_in_same_query!`.
pub fn tables_to_diesel_schema(tables: &[TableModel]) -> String {
    let mut out = String::new();

    // ── table! blocks ──────────────────────────────────────────────────
    for table in tables {
        out.push_str(&table_model_to_diesel_table(table));
        out.push('\n');
    }

    // ── joinable! lines ────────────────────────────────────────────────
    let mut joinables: Vec<String> = Vec::new();
    for table in tables {
        for col in &table.columns {
            if let Some(fk) = &col.foreign_key {
                if !fk.ref_table.is_empty() && !fk.ref_column.is_empty() {
                    joinables.push(format!(
                        "diesel::joinable!({} -> {} ({}));",
                        table.sql_name, fk.ref_table, col.sql_name
                    ));
                }
            }
        }
    }
    if !joinables.is_empty() {
        joinables.sort();
        for j in &joinables {
            out.push_str(j);
            out.push('\n');
        }
        out.push('\n');
    }

    // ── allow_tables_to_appear_in_same_query! ──────────────────────────
    let table_names: Vec<&str> = {
        let mut names: Vec<&str> = tables.iter().map(|t| t.sql_name.as_str()).collect();
        names.sort();
        names
    };
    if !table_names.is_empty() {
        out.push_str("diesel::allow_tables_to_appear_in_same_query!(");
        let indent = "\n    ";
        for (i, name) in table_names.iter().enumerate() {
            if i == 0 {
                out.push_str(indent);
            }
            out.push_str(name);
            if i < table_names.len() - 1 {
                out.push_str(",");
                out.push_str(indent);
            }
        }
        out.push_str(",\n);\n");
    }

    out
}

// ---------------------------------------------------------------------------
// Stage 3b: TableModel → Diesel model file
// ---------------------------------------------------------------------------

/// Return the Rust struct name with the `Record` suffix convention used in
/// nocodo-managed template projects.
pub fn sql_name_to_record_name(sql_name: &str) -> String {
    format!("{}Record", sql_name_to_rust_struct(sql_name))
}

/// Generate the Diesel model struct definition (without imports or impl).
pub fn table_model_to_diesel_struct(table: &TableModel) -> String {
    let struct_name = sql_name_to_record_name(&table.sql_name);
    let mut out = String::new();

    // Derives
    out.push_str(
        "#[derive(Queryable, Selectable, Identifiable, Serialize, Deserialize)]\n",
    );
    out.push_str(&format!(
        "#[diesel(table_name = {})]\n",
        table.sql_name
    ));
    out.push_str("#[diesel(check_for_backend(diesel::sqlite::Sqlite))]\n");
    out.push_str(&format!("pub struct {struct_name} {{\n"));

    for col in &table.columns {
        let rust_type = data_type_to_diesel_rust(&col.data_type, col.nullable);
        out.push_str(&format!("    pub {}: {},\n", col.sql_name, rust_type));
    }

    out.push_str("}\n");
    out
}

/// Information about which `chrono` types are needed for imports.
#[derive(Default)]
struct ChronoNeeds {
    naive_date: bool,
    naive_date_time: bool,
}

fn collect_chrono_needs(table: &TableModel) -> ChronoNeeds {
    let mut needs = ChronoNeeds::default();
    for col in &table.columns {
        match col.data_type {
            DataType::Date => needs.naive_date = true,
            DataType::DateTime => needs.naive_date_time = true,
            _ => {}
        }
    }
    needs
}

/// Generate a complete Diesel model file (imports + struct + `impl` block
/// with templated `find_by_id` and `list`).
pub fn table_model_to_diesel_file(table: &TableModel) -> String {
    let struct_name = sql_name_to_record_name(&table.sql_name);
    let chrono = collect_chrono_needs(table);
    let singular = to_singular(&table.sql_name);
    let mut out = String::new();

    // ── Imports ────────────────────────────────────────────────────────
    if chrono.naive_date {
        out.push_str("use chrono::NaiveDate;\n");
    }
    if chrono.naive_date_time {
        out.push_str("use chrono::NaiveDateTime;\n");
    }
    out.push_str("use diesel::prelude::*;\n");
    out.push_str("use serde::{Deserialize, Serialize};\n");
    out.push('\n');
    out.push_str("use crate::db::DbPool;\n");
    out.push_str(&format!("use crate::schema::{};\n", table.sql_name));
    out.push('\n');

    // ── Struct ─────────────────────────────────────────────────────────
    out.push_str(&table_model_to_diesel_struct(table));
    out.push('\n');

    // ── impl block ─────────────────────────────────────────────────────
    out.push_str(&format!("impl {struct_name} {{\n"));

    // find_by_id
    let id_param = format!("{}_id", singular);
    out.push_str("    pub fn find_by_id(\n");
    out.push_str("        pool: &DbPool,\n");
    out.push_str(&format!("        {id_param}: i64,\n"));
    out.push_str("    ) -> Result<Option<Self>, diesel::result::Error> {\n");
    out.push_str(
        "        let mut conn = pool.get().expect(\"Failed to get connection\");\n",
    );
    out.push_str(&format!("        {}::table\n", table.sql_name));
    out.push_str(&format!("            .filter({}::id.eq({id_param}))\n", table.sql_name));
    out.push_str("            .select(Self::as_select())\n");
    out.push_str("            .first::<Self>(&mut conn)\n");
    out.push_str("            .optional()\n");
    out.push_str("    }\n");
    out.push('\n');

    // list
    out.push_str("    pub fn list(\n");
    out.push_str("        pool: &DbPool,\n");
    out.push_str("    ) -> Result<Vec<Self>, diesel::result::Error> {\n");
    out.push_str(
        "        let mut conn = pool.get().expect(\"Failed to get connection\");\n",
    );
    out.push_str(&format!("        {}::table\n", table.sql_name));
    out.push_str("            .select(Self::as_select())\n");
    out.push_str("            .load::<Self>(&mut conn)\n");
    out.push_str("    }\n");

    out.push_str("}\n");
    out
}

/// Generate the `pub mod` / `pub use` registration lines for one table.
pub fn table_to_mod_registration(table: &TableModel) -> String {
    let struct_name = sql_name_to_record_name(&table.sql_name);
    let file_stem = to_singular(&table.sql_name);
    format!(
        "pub mod {file_stem};\npub use {file_stem}::{struct_name};"
    )
}

/// Generate the complete `models/mod.rs` content for a set of tables.
pub fn tables_to_model_mod(tables: &[TableModel]) -> String {
    let mut out = String::new();
    for table in tables {
        out.push_str(&table_to_mod_registration(table));
        out.push('\n');
    }
    out
}

/// Return the relative file path for a table's model file.
pub fn table_model_file_path(table: &TableModel) -> String {
    let file_stem = to_singular(&table.sql_name);
    format!("backend/src/models/{file_stem}.rs")
}

// ---------------------------------------------------------------------------
// Convenience: SchemaDef → Diesel codegen
// ---------------------------------------------------------------------------

/// Run the full Diesel codegen pipeline: `SchemaDef` → schema.rs + per-table
/// model files + models/mod.rs.
pub fn generate_diesel(schema: &SchemaDef) -> DieselCodegenResult {
    let tables = parse_schema_def(schema);
    let schema_code = tables_to_diesel_schema(&tables);
    let model_files: Vec<DieselModelFile> = tables
        .iter()
        .map(|table| {
            let content = table_model_to_diesel_file(table);
            DieselModelFile {
                table_name: table.sql_name.clone(),
                file_path: table_model_file_path(table),
                content,
            }
        })
        .collect();
    let model_mod = tables_to_model_mod(&tables);
    DieselCodegenResult {
        schema_code,
        model_files,
        model_mod,
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn data_type_to_rust(dt: &DataType, nullable: bool) -> String {
    let base = match dt {
        DataType::Text => "String",
        DataType::Integer => "i64",
        DataType::Real => "f64",
        DataType::Boolean => "bool",
        DataType::Date => "i64",
        DataType::DateTime => "i64",
    };
    if nullable {
        format!("Option<{}>", base)
    } else {
        base.to_string()
    }
}

fn data_type_to_sql(dt: &DataType) -> &'static str {
    match dt {
        DataType::Text => "TEXT",
        DataType::Integer => "INTEGER",
        DataType::Real => "REAL",
        // SQLite has no native BOOLEAN; store as INTEGER 0/1
        DataType::Boolean => "INTEGER",
        // Store timestamps as Unix epoch seconds
        DataType::Date => "INTEGER",
        DataType::DateTime => "INTEGER",
    }
}

/// Convert a plural snake_case SQL table name to singular PascalCase Rust struct name.
fn sql_name_to_rust_struct(sql_name: &str) -> String {
    let singular = to_singular(sql_name);
    to_pascal_case(&singular)
}

fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    let mut word = first.to_uppercase().to_string();
                    word.extend(chars.map(|c| c.to_lowercase().to_string()));
                    word
                }
            }
        })
        .collect()
}

/// Very light singularisation heuristic.
fn to_singular(s: &str) -> String {
    if s.ends_with("ies") {
        format!("{}y", &s[..s.len() - 3])
    } else if s.ends_with("s") && !s.ends_with("ss") && !s.ends_with("us") {
        s[..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use shared_types::ForeignKeyDef;

    fn sample_schema() -> SchemaDef {
        SchemaDef {
            name: "Test App".to_string(),
            label: Some("Test App".to_string()),
            tables: vec![
                TableDef {
                    name: "users".to_string(),
                    label: Some("Users".to_string()),
                    columns: vec![
                        ColumnDef {
                            name: "id".to_string(),
                            label: Some("ID".to_string()),
                            data_type: DataType::Integer,
                            nullable: false,
                            primary_key: true,
                            foreign_key: None,
                        },
                        ColumnDef {
                            name: "name".to_string(),
                            label: Some("Name".to_string()),
                            data_type: DataType::Text,
                            nullable: false,
                            primary_key: false,
                            foreign_key: None,
                        },
                        ColumnDef {
                            name: "email".to_string(),
                            label: Some("Email".to_string()),
                            data_type: DataType::Text,
                            nullable: true,
                            primary_key: false,
                            foreign_key: None,
                        },
                        ColumnDef {
                            name: "active".to_string(),
                            label: Some("Active".to_string()),
                            data_type: DataType::Boolean,
                            nullable: false,
                            primary_key: false,
                            foreign_key: None,
                        },
                    ],
                },
                TableDef {
                    name: "orders".to_string(),
                    label: Some("Orders".to_string()),
                    columns: vec![
                        ColumnDef {
                            name: "id".to_string(),
                            label: Some("ID".to_string()),
                            data_type: DataType::Integer,
                            nullable: false,
                            primary_key: true,
                            foreign_key: None,
                        },
                        ColumnDef {
                            name: "user_id".to_string(),
                            label: Some("User".to_string()),
                            data_type: DataType::Integer,
                            nullable: false,
                            primary_key: false,
                            foreign_key: Some(ForeignKeyDef {
                                ref_table: "users".to_string(),
                                ref_column: "id".to_string(),
                            }),
                        },
                        ColumnDef {
                            name: "total".to_string(),
                            label: Some("Total".to_string()),
                            data_type: DataType::Real,
                            nullable: false,
                            primary_key: false,
                            foreign_key: None,
                        },
                    ],
                },
            ],
        }
    }

    #[test]
    fn test_parse_schema_def() {
        let schema = sample_schema();
        let tables = parse_schema_def(&schema);
        assert_eq!(tables.len(), 2);
        assert_eq!(tables[0].sql_name, "users");
        assert_eq!(tables[0].label.as_deref(), Some("Users"));
        assert_eq!(tables[0].rust_name, "User");
        assert_eq!(tables[0].columns[1].label.as_deref(), Some("Name"));
        assert_eq!(tables[1].sql_name, "orders");
        assert_eq!(tables[1].rust_name, "Order");
    }

    #[test]
    fn test_table_model_to_rust_struct() {
        let schema = sample_schema();
        let tables = parse_schema_def(&schema);
        let rust = table_model_to_rust_struct(&tables[0]);
        assert!(rust.contains("pub struct User"));
        assert!(rust.contains("pub id: i64,"));
        assert!(rust.contains("pub name: String,"));
        assert!(rust.contains("pub email: Option<String>,"));
        assert!(rust.contains("pub active: bool,"));
    }

    #[test]
    fn test_table_model_to_sql_create() {
        let schema = sample_schema();
        let tables = parse_schema_def(&schema);
        let sql = table_model_to_sql_create(&tables[1]);
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS orders"));
        assert!(sql.contains("id INTEGER PRIMARY KEY AUTOINCREMENT"));
        assert!(sql.contains("user_id INTEGER NOT NULL"));
        assert!(sql.contains("total REAL NOT NULL"));
        assert!(sql.contains("FOREIGN KEY (user_id) REFERENCES users(id)"));
    }

    #[test]
    fn test_generate_full() {
        let schema = sample_schema();
        let result = generate(&schema);
        assert!(result.rust_code.contains("pub struct User"));
        assert!(result.rust_code.contains("pub struct Order"));
        assert!(result.sql_ddl.contains("CREATE TABLE IF NOT EXISTS users"));
        assert!(result.sql_ddl.contains("CREATE TABLE IF NOT EXISTS orders"));
    }

    #[test]
    fn test_schema_labels_extraction() {
        let schema = sample_schema();
        let labels = schema_labels(&schema);
        assert_eq!(labels.schema_name, "Test App");
        assert_eq!(labels.schema_label.as_deref(), Some("Test App"));
        assert_eq!(labels.tables[0].table_name, "users");
        assert_eq!(labels.tables[0].table_label.as_deref(), Some("Users"));
        assert_eq!(labels.tables[0].columns[1].column_name, "name");
        assert_eq!(
            labels.tables[0].columns[1].column_label.as_deref(),
            Some("Name")
        );
    }

    #[test]
    fn test_singularisation() {
        assert_eq!(to_singular("users"), "user");
        assert_eq!(to_singular("orders"), "order");
        assert_eq!(to_singular("categories"), "category");
        assert_eq!(to_singular("status"), "status");
    }

    #[test]
    fn test_pascal_case() {
        assert_eq!(to_pascal_case("user"), "User");
        assert_eq!(to_pascal_case("order_item"), "OrderItem");
    }

    // -----------------------------------------------------------------------
    // Diesel codegen tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_data_type_to_diesel_sql() {
        assert_eq!(data_type_to_diesel_sql(&DataType::Text), "Text");
        assert_eq!(data_type_to_diesel_sql(&DataType::Integer), "BigInt");
        assert_eq!(data_type_to_diesel_sql(&DataType::Real), "Float");
        assert_eq!(data_type_to_diesel_sql(&DataType::Boolean), "Bool");
        assert_eq!(data_type_to_diesel_sql(&DataType::Date), "Date");
        assert_eq!(data_type_to_diesel_sql(&DataType::DateTime), "Timestamp");
    }

    #[test]
    fn test_data_type_to_diesel_rust() {
        assert_eq!(data_type_to_diesel_rust(&DataType::Text, false), "String");
        assert_eq!(data_type_to_diesel_rust(&DataType::Integer, false), "i64");
        assert_eq!(data_type_to_diesel_rust(&DataType::Real, false), "f64");
        assert_eq!(data_type_to_diesel_rust(&DataType::Boolean, false), "bool");
        assert_eq!(
            data_type_to_diesel_rust(&DataType::Date, false),
            "NaiveDate"
        );
        assert_eq!(
            data_type_to_diesel_rust(&DataType::DateTime, false),
            "NaiveDateTime"
        );
        assert_eq!(
            data_type_to_diesel_rust(&DataType::Text, true),
            "Option<String>"
        );
        assert_eq!(
            data_type_to_diesel_rust(&DataType::Integer, true),
            "Option<i64>"
        );
    }

    #[test]
    fn test_sql_name_to_record_name() {
        assert_eq!(sql_name_to_record_name("users"), "UserRecord");
        assert_eq!(sql_name_to_record_name("households"), "HouseholdRecord");
        assert_eq!(
            sql_name_to_record_name("order_items"),
            "OrderItemRecord"
        );
    }

    #[test]
    fn test_table_model_to_diesel_table() {
        let schema = sample_schema();
        let tables = parse_schema_def(&schema);
        let table_block = table_model_to_diesel_table(&tables[0]);
        // users table
        assert!(table_block.contains("diesel::table! {"));
        assert!(table_block.contains("users (id) {"));
        assert!(table_block.contains("id -> BigInt,"));
        assert!(table_block.contains("name -> Text,"));
        assert!(table_block.contains("email -> Nullable<Text>,"));
        assert!(table_block.contains("active -> Bool,"));
    }

    #[test]
    fn test_table_model_to_diesel_struct() {
        let schema = sample_schema();
        let tables = parse_schema_def(&schema);
        let struct_def = table_model_to_diesel_struct(&tables[0]);
        assert!(struct_def.contains("#[derive(Queryable, Selectable, Identifiable, Serialize, Deserialize)]"));
        assert!(struct_def.contains("#[diesel(table_name = users)]"));
        assert!(struct_def.contains("#[diesel(check_for_backend(diesel::sqlite::Sqlite))]"));
        assert!(struct_def.contains("pub struct UserRecord {"));
        assert!(struct_def.contains("pub id: i64,"));
        assert!(struct_def.contains("pub name: String,"));
        assert!(struct_def.contains("pub email: Option<String>,"));
        assert!(struct_def.contains("pub active: bool,"));
    }

    #[test]
    fn test_table_model_to_diesel_file_basics() {
        let schema = sample_schema();
        let tables = parse_schema_def(&schema);
        let file = table_model_to_diesel_file(&tables[0]);
        // Imports
        assert!(file.contains("use diesel::prelude::*;"));
        assert!(file.contains("use serde::{Deserialize, Serialize};"));
        assert!(file.contains("use crate::db::DbPool;"));
        assert!(file.contains("use crate::schema::users;"));
        // Struct
        assert!(file.contains("pub struct UserRecord {"));
        // impl
        assert!(file.contains("impl UserRecord {"));
        assert!(file.contains("pub fn find_by_id("));
        assert!(file.contains("user_id: i64"));
        assert!(file.contains("pub fn list("));
    }

    #[test]
    fn test_table_model_to_diesel_file_chrono_imports() {
        let schema = SchemaDef {
            name: "test".to_string(),
            label: None,
            tables: vec![TableDef {
                name: "events".to_string(),
                label: None,
                columns: vec![
                    ColumnDef {
                        name: "id".to_string(),
                        label: None,
                        data_type: DataType::Integer,
                        nullable: false,
                        primary_key: true,
                        foreign_key: None,
                    },
                    ColumnDef {
                        name: "event_date".to_string(),
                        label: None,
                        data_type: DataType::Date,
                        nullable: false,
                        primary_key: false,
                        foreign_key: None,
                    },
                    ColumnDef {
                        name: "created_at".to_string(),
                        label: None,
                        data_type: DataType::DateTime,
                        nullable: false,
                        primary_key: false,
                        foreign_key: None,
                    },
                    ColumnDef {
                        name: "finished_at".to_string(),
                        label: None,
                        data_type: DataType::DateTime,
                        nullable: true,
                        primary_key: false,
                        foreign_key: None,
                    },
                ],
            }],
        };
        let tables = parse_schema_def(&schema);
        let file = table_model_to_diesel_file(&tables[0]);
        assert!(file.contains("use chrono::NaiveDate;"));
        assert!(file.contains("use chrono::NaiveDateTime;"));
        assert!(file.contains("pub event_date: NaiveDate,"));
        assert!(file.contains("pub created_at: NaiveDateTime,"));
        assert!(file.contains("pub finished_at: Option<NaiveDateTime>,"));
    }

    #[test]
    fn test_tables_to_diesel_schema_with_joins() {
        let schema = sample_schema();
        let tables = parse_schema_def(&schema);
        let schema_code = tables_to_diesel_schema(&tables);

        // table! blocks
        assert!(schema_code.contains("diesel::table! {"));
        assert!(schema_code.contains("users (id) {"));
        assert!(schema_code.contains("orders (id) {"));

        // joinable!
        assert!(schema_code.contains("diesel::joinable!(orders -> users (user_id));"));

        // allow_tables_to_appear_in_same_query!
        assert!(schema_code.contains("diesel::allow_tables_to_appear_in_same_query!("));
        assert!(schema_code.contains("orders"));
        assert!(schema_code.contains("users"));
    }

    #[test]
    fn test_tables_to_model_mod() {
        let schema = sample_schema();
        let tables = parse_schema_def(&schema);
        let mod_rs = tables_to_model_mod(&tables);
        assert!(mod_rs.contains("pub mod user;\npub use user::UserRecord;"));
        assert!(mod_rs.contains("pub mod order;\npub use order::OrderRecord;"));
    }

    #[test]
    fn test_table_model_file_path() {
        let schema = sample_schema();
        let tables = parse_schema_def(&schema);
        assert_eq!(
            table_model_file_path(&tables[0]),
            "backend/src/models/user.rs"
        );
        assert_eq!(
            table_model_file_path(&tables[1]),
            "backend/src/models/order.rs"
        );
    }

    #[test]
    fn test_generate_diesel() {
        let schema = sample_schema();
        let result = generate_diesel(&schema);

        // Schema code
        assert!(result.schema_code.contains("diesel::table!"));
        assert!(result.schema_code.contains("joinable!"));
        assert!(result.schema_code.contains("allow_tables_to_appear_in_same_query!"));

        // Model files
        assert_eq!(result.model_files.len(), 2);
        let user_file = &result.model_files[0];
        assert_eq!(user_file.table_name, "users");
        assert_eq!(user_file.file_path, "backend/src/models/user.rs");
        assert!(user_file.content.contains("impl UserRecord {"));

        // Model mod
        assert!(result.model_mod.contains("pub mod user;"));
        assert!(result.model_mod.contains("pub mod order;"));
    }

    #[test]
    fn test_table_to_mod_registration() {
        let schema = sample_schema();
        let tables = parse_schema_def(&schema);
        let reg = table_to_mod_registration(&tables[0]);
        assert_eq!(reg, "pub mod user;\npub use user::UserRecord;");
    }

    #[test]
    fn test_composite_pk_table_schema() {
        let schema = SchemaDef {
            name: "test".to_string(),
            label: None,
            tables: vec![TableDef {
                name: "organization_users".to_string(),
                label: None,
                columns: vec![
                    ColumnDef {
                        name: "user_id".to_string(),
                        label: None,
                        data_type: DataType::Integer,
                        nullable: false,
                        primary_key: true,
                        foreign_key: Some(ForeignKeyDef {
                            ref_table: "users".to_string(),
                            ref_column: "id".to_string(),
                        }),
                    },
                    ColumnDef {
                        name: "organization_id".to_string(),
                        label: None,
                        data_type: DataType::Integer,
                        nullable: false,
                        primary_key: true,
                        foreign_key: Some(ForeignKeyDef {
                            ref_table: "organizations".to_string(),
                            ref_column: "id".to_string(),
                        }),
                    },
                    ColumnDef {
                        name: "role".to_string(),
                        label: None,
                        data_type: DataType::Text,
                        nullable: false,
                        primary_key: false,
                        foreign_key: None,
                    },
                ],
            }],
        };
        let tables = parse_schema_def(&schema);
        let table_block = table_model_to_diesel_table(&tables[0]);
        // Composite primary key
        assert!(table_block.contains("organization_users (user_id, organization_id) {"));

        let schema_code = tables_to_diesel_schema(&tables);
        assert!(
            schema_code
                .contains("diesel::joinable!(organization_users -> organizations (organization_id));")
        );
        assert!(
            schema_code
                .contains("diesel::joinable!(organization_users -> users (user_id));")
        );
    }
}
