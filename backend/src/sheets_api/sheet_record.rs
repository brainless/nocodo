//! SheetRecord trait and implementations for virtual sheets
//!
//! This module provides a bridge between the sheet schema (defined in
//! sheet/sheet_tab/sheet_tab_column tables) and actual SQL tables.
//!
//! Each SQL table that corresponds to a sheet tab implements SheetRecord,
//! enabling generic CRUD operations while maintaining type safety.

use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::Serialize;
use shared_types::Project;

/// Trait for records that correspond to a sheet_tab
///
/// Implement this for SQL tables that should be exposed as sheets.
/// The implementation is typically code-generated based on sheet_tab_column entries.
pub trait SheetRecord: Sized + Serialize {
    /// The sheet_tab.id this record maps to
    fn sheet_tab_id() -> i64;

    /// SQL table name (e.g., "project", "agent_chat_session")
    fn table_name() -> &'static str;

    /// Column names for SELECT queries, matching sheet_tab_column.name order
    fn column_names() -> &'static [&'static str];

    /// SQL column expressions for SELECT (usually same as column_names)
    /// Override if columns need transformation (e.g., datetime formatting)
    fn select_columns() -> &'static [&'static str] {
        Self::column_names()
    }

    /// Parse from SQL row. Column order matches column_names().
    fn from_row(row: &Row) -> Result<Self, rusqlite::Error>;

    /// Primary key value
    fn id(&self) -> i64;

    /// Created timestamp for sorting
    fn created_at(&self) -> i64;

    /// Convert to column-ID-keyed JSON for the API response.
    /// The keys are sheet_tab_column.id values, not struct field names.
    /// This is a placeholder for code-generated implementations.
    fn to_column_json(&self) -> serde_json::Value;
}

/// Generic read helper for SheetRecord types
///
/// Returns records and total count for pagination
pub fn list_records<T: SheetRecord>(
    conn: &Connection,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<(Vec<T>, i64), rusqlite::Error> {
    let limit = limit.unwrap_or(100).clamp(1, 1000);
    let offset = offset.unwrap_or(0);

    // Get total count
    let total_count: i64 = conn.query_row(
        &format!("SELECT COUNT(*) FROM {}", T::table_name()),
        [],
        |row| row.get(0),
    )?;

    // Build query
    let columns = T::select_columns().join(", ");
    let sql = format!(
        "SELECT {} FROM {} ORDER BY id LIMIT ?1 OFFSET ?2",
        columns,
        T::table_name()
    );

    let mut stmt = conn.prepare(&sql)?;
    let records = stmt
        .query_map(params![limit, offset], |row| T::from_row(row))?
        .collect::<Result<Vec<_>, _>>()?;

    Ok((records, total_count))
}

/// Get a single record by ID
pub fn get_record_by_id<T: SheetRecord>(
    conn: &Connection,
    id: i64,
) -> Result<Option<T>, rusqlite::Error> {
    let columns = T::select_columns().join(", ");
    let sql = format!("SELECT {} FROM {} WHERE id = ?1", columns, T::table_name());

    conn.query_row(&sql, params![id], |row| T::from_row(row))
        .optional()
}

// ============================================================================
// Concrete implementations for existing tables
// ============================================================================

impl SheetRecord for Project {
    fn sheet_tab_id() -> i64 {
        6 // Projects tab (Nocodo Internal sheet)
    }

    fn table_name() -> &'static str {
        "project"
    }

    fn column_names() -> &'static [&'static str] {
        &["id", "name", "path", "created_at"]
    }

    fn from_row(row: &Row) -> Result<Self, rusqlite::Error> {
        Ok(Project {
            id: row.get(0)?,
            name: row.get(1)?,
            path: row.get(2)?,
            created_at: row.get(3)?,
        })
    }

    fn id(&self) -> i64 {
        self.id
    }

    fn created_at(&self) -> i64 {
        self.created_at
    }

    fn to_column_json(&self) -> serde_json::Value {
        // Column IDs for Projects tab: 10=ID, 11=Name, 12=Path, 13=Created At
        serde_json::json!({
            "10": self.id,
            "11": &self.name,
            "12": &self.path,
            "13": self.created_at
        })
    }
}

/// AgentChatSession record - maps to `agent_chat_session` table
#[derive(Debug, Clone, Serialize)]
pub struct AgentChatSession {
    pub id: i64,
    pub project_id: i64, // This is the relation field
    pub agent_type: String,
    pub created_at: i64,
}

impl SheetRecord for AgentChatSession {
    fn sheet_tab_id() -> i64 {
        7 // Sessions tab (Nocodo Internal sheet)
    }

