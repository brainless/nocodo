use sqlx::{Connection, Row, SqliteConnection, SqlitePool};
use std::path::Path;

#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        // For SQLite, we need to ensure the directory exists
        if let Some(parent) = Path::new(database_url).parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }
        
        let pool = SqlitePool::connect(database_url).await?;
        Ok(Database { pool })
    }

    pub async fn init(&self) -> Result<(), sqlx::Error> {
        // Create tables for users
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY DEFAULT (lower(hex(randomblob(16)))),
                email TEXT UNIQUE NOT NULL,
                created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
                updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create index on email
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_users_email ON users(email)
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_connection(&self) -> Result<SqliteConnection, sqlx::Error> {
        let mut conn = self.pool.acquire().await?;
        // Enable foreign key constraints for SQLite
        conn.execute("PRAGMA foreign_keys = ON").await?;
        Ok(conn)
    }
}