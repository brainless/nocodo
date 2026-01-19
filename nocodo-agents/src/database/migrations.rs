use refinery::embed_migrations;

// Embed migrations from the migrations directory
embed_migrations!("src/database/migrations");

/// Run agent core migrations on a database connection
///
/// This function runs migrations for the core agent tables:
/// - agent_sessions: Tracks agent execution sessions
/// - agent_messages: Stores conversation messages
/// - agent_tool_calls: Tracks tool executions
///
/// This function is public so that external projects can run these migrations
/// on their own database connections. It works with any database that Refinery
/// supports (SQLite, PostgreSQL, MySQL, SQL Server).
///
/// Currently generates SQLite dialect only. Support for other databases
/// can be added by creating database-specific migration modules.
///
/// # Arguments
/// * `conn` - A mutable reference to any database connection that implements refinery::migrate::Migrate
///
/// # Example with SQLite
/// ```no_run
/// use rusqlite::Connection;
/// use nocodo_agents::database::migrations::run_agent_migrations;
///
/// let mut conn = Connection::open("mydb.db")?;
/// run_agent_migrations(&mut conn)?;
/// # Ok::<(), anyhow::Error>(())
/// ```
///
/// # Example with PostgreSQL
/// ```no_run
/// use postgres::Client;
/// use nocodo_agents::database::migrations::run_agent_migrations;
///
/// let mut client = Client::connect("postgresql://localhost/mydb", postgres::NoTls)?;
/// run_agent_migrations(&mut client)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn run_agent_migrations<C>(conn: &mut C) -> anyhow::Result<()>
where
    C: refinery::Migrate,
{
    migrations::runner().run(conn)?;
    Ok(())
}

/// Run agent migrations on a SQLite connection with legacy database support
///
/// This function handles both new and legacy databases:
/// - For legacy databases (tables exist but no refinery_schema_history), it initializes the history table
/// - For new databases, it runs migrations normally
pub fn run_agent_migrations_sqlite(conn: &mut rusqlite::Connection) -> anyhow::Result<()> {
    // Check if this is a legacy database
    if is_legacy_database(conn)? {
        initialize_schema_history_for_legacy_db(conn)?;
        return Ok(());
    }

    migrations::runner().run(conn)?;
    Ok(())
}

/// Check if this is a legacy database (has agent tables but no refinery_schema_history records)
fn is_legacy_database(conn: &rusqlite::Connection) -> anyhow::Result<bool> {
    // Check if refinery_schema_history table exists and has records
    let mut stmt = conn.prepare(
        "SELECT name FROM sqlite_master WHERE type='table' AND name='refinery_schema_history'",
    )?;
    let has_history_table = stmt.exists([])?;

    if has_history_table {
        // Check if the table has any records
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM refinery_schema_history")?;
        let count: i64 = stmt.query_row([], |row| row.get(0))?;
        if count > 0 {
            return Ok(false);
        }
    }

    // Check if agent_sessions table exists (indicating a legacy database)
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='agent_sessions'")?;
    let has_agent_tables = stmt.exists([])?;

    Ok(has_agent_tables)
}

/// Initialize the schema history table for a legacy database
fn initialize_schema_history_for_legacy_db(conn: &mut rusqlite::Connection) -> anyhow::Result<()> {
    // Create the refinery_schema_history table if it doesn't exist
    conn.execute(
        "CREATE TABLE IF NOT EXISTS refinery_schema_history (
            version INTEGER PRIMARY KEY,
            name VARCHAR(255),
            applied_on VARCHAR(255),
            checksum VARCHAR(255)
        )",
        [],
    )?;

    // Get the list of migrations
    let migration_list = vec![
        ("V1__create_agent_sessions", 1),
        ("V2__create_agent_messages", 2),
        ("V3__create_agent_tool_calls", 3),
        ("V4__create_project_requirements_qna", 4),
        ("V5__create_project_settings", 5),
    ];

    // Mark all migrations as applied
    let applied_on = chrono::Utc::now().to_rfc3339();
    for (name, version) in migration_list {
        // Check if the corresponding table exists before marking as applied
        let table_name = match version {
            1 => "agent_sessions",
            2 => "agent_messages",
            3 => "agent_tool_calls",
            4 => "project_requirements_qna",
            5 => "project_settings",
            _ => continue,
        };

        let mut stmt = conn.prepare(&format!(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='{}'",
            table_name
        ))?;

        if stmt.exists([])? {
            conn.execute(
                "INSERT INTO refinery_schema_history (version, name, applied_on, checksum) VALUES (?1, ?2, ?3, '')",
                rusqlite::params![version, name, &applied_on],
            )?;
        }
    }

    Ok(())
}

/// Check if agent tables exist in database
pub fn has_agent_schema(conn: &rusqlite::Connection) -> anyhow::Result<bool> {
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='agent_sessions'")?;
    Ok(stmt.exists([])?)
}