    fn table_name() -> &'static str {
        "agent_chat_session"
    }

    fn column_names() -> &'static [&'static str] {
        &["id", "project_id", "agent_type", "created_at"]
    }

    fn from_row(row: &Row) -> Result<Self, rusqlite::Error> {
        Ok(AgentChatSession {
            id: row.get(0)?,
            project_id: row.get(1)?,
            agent_type: row.get(2)?,
            created_at: row.get(3)?,
        })
    }

    fn id(&self) -> i64 {
        self.id
    }

    fn created_at(&self) -> i64 {
        self.created_at
    }

    fn to_column_json(&self) -> serde_json::Value {
        // Column IDs for Sessions tab: 14=ID, 15=Project, 16=Agent Type, 17=Created At
        serde_json::json!({
            "14": self.id,
            "15": self.project_id,
            "16": &self.agent_type,
            "17": self.created_at
        })
    }
}

/// AgentChatMessage record - maps to `agent_chat_message` table
#[derive(Debug, Clone, Serialize)]
pub struct AgentChatMessage {
    pub id: i64,
    pub session_id: i64, // Relation field
    pub role: String,
    pub content: String,
    pub created_at: i64,
}

impl SheetRecord for AgentChatMessage {
    fn sheet_tab_id() -> i64 {
        8 // Messages tab (Nocodo Internal sheet)
    }

    fn table_name() -> &'static str {
        "agent_chat_message"
    }

    fn column_names() -> &'static [&'static str] {
        &["id", "session_id", "role", "content", "created_at"]
    }

    fn from_row(row: &Row) -> Result<Self, rusqlite::Error> {
        Ok(AgentChatMessage {
            id: row.get(0)?,
            session_id: row.get(1)?,
            role: row.get(2)?,
            content: row.get(3)?,
            created_at: row.get(4)?,
        })
    }

    fn id(&self) -> i64 {
        self.id
    }

    fn created_at(&self) -> i64 {
        self.created_at
    }

    fn to_column_json(&self) -> serde_json::Value {
        // Column IDs for Messages tab: 18=ID, 19=Session, 20=Role, 21=Content, 22=Created At
        serde_json::json!({
            "18": self.id,
            "19": self.session_id,
            "20": &self.role,
            "21": &self.content,
            "22": self.created_at
        })
    }
}

/// AgentToolCall record - maps to `agent_tool_call` table
#[derive(Debug, Clone, Serialize)]
pub struct AgentToolCall {
    pub id: i64,
    pub message_id: i64, // Relation field
    pub call_id: String,
    pub tool_name: String,
    pub arguments: String, // JSON stored as text
    pub result: Option<String>,
    pub created_at: i64,
}

impl SheetRecord for AgentToolCall {
    fn sheet_tab_id() -> i64 {
        9 // Tool Calls tab (Nocodo Internal sheet)
    }

    fn table_name() -> &'static str {
        "agent_tool_call"
    }

    fn column_names() -> &'static [&'static str] {
        &[
            "id",
            "message_id",
            "call_id",
            "tool_name",
            "arguments",
            "result",
            "created_at",
        ]
    }

    fn from_row(row: &Row) -> Result<Self, rusqlite::Error> {
        Ok(AgentToolCall {
            id: row.get(0)?,
            message_id: row.get(1)?,
            call_id: row.get(2)?,
            tool_name: row.get(3)?,
            arguments: row.get(4)?,
            result: row.get(5)?,
            created_at: row.get(6)?,
        })
    }

    fn id(&self) -> i64 {
        self.id
    }

    fn created_at(&self) -> i64 {
        self.created_at
    }

    fn to_column_json(&self) -> serde_json::Value {
        // Column IDs for Tool Calls tab: 23=ID, 24=Message, 25=Call ID, 26=Tool Name,
        // 27=Arguments, 28=Result, 29=Created At
        let mut json = serde_json::json!({
            "23": self.id,
            "24": self.message_id,
            "25": &self.call_id,
            "26": &self.tool_name,
            "27": &self.arguments,
            "29": self.created_at
        });
        if let Some(ref result) = self.result {
            json["28"] = serde_json::json!(result);
        }
        json
    }
}

// ============================================================================
// Helper to route by sheet_tab_id
// ============================================================================

/// Mapping from sheet_tab_id to the corresponding SQL table
///
/// These are the "Nocodo Internal" sheet tabs:
/// - 6: Projects (project table)
/// - 7: Sessions (agent_chat_session table)  
/// - 8: Messages (agent_chat_message table)
/// - 9: Tool Calls (agent_tool_call table)
pub fn get_sheet_tab_table_name(sheet_tab_id: i64) -> Option<&'static str> {
    match sheet_tab_id {
        6 => Some("project"),
        7 => Some("agent_chat_session"),
        8 => Some("agent_chat_message"),
        9 => Some("agent_tool_call"),
        _ => None,
    }
}

/// Check if a sheet_tab_id corresponds to a virtual table (not sheet_tab_row)
pub fn is_virtual_sheet_tab(sheet_tab_id: i64) -> bool {
    get_sheet_tab_table_name(sheet_tab_id).is_some()
}
