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

/// Check if agent tables exist in database
pub fn has_agent_schema(conn: &rusqlite::Connection) -> anyhow::Result<bool> {
    let mut stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='agent_sessions'")?;
    Ok(stmt.exists([])?)
}
