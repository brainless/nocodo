use super::*;
use crate::database::Database;
use manager_tools::ToolExecutor;
use nocodo_llm_sdk::claude::ClaudeClient;
use rusqlite::Connection;
use std::path::PathBuf;
use tempfile::NamedTempFile;

/// Test fixture: Creates a temporary SQLite database with user data
///
/// Returns a tuple of (NamedTempFile, absolute_path_string)
/// The NamedTempFile must be kept alive for the duration of the test
fn setup_test_db() -> anyhow::Result<(NamedTempFile, String)> {
    let temp_file = NamedTempFile::new()?;
    let db_path = temp_file
        .path()
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Failed to convert path to string"))?
        .to_string();

    let conn = Connection::open(&db_path)?;

    // Create users table with created_at timestamp
    conn.execute(
        "CREATE TABLE users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            email TEXT NOT NULL,
            created_at TEXT NOT NULL
        )",
        [],
    )?;

    // Insert sample users with different registration dates
    let users = vec![
        ("Alice Johnson", "alice@example.com", "2024-01-15 10:30:00"),
        ("Bob Smith", "bob@example.com", "2024-02-20 14:15:00"),
        ("Charlie Brown", "charlie@example.com", "2024-03-10 09:00:00"),
        ("Diana Prince", "diana@example.com", "2024-04-05 16:45:00"),
        ("Eve Anderson", "eve@example.com", "2024-05-12 11:20:00"),
    ];

    for (name, email, created_at) in users {
        conn.execute(
            "INSERT INTO users (name, email, created_at) VALUES (?, ?, ?)",
            [name, email, created_at],
        )?;
    }

    // Verify data was inserted
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))?;
    assert_eq!(count, 5, "Expected 5 users to be inserted");

    Ok((temp_file, db_path))
}

#[tokio::test]
#[ignore] // Run with: cargo test --package nocodo-agents -- --ignored
async fn test_count_users_integration() -> anyhow::Result<()> {
    // Check for API key
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .expect("ANTHROPIC_API_KEY must be set for integration tests");

    // Setup test database
    let (_temp_file, db_path) = setup_test_db()?;
    println!("Created test database at: {}", db_path);

    // Create LLM client
    let llm_client = Arc::new(ClaudeClient::new(&api_key)?);

    // Create agent session database (for tracking conversations)
    let session_db = Arc::new(Database::new(&PathBuf::from(":memory:"))?);

    // Create tool executor
    let tool_executor = Arc::new(ToolExecutor::new(PathBuf::from(".")));

    // Create agent
    let agent = SqliteAnalysisAgent::new_for_testing(
        llm_client,
        session_db,
        tool_executor,
        db_path.clone(),
    );

    // Execute query
    println!("Asking: How many users do we have?");
    let result = agent.execute("How many users do we have?").await?;
    println!("Agent response: {}", result);

    // Verify the response mentions 5 users
    let result_lower = result.to_lowercase();
    assert!(
        result_lower.contains("5") || result_lower.contains("five"),
        "Response should mention 5 users. Got: {}",
        result
    );

    println!("✓ Test passed!");
    Ok(())
}

#[tokio::test]
#[ignore] // Run with: cargo test --package nocodo-agents -- --ignored
async fn test_latest_user_registration_integration() -> anyhow::Result<()> {
    // Initialize tracing for debugging
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .try_init();

    // Check for API key
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .expect("ANTHROPIC_API_KEY must be set for integration tests");

    // Setup test database
    let (_temp_file, db_path) = setup_test_db()?;
    println!("Created test database at: {}", db_path);

    // Create LLM client
    let llm_client = Arc::new(ClaudeClient::new(&api_key)?);

    // Create agent session database (for tracking conversations)
    let session_db = Arc::new(Database::new(&PathBuf::from(":memory:"))?);

    // Create tool executor
    let tool_executor = Arc::new(ToolExecutor::new(PathBuf::from(".")));

    // Create agent
    let agent = SqliteAnalysisAgent::new_for_testing(
        llm_client,
        session_db,
        tool_executor,
        db_path.clone(),
    );

    // Execute query
    println!("Asking: When was the latest user registration?");
    let result = agent
        .execute("When was the latest user registration?")
        .await?;
    println!("Agent response: {}", result);

    // Verify the response mentions the correct date (May 12, 2024 or 2024-05-12)
    let result_lower = result.to_lowercase();
    let has_correct_date = result_lower.contains("2024-05-12")
        || result_lower.contains("may 12")
        || result_lower.contains("may 12th")
        || (result_lower.contains("may") && result_lower.contains("12"));

    assert!(
        has_correct_date,
        "Response should mention May 12, 2024 (the latest registration date). Got: {}",
        result
    );

    println!("✓ Test passed!");
    Ok(())
}

