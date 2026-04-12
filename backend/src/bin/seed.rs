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
    let mut conn = Connection::open(&database_url)?;

    let now_ts = now();

    // Ensure we have a default project
    let project_exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM project WHERE id = 1)",
        [],
        |row| row.get(0),
    )?;

    if !project_exists {
        conn.execute(
            "INSERT INTO project (id, name, created_at) VALUES (1, 'Default Project', ?1)",
            params![now_ts],
        )?;
        println!("Created default project");
    }

    // Create a "Sales CRM" sheet
    conn.execute(
        "INSERT INTO sheet (project_id, name, created_at, updated_at) VALUES (1, 'Sales CRM', ?1, ?1)",
        params![now_ts],
    )?;
    let sheet_id = conn.last_insert_rowid();
    println!("Created sheet: Sales CRM (id={})", sheet_id);

    // Create tabs: Leads, Pipeline, Forecast
    let tabs = vec![
        ("Leads", 0),
        ("Pipeline", 1),
        ("Forecast", 2),
        ("Invoices", 3),
        ("Archive", 4),
    ];

    for (name, order) in &tabs {
        conn.execute(
            "INSERT INTO sheet_tab (sheet_id, name, display_order, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)",
            params![sheet_id, name, order, now_ts],
        )?;
    }
    println!("Created {} tabs", tabs.len());

    // Get the Leads tab id and add columns
    let leads_tab_id: i64 = conn.query_row(
        "SELECT id FROM sheet_tab WHERE sheet_id = ?1 AND name = 'Leads'",
        params![sheet_id],
        |row| row.get(0),
    )?;

    let columns = vec![
        ("Name", r#"{"type": "text"}"#, 0, true, false),
        ("Company", r#"{"type": "text"}"#, 1, false, false),
        ("Email", r#"{"type": "text"}"#, 2, false, false),
        ("Status", r#"{"type": "text"}"#, 3, false, false),
        ("Value", r#"{"type": "number"}"#, 4, false, false),
    ];

    for (name, col_type, order, required, unique) in &columns {
        conn.execute(
            "INSERT INTO sheet_tab_column (sheet_tab_id, name, column_type, display_order, is_required, is_unique, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![leads_tab_id, name, col_type, order, *required as i64, *unique as i64, now_ts],
        )?;
    }
    println!("Created {} columns in Leads tab", columns.len());

    // Insert some sample leads
    let leads = vec![
        r#"{"1": "Alice Johnson", "2": "Acme Inc", "3": "alice@acme.com", "4": "Qualified", "5": "50000"}"#,
        r#"{"1": "Bob Smith", "2": "Globex Corp", "3": "bob@globex.com", "4": "New", "5": "25000"}"#,
        r#"{"1": "Carol White", "2": "Initech", "3": "carol@initech.com", "4": "Negotiating", "5": "100000"}"#,
        r#"{"1": "David Brown", "2": "Umbrella Corp", "3": "david@umbrella.com", "4": "Qualified", "5": "75000"}"#,
    ];

    for lead_data in &leads {
        conn.execute(
            "INSERT INTO sheet_tab_row (sheet_tab_id, data, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
            params![leads_tab_id, lead_data, now_ts],
        )?;
    }
    println!("Created {} sample leads", leads.len());

    // Create Pipeline tab with relation to Leads
    let pipeline_tab_id: i64 = conn.query_row(
        "SELECT id FROM sheet_tab WHERE sheet_id = ?1 AND name = 'Pipeline'",
        params![sheet_id],
        |row| row.get(0),
    )?;

    let pipeline_columns = vec![
        ("Deal Name", r#"{"type": "text"}"#, 0, true, false),
        (
            "Lead",
            r#"{"type": "relation", "target_sheet_tab_id": 1, "display_column": "Name"}"#,
            1,
            true,
            false,
        ),
        ("Stage", r#"{"type": "text"}"#, 2, true, false),
        ("Amount", r#"{"type": "currency"}"#, 3, false, false),
    ];

    for (name, col_type, order, required, unique) in pipeline_columns {
        conn.execute(
            "INSERT INTO sheet_tab_column (sheet_tab_id, name, column_type, display_order, is_required, is_unique, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![pipeline_tab_id, name, col_type, order, required as i64, unique as i64, now_ts],
        )?;
    }
    println!("Created Pipeline tab with relation to Leads");

    println!("\nSeed data inserted successfully!");
    println!("You can now start the backend and view the data in the admin-gui.");

    Ok(())
}
