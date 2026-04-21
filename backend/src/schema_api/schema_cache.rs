//! In-memory cache of schema/table/column metadata.
//!
//! Loaded once at startup. Invalidation will be added when schema mutation
//! endpoints are implemented.

use rusqlite::Connection;
use serde_json::Value;
use shared_types::{Column, DataType, Schema, Table};
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct SchemaCache {
    schemas: HashMap<i64, Schema>,
    tables: HashMap<i64, Table>,
    columns: HashMap<i64, Column>,
    /// Ordered column IDs per table
    table_columns: HashMap<i64, Vec<i64>>,
}

impl SchemaCache {
    pub fn load(conn: &Connection) -> Result<Self, rusqlite::Error> {
        let mut cache = SchemaCache::default();

        // Load schemas
        let mut stmt = conn.prepare(
            "SELECT id, project_id, name, created_at FROM app_schema ORDER BY id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Schema {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;
        for r in rows {
            let s = r?;
            cache.schemas.insert(s.id, s);
        }

        // Load tables
        let mut stmt = conn.prepare(
            "SELECT id, schema_id, name, created_at FROM schema_table ORDER BY id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(Table {
                id: row.get(0)?,
                schema_id: row.get(1)?,
                name: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?;
        for r in rows {
            let t = r?;
            cache.tables.insert(t.id, t);
        }

        // Load columns
        let mut stmt = conn.prepare(
            "SELECT id, table_id, name, data_type, nullable, primary_key, display_order, created_at
             FROM schema_column
             ORDER BY display_order, id",
        )?;
        let rows = stmt.query_map([], |row| {
            let data_type_str: String = row.get(3)?;
            let data_type = data_type_from_str(&data_type_str);
            Ok(Column {
                id: row.get(0)?,
                table_id: row.get(1)?,
                name: row.get(2)?,
                data_type,
                nullable: row.get::<_, i64>(4)? != 0,
                primary_key: row.get::<_, i64>(5)? != 0,
                display_order: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?;
        for r in rows {
            let col = r?;
            let table_id = col.table_id;
            let col_id = col.id;
            cache.columns.insert(col_id, col);
            cache.table_columns.entry(table_id).or_default().push(col_id);
        }

        Ok(cache)
    }

    pub fn get_schema(&self, id: i64) -> Option<&Schema> {
        self.schemas.get(&id)
    }

    pub fn get_table(&self, id: i64) -> Option<&Table> {
        self.tables.get(&id)
    }

    pub fn get_column(&self, id: i64) -> Option<&Column> {
        self.columns.get(&id)
    }

    /// Columns for a table in display order.
    pub fn get_table_columns(&self, table_id: i64) -> Vec<&Column> {
        self.table_columns
            .get(&table_id)
            .map(|ids| ids.iter().filter_map(|id| self.columns.get(id)).collect())
            .unwrap_or_default()
    }

    /// SQL column names for a table in display order (snake_case of column.name).
    pub fn get_table_sql_column_names(&self, table_id: i64) -> Vec<String> {
        self.get_table_columns(table_id)
            .iter()
            .map(|c| to_snake_case(&c.name))
            .collect()
    }

    /// SQL table name for a given table_id.
    /// Convention: {schema_name}_{table_name} in snake_case.
    /// "Nocodo Internal" schema maps to the hardcoded system tables.
    pub fn get_sql_table_name(&self, table_id: i64) -> Option<String> {
        let table = self.tables.get(&table_id)?;
        let schema = self.schemas.get(&table.schema_id)?;

        if schema.name == "Nocodo Internal" {
            let sql_name = match table.name.as_str() {
                "Projects"   => "project",
                "Sessions"   => "agent_chat_session",
                "Messages"   => "agent_chat_message",
                "Tool Calls" => "agent_tool_call",
                _ => return None,
            };
            Some(sql_name.to_string())
        } else {
            Some(format!(
                "{}_{}",
                to_snake_case(&schema.name),
                to_snake_case(&table.name)
            ))
        }
    }
}

/// Parse a stored snake_case data_type string into a DataType variant.
pub fn data_type_from_str(s: &str) -> DataType {
    serde_json::from_value(Value::String(s.to_string())).unwrap_or(DataType::Text)
}

/// Serialize a DataType to the snake_case string stored in the database.
pub fn data_type_to_str(dt: &DataType) -> String {
    serde_json::to_value(dt)
        .ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "text".to_string())
}

pub fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut prev_is_upper = false;
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 && !prev_is_upper {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap_or(c));
            prev_is_upper = true;
        } else if c == ' ' || c == '-' {
            result.push('_');
            prev_is_upper = false;
        } else {
            result.push(c);
            prev_is_upper = false;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("User Data"), "user_data");
        assert_eq!(to_snake_case("CustomerOrders"), "customer_orders");
        assert_eq!(to_snake_case("API-Keys"), "api_keys");
        assert_eq!(to_snake_case("simple"), "simple");
    }

    #[test]
    fn test_data_type_roundtrip() {
        for dt in [
            DataType::Text,
            DataType::Integer,
            DataType::Real,
            DataType::Boolean,
            DataType::Date,
            DataType::DateTime,
        ] {
            let s = data_type_to_str(&dt);
            assert_eq!(data_type_from_str(&s), dt);
        }
    }
}
