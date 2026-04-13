//! Schema cache for sheet metadata
//!
//! Provides in-memory caching of sheet/sheet_tab/sheet_tab_column tables
//! to avoid repeated database queries when building dynamic SELECT statements.
//!
//! Cache invalidation will be added later when schema mutations are implemented.

use rusqlite::Connection;
use shared_types::{Sheet, SheetTab, SheetTabColumn};
use std::collections::HashMap;

/// Cache entry for a complete schema hierarchy
#[derive(Debug, Clone, Default)]
pub struct SchemaCache {
    sheets: HashMap<i64, Sheet>,
    sheet_tabs: HashMap<i64, SheetTab>,
    sheet_tab_columns: HashMap<i64, SheetTabColumn>,
    /// Map from sheet_tab_id to ordered list of column IDs
    tab_columns_order: HashMap<i64, Vec<i64>>,
    /// Map from column_id to SQL column name (actual database column name)
    column_sql_names: HashMap<i64, String>,
}

impl SchemaCache {
    /// Load all schema data from the database
    pub fn load(conn: &Connection) -> Result<Self, rusqlite::Error> {
        let mut cache = SchemaCache::default();

        // Load sheets
        let mut stmt = conn.prepare(
            "SELECT id, project_id, name, created_at, updated_at FROM sheet ORDER BY id",
        )?;
        let sheet_rows = stmt.query_map([], |row| {
            Ok(Sheet {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;
        for sheet in sheet_rows {
            let sheet = sheet?;
            cache.sheets.insert(sheet.id, sheet);
        }

        // Load sheet_tabs
        let mut stmt = conn.prepare(
            "SELECT id, sheet_id, name, display_order, created_at, updated_at FROM sheet_tab ORDER BY display_order, id"
        )?;
        let tab_rows = stmt.query_map([], |row| {
            Ok(SheetTab {
                id: row.get(0)?,
                sheet_id: row.get(1)?,
                name: row.get(2)?,
                display_order: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?;
        for tab in tab_rows {
            let tab = tab?;
            cache.sheet_tabs.insert(tab.id, tab);
        }

        // Load sheet_tab_columns
        let mut stmt = conn.prepare(
            "SELECT id, sheet_tab_id, name, sql_name, column_type, is_required, is_unique,
                    default_value, display_order, created_at, width
             FROM sheet_tab_column
             ORDER BY display_order, id",
        )?;
        let col_rows = stmt.query_map([], |row| {
            let column_type_json: String = row.get(4)?;
            let column_type = serde_json::from_str(&column_type_json)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
            let sql_name: Option<String> = row.get(3)?;

            Ok((
                SheetTabColumn {
                    id: row.get(0)?,
                    sheet_tab_id: row.get(1)?,
                    name: row.get(2)?,
                    column_type,
                    is_required: row.get::<_, i64>(5)? != 0,
                    is_unique: row.get::<_, i64>(6)? != 0,
                    default_value: row.get(7)?,
                    display_order: row.get(8)?,
                    created_at: row.get(9)?,
                    width: row.get::<_, Option<i32>>(10)?.unwrap_or(120),
                },
                sql_name,
            ))
        })?;
        for col_result in col_rows {
            let (col, sql_name) = col_result?;
            let tab_id = col.sheet_tab_id;
            let col_id = col.id;

            // Store sql_name (fallback to name if not set)
            let sql_name = sql_name.unwrap_or_else(|| to_snake_case(&col.name));
            cache.column_sql_names.insert(col_id, sql_name);

            cache.sheet_tab_columns.insert(col_id, col);
            cache
                .tab_columns_order
                .entry(tab_id)
                .or_default()
                .push(col_id);
        }

        Ok(cache)
    }

    /// Get a sheet by ID
    pub fn get_sheet(&self, id: i64) -> Option<&Sheet> {
        self.sheets.get(&id)
    }

    /// Get a sheet tab by ID
    pub fn get_sheet_tab(&self, id: i64) -> Option<&SheetTab> {
        self.sheet_tabs.get(&id)
    }

    /// Get a sheet tab column by ID
    pub fn get_column(&self, id: i64) -> Option<&SheetTabColumn> {
        self.sheet_tab_columns.get(&id)
    }

    /// Get all columns for a sheet tab in display order
    pub fn get_tab_columns(&self, tab_id: i64) -> Vec<&SheetTabColumn> {
        self.tab_columns_order
            .get(&tab_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.sheet_tab_columns.get(id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get the SQL column name for a column
    pub fn get_column_sql_name(&self, col_id: i64) -> Option<&str> {
        self.column_sql_names.get(&col_id).map(|s| s.as_str())
    }

    /// Get SQL column names for all columns in a sheet tab (in display order)
    pub fn get_tab_sql_column_names(&self, tab_id: i64) -> Vec<&str> {
        self.tab_columns_order
            .get(&tab_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.column_sql_names.get(id).map(|s| s.as_str()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get the SQL table name for a sheet tab
    /// Returns (table_name, sheet_name) or None if not found
    pub fn get_sql_table_name(&self, tab_id: i64) -> Option<String> {
        let tab = self.sheet_tabs.get(&tab_id)?;
        let sheet = self.sheets.get(&tab.sheet_id)?;

        // SQL table naming convention: {sheet_name}_{tab_name} in snake_case
        // For "Nocodo Internal" sheet, we use the hardcoded virtual tables
        if sheet.name == "Nocodo Internal" {
            match tab.name.as_str() {
                "Projects" => Some("project".to_string()),
                "Sessions" => Some("agent_chat_session".to_string()),
                "Messages" => Some("agent_chat_message".to_string()),
                "Tool Calls" => Some("agent_tool_call".to_string()),
                _ => None,
            }
        } else {
            // User-defined tables: sheet_name_tab_name
            let table_name = format!(
                "{}_{}",
                to_snake_case(&sheet.name),
                to_snake_case(&tab.name)
            );
            Some(table_name)
        }
    }
}

/// Convert a string to snake_case for SQL table names
fn to_snake_case(s: &str) -> String {
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
}
