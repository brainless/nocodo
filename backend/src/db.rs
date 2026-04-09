#[cfg(not(any(feature = "db-sqlite", feature = "db-postgres")))]
compile_error!("Enable one DB feature: db-sqlite or db-postgres");

#[cfg(all(feature = "db-sqlite", feature = "db-postgres"))]
compile_error!("Enable only one DB feature: db-sqlite or db-postgres");

use std::io;

#[cfg(feature = "db-sqlite")]
mod sqlite {
    use super::*;
    use refinery::embed_migrations;
    use rusqlite::Connection;

    embed_migrations!("migrations/sqlite");

    pub fn run_startup_migrations(database_url: &str) -> io::Result<()> {
        let mut conn = Connection::open(database_url).map_err(io::Error::other)?;
        migrations::runner()
            .run(&mut conn)
            .map_err(io::Error::other)?;
        Ok(())
    }

    pub fn ensure_default_project(database_url: &str) -> io::Result<()> {
        let mut conn = Connection::open(database_url).map_err(io::Error::other)?;

        // Check if any project exists
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM project", [], |row| row.get(0))
            .map_err(io::Error::other)?;

        if count == 0 {
            // Create a default project with id=1
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;

            conn.execute(
                "INSERT INTO project (id, name, created_at) VALUES (1, 'Default Project', ?1)",
                [now],
            )
            .map_err(io::Error::other)?;

            println!("Created default project with id=1");
        }

        Ok(())
    }
}

#[cfg(feature = "db-postgres")]
mod postgres_db {
    use super::*;
    use postgres::{Client, NoTls};
    use refinery::embed_migrations;

    embed_migrations!("migrations/postgres");

    pub fn run_startup_migrations(database_url: &str) -> io::Result<()> {
        let mut client = Client::connect(database_url, NoTls).map_err(io::Error::other)?;
        migrations::runner()
            .run(&mut client)
            .map_err(io::Error::other)?;
        Ok(())
    }
}

#[cfg(feature = "db-sqlite")]
pub use sqlite::{ensure_default_project, run_startup_migrations};

#[cfg(feature = "db-postgres")]
pub use postgres_db::run_startup_migrations;

#[cfg(feature = "db-postgres")]
pub fn ensure_default_project(_database_url: &str) -> io::Result<()> {
    // TODO: Implement for postgres when needed
    Ok(())
}
