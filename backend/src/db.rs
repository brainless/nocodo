#[cfg(not(any(feature = "db-sqlite", feature = "db-postgres")))]
compile_error!("Enable one DB feature: db-sqlite or db-postgres");

#[cfg(all(feature = "db-sqlite", feature = "db-postgres"))]
compile_error!("Enable only one DB feature: db-sqlite or db-postgres");

use std::{env, io};

#[cfg(feature = "db-sqlite")]
mod sqlite {
    use super::*;
    use refinery::embed_migrations;
    use rusqlite::Connection;

    embed_migrations!("../migrations/sqlite");

    pub fn run_startup_migrations() -> io::Result<()> {
        let path = env::var("DATABASE_URL").unwrap_or_else(|_| "app.db".to_string());
        let mut conn = Connection::open(path).map_err(io::Error::other)?;
        migrations::runner().run(&mut conn).map_err(io::Error::other)?;
        Ok(())
    }
}

#[cfg(feature = "db-postgres")]
mod postgres_db {
    use super::*;
    use postgres::{Client, NoTls};
    use refinery::embed_migrations;

    embed_migrations!("../migrations/postgres");

    pub fn run_startup_migrations() -> io::Result<()> {
        let url = env::var("DATABASE_URL").map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "DATABASE_URL is required when db-postgres feature is enabled",
            )
        })?;
        let mut client = Client::connect(&url, NoTls).map_err(io::Error::other)?;
        migrations::runner().run(&mut client).map_err(io::Error::other)?;
        Ok(())
    }
}

#[cfg(feature = "db-sqlite")]
pub use sqlite::run_startup_migrations;

#[cfg(feature = "db-postgres")]
pub use postgres_db::run_startup_migrations;
