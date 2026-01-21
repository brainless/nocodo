use super::*;
use crate::database::Database;
use nocodo_llm_sdk::claude::ClaudeClient;
use nocodo_tools::ToolExecutor;
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
        (
            "Charlie Brown",
            "charlie@example.com",
            "2024-03-10 09:00:00",
        ),
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
    let agent = SqliteReaderAgent::new_for_testing(
        llm_client,
        session_db.clone(),
        tool_executor,
        db_path.clone(),
        vec!["users".to_string()],
    );

    // Create session
    let session_id = session_db.create_session(
        "sqlite-analysis",
        "claude",
        "claude-3-5-sonnet-20241022",
        Some("test system prompt"),
        "How many users do we have?",
        None,
    )?;

    // Execute query
    println!("Asking: How many users do we have?");
    let result = agent
        .execute("How many users do we have?", session_id)
        .await?;
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
    let agent = SqliteReaderAgent::new_for_testing(
        llm_client,
        session_db.clone(),
        tool_executor,
        db_path.clone(),
        vec!["users".to_string()],
    );

    // Create session
    let session_id = session_db.create_session(
        "sqlite-analysis",
        "claude",
        "claude-3-5-sonnet-20241022",
        Some("test system prompt"),
        "When was the latest user registration?",
        None,
    )?;

    // Execute query
    println!("Asking: When was the latest user registration?");
    let result = agent
        .execute("When was the latest user registration?", session_id)
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
fn test_generate_system_prompt() {
    let table_names = vec!["users".to_string(), "posts".to_string()];
    let prompt = generate_system_prompt("my_database", &table_names);
    assert!(prompt.contains("You are analyzing the database named: my_database"));
    assert!(prompt.contains("Tables in the database: users, posts"));
    assert!(prompt.contains("PRAGMA table_info"));
    assert!(!prompt.contains("PRAGMA table_list"));
    assert!(!prompt.contains("QUERY MODE"));
    assert!(!prompt.contains("REFLECT MODE"));
}
