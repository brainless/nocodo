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

    // Check if "Nocodo Internal" schema already exists
    let internal_exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM app_schema WHERE name = 'Nocodo Internal')",
        [],
        |row| row.get(0),
    )?;

    if internal_exists {
        println!("'Nocodo Internal' schema already exists, skipping.");
        return Ok(());
    }

    // Create "Nocodo Internal" schema (project_id=1 is the default project)
    conn.execute(
        "INSERT INTO app_schema (project_id, name, created_at) VALUES (1, 'Nocodo Internal', ?1)",
        params![now_ts],
    )?;
    let schema_id = conn.last_insert_rowid();
    println!("Created schema: Nocodo Internal (id={})", schema_id);

    // Create tables
    let tables = [
        ("Projects",   "project"),
        ("Sessions",   "agent_chat_session"),
        ("Messages",   "agent_chat_message"),
        ("Tool Calls", "agent_tool_call"),
    ];

    let mut table_ids: Vec<i64> = Vec::new();
    for (name, _) in &tables {
        conn.execute(
            "INSERT INTO schema_table (schema_id, name, created_at) VALUES (?1, ?2, ?3)",
            params![schema_id, name, now_ts],
        )?;
        table_ids.push(conn.last_insert_rowid());
    }
    let [projects_id, sessions_id, messages_id, tool_calls_id] =
        [table_ids[0], table_ids[1], table_ids[2], table_ids[3]];
    println!(
        "Created tables: Projects({}) Sessions({}) Messages({}) Tool Calls({})",
        projects_id, sessions_id, messages_id, tool_calls_id
    );

    // Helper to insert a column
    let insert_col = |table_id: i64,
                      name: &str,
                      data_type: &str,
                      nullable: bool,
                      primary_key: bool,
                      display_order: i32|
     -> Result<i64, Box<dyn std::error::Error>> {
        conn.execute(
            "INSERT INTO schema_column
             (table_id, name, data_type, nullable, primary_key, display_order, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                table_id,
                name,
                data_type,
                nullable as i64,
                primary_key as i64,
                display_order,
                now_ts
            ],
        )?;
        Ok(conn.last_insert_rowid())
    };

    // Helper to insert a foreign key
    let insert_fk =
        |column_id: i64, ref_table: &str, ref_column: &str| -> Result<(), Box<dyn std::error::Error>> {
            conn.execute(
                "INSERT INTO schema_fk (column_id, ref_table, ref_column) VALUES (?1, ?2, ?3)",
                params![column_id, ref_table, ref_column],
            )?;
            Ok(())
        };

    // Helper to insert column_display
    let insert_display =
        |column_id: i64, display_column: Option<&str>| -> Result<(), Box<dyn std::error::Error>> {
            conn.execute(
                "INSERT INTO column_display (column_id, width, display_column) VALUES (?1, 120, ?2)",
                params![column_id, display_column],
            )?;
            Ok(())
        };

    // ── Projects ──────────────────────────────────────────────────────────────
    insert_col(projects_id, "id",         "integer",   false, true,  0)?;
    insert_col(projects_id, "name",       "text",      false, false, 1)?;
    insert_col(projects_id, "path",       "text",      false, false, 2)?;
    insert_col(projects_id, "created_at", "date_time", false, false, 3)?;
    println!("Created Projects columns");

    // ── Sessions ─────────────────────────────────────────────────────────────
    insert_col(sessions_id, "id",         "integer",   false, true,  0)?;
    let sessions_project_col = insert_col(sessions_id, "project_id", "integer", false, false, 1)?;
    insert_col(sessions_id, "agent_type", "text",      false, false, 2)?;
    insert_col(sessions_id, "created_at", "date_time", false, false, 3)?;
    // FK: sessions.project_id → project.id
    insert_fk(sessions_project_col, "project", "id")?;
    insert_display(sessions_project_col, Some("name"))?;
    println!("Created Sessions columns (FK → Projects)");

    // ── Messages ──────────────────────────────────────────────────────────────
    insert_col(messages_id, "id",         "integer",   false, true,  0)?;
    let messages_session_col = insert_col(messages_id, "session_id", "integer", false, false, 1)?;
    insert_col(messages_id, "role",       "text",      false, false, 2)?;
    insert_col(messages_id, "content",    "text",      false, false, 3)?;
    insert_col(messages_id, "created_at", "date_time", false, false, 4)?;
    // FK: messages.session_id → agent_chat_session.id
    insert_fk(messages_session_col, "agent_chat_session", "id")?;
    insert_display(messages_session_col, Some("agent_type"))?;
    println!("Created Messages columns (FK → Sessions)");

    // ── Tool Calls ────────────────────────────────────────────────────────────
    insert_col(tool_calls_id, "id",         "integer",   false, true,  0)?;
    let tc_message_col = insert_col(tool_calls_id, "message_id", "integer", false, false, 1)?;
    insert_col(tool_calls_id, "call_id",    "text",      false, false, 2)?;
    insert_col(tool_calls_id, "tool_name",  "text",      false, false, 3)?;
    insert_col(tool_calls_id, "arguments",  "text",      true,  false, 4)?;
    insert_col(tool_calls_id, "result",     "text",      true,  false, 5)?;
    insert_col(tool_calls_id, "created_at", "date_time", false, false, 6)?;
    // FK: tool_calls.message_id → agent_chat_message.id
    insert_fk(tc_message_col, "agent_chat_message", "id")?;
    insert_display(tc_message_col, Some("id"))?;
    println!("Created Tool Calls columns (FK → Messages)");

    println!("\n'Nocodo Internal' schema seeded successfully.");
    Ok(())
}