#[test]
fn test_parse_tables_from_reflection() {
    let output = r#"Schema Reflection (tables):
Query executed successfully. Returned 2 rows.
Execution time: 1ms

name              | sql
-----------------+-------------------------------------------------------------
users             | CREATE TABLE users (
                  |     id INTEGER PRIMARY KEY AUTOINCREMENT,
                  |     name TEXT NOT NULL,
                  |     email TEXT NOT NULL,
                  |     created_at TEXT NOT NULL
                  | )
posts             | CREATE TABLE posts (
                  |     id INTEGER PRIMARY KEY AUTOINCREMENT,
                  |     user_id INTEGER NOT NULL,
                  |     title TEXT NOT NULL
                  | )"#;

    let tables = parse_tables_from_reflection(output).unwrap();
    assert_eq!(tables.len(), 2);
    assert_eq!(tables[0].name, "users");
    assert!(tables[0].create_sql.is_some());
    assert_eq!(tables[1].name, "posts");
    assert!(tables[1].create_sql.is_some());
}

#[test]
fn test_extract_table_name_from_create_sql() {
    assert_eq!(
        extract_table_name_from_create_sql("CREATE TABLE users (id INTEGER)"),
        Some("users".to_string())
    );
    assert_eq!(
        extract_table_name_from_create_sql("CREATE TABLE IF NOT EXISTS posts (id INTEGER)"),
        Some("posts".to_string())
    );
    assert_eq!(
        extract_table_name_from_create_sql("CREATE TABLE [user_data] (id INTEGER)"),
        Some("user_data".to_string())
    );
    assert!(extract_table_name_from_create_sql("SELECT * FROM users").is_none());
}

#[test]
fn test_extract_columns_from_ddl() {
    let ddl = "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL, email TEXT)";
    let columns = extract_columns_from_ddl(ddl);
    assert!(columns.contains("id"));
    assert!(columns.contains("name"));
    assert!(columns.contains("email"));
}

#[test]
fn test_extract_columns_from_ddl_long() {
    let ddl = "CREATE TABLE users (id INTEGER, name TEXT, email TEXT, created_at TEXT, updated_at TEXT, status TEXT, age INTEGER, score INTEGER, rank INTEGER, level INTEGER, extra1 TEXT, extra2 TEXT)";
    let columns = extract_columns_from_ddl(ddl);
    assert!(columns.contains("..."));
    assert!(columns.contains("more"));
}

#[test]
fn test_schema_info_empty() {
    let schema = SchemaInfo { tables: vec![] };
    assert_eq!(schema.tables.len(), 0);
}

#[test]
fn test_generate_system_prompt_with_schema() {
    let schema = SchemaInfo {
        tables: vec![
            TableInfo {
                name: "users".to_string(),
                create_sql: Some("CREATE TABLE users (id INTEGER, name TEXT)".to_string()),
            },
            TableInfo {
                name: "posts".to_string(),
                create_sql: Some("CREATE TABLE posts (id INTEGER, title TEXT)".to_string()),
            },
        ],
    };

    let prompt = generate_system_prompt_with_schema("/path/to/db.db", &schema);
    assert!(prompt.contains("users"));
    assert!(prompt.contains("posts"));
    assert!(prompt.contains("DATABASE SCHEMA"));
    assert!(prompt.contains("QUERY MODE"));
    assert!(prompt.contains("REFLECT MODE"));
}

#[test]
fn test_generate_system_prompt_empty_schema() {
    let schema = SchemaInfo { tables: vec![] };
    let prompt = generate_system_prompt_with_schema("/path/to/db.db", &schema);
    assert!(prompt.contains("No tables found"));
}
