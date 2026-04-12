use rusqlite::{params, Connection};
use std::time::{SystemTime, UNIX_EPOCH};

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "nocodo.db".to_string());
    let conn = Connection::open(&database_url)?;

    let now_ts = now();

    // Check if "Nocodo Internal" sheet already exists
    let internal_exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sheet WHERE name = 'Nocodo Internal')",
        [],
        |row| row.get(0),
    )?;

    if internal_exists {
        println!("'Nocodo Internal' sheet already exists, skipping schema creation.");
        return Ok(());
    }

    // Create the "Nocodo Internal" sheet
    conn.execute(
        "INSERT INTO sheet (project_id, name, created_at, updated_at) VALUES (1, 'Nocodo Internal', ?1, ?1)",
        params![now_ts],
    )?;
    let sheet_id = conn.last_insert_rowid();
    println!("Created sheet: Nocodo Internal (id={})", sheet_id);

    // Create tabs for our internal tables
    let tabs = vec![
        ("Projects", 0),
        ("Sessions", 1),
        ("Messages", 2),
        ("Tool Calls", 3),
    ];

    let mut tab_ids: Vec<i64> = Vec::new();
    for (name, order) in &tabs {
        conn.execute(
            "INSERT INTO sheet_tab (sheet_id, name, display_order, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)",
            params![sheet_id, name, order, now_ts],
        )?;
        tab_ids.push(conn.last_insert_rowid());
    }
    println!(
        "Created {} tabs: Projects(id={}), Sessions(id={}), Messages(id={}), Tool Calls(id={})",
        tabs.len(),
        tab_ids[0],
        tab_ids[1],
        tab_ids[2],
        tab_ids[3]
    );

    // ============================================
    // Tab 0: Projects - columns matching project table
    // ============================================
    let projects_tab_id = tab_ids[0];
    let projects_columns = vec![
        ("ID", r#"{"type": "integer"}"#, 0, true, true),
        ("Name", r#"{"type": "text"}"#, 1, true, false),
        ("Created At", r#"{"type": "date_time"}"#, 2, true, false),
    ];

    for (name, col_type, order, required, unique) in &projects_columns {
        conn.execute(
            "INSERT INTO sheet_tab_column (sheet_tab_id, name, column_type, display_order, is_required, is_unique, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![projects_tab_id, name, col_type, order, *required as i64, *unique as i64, now_ts],
        )?;
    }
    println!("Created {} columns in Projects tab", projects_columns.len());

    // ============================================
    // Tab 1: Sessions - columns with relation to Projects
    // ============================================
    let sessions_tab_id = tab_ids[1];
    let project_relation_type = format!(
        r#"{{"type": "relation", "target_sheet_tab_id": {}, "display_column": "Name"}}"#,
        projects_tab_id
    );
    let sessions_columns = vec![
        ("ID", r#"{"type": "integer"}"#.to_string(), 0, true, true),
        ("Project", project_relation_type, 1, true, false),
        (
            "Agent Type",
            r#"{"type": "text"}"#.to_string(),
            2,
            true,
            false,
        ),
        (
            "Created At",
            r#"{"type": "date_time"}"#.to_string(),
            3,
            true,
            false,
        ),
    ];

    for (name, col_type, order, required, unique) in &sessions_columns {
        conn.execute(
            "INSERT INTO sheet_tab_column (sheet_tab_id, name, column_type, display_order, is_required, is_unique, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![sessions_tab_id, name, col_type, order, *required as i64, *unique as i64, now_ts],
        )?;
    }
    println!(
        "Created {} columns in Sessions tab (with relation to Projects)",
        sessions_columns.len()
    );

    // ============================================
    // Tab 2: Messages - columns with relation to Sessions
    // ============================================
    let messages_tab_id = tab_ids[2];
    let session_relation_type = format!(
        r#"{{"type": "relation", "target_sheet_tab_id": {}, "display_column": "Agent Type"}}"#,
        sessions_tab_id
    );
    let messages_columns = vec![
        ("ID", r#"{"type": "integer"}"#.to_string(), 0, true, true),
        ("Session", session_relation_type, 1, true, false),
        ("Role", r#"{"type": "text"}"#.to_string(), 2, true, false),
        ("Content", r#"{"type": "text"}"#.to_string(), 3, true, false),
        (
            "Created At",
            r#"{"type": "date_time"}"#.to_string(),
            4,
            true,
            false,
        ),
    ];

    for (name, col_type, order, required, unique) in &messages_columns {
        conn.execute(
            "INSERT INTO sheet_tab_column (sheet_tab_id, name, column_type, display_order, is_required, is_unique, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![messages_tab_id, name, col_type, order, *required as i64, *unique as i64, now_ts],
        )?;
    }
    println!(
        "Created {} columns in Messages tab (with relation to Sessions)",
        messages_columns.len()
    );

    // ============================================
    // Tab 3: Tool Calls - columns with relations to Messages
    // ============================================
    let tool_calls_tab_id = tab_ids[3];
    let message_relation_type = format!(
        r#"{{"type": "relation", "target_sheet_tab_id": {}, "display_column": "ID"}}"#,
        messages_tab_id
    );
    let tool_calls_columns = vec![
        ("ID", r#"{"type": "integer"}"#.to_string(), 0, true, true),
        ("Message", message_relation_type, 1, true, false),
        ("Call ID", r#"{"type": "text"}"#.to_string(), 2, true, false),
        (
            "Tool Name",
            r#"{"type": "text"}"#.to_string(),
            3,
            true,
            false,
        ),
        (
            "Arguments",
            r#"{"type": "json"}"#.to_string(),
            4,
            false,
            false,
        ),
        ("Result", r#"{"type": "text"}"#.to_string(), 5, false, false),
        (
            "Created At",
            r#"{"type": "date_time"}"#.to_string(),
            6,
            true,
            false,
        ),
    ];

    for (name, col_type, order, required, unique) in &tool_calls_columns {
        conn.execute(
            "INSERT INTO sheet_tab_column (sheet_tab_id, name, column_type, display_order, is_required, is_unique, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![tool_calls_tab_id, name, col_type, order, *required as i64, *unique as i64, now_ts],
        )?;
    }
    println!(
        "Created {} columns in Tool Calls tab (with relation to Messages)",
        tool_calls_columns.len()
    );

    println!("\n✅ 'Nocodo Internal' schema created successfully!");
    println!("Tabs: Projects → Sessions → Messages → Tool Calls");
    println!("Relations: Sessions→Projects, Messages→Sessions, Tool Calls→Messages");

    Ok(())
}
